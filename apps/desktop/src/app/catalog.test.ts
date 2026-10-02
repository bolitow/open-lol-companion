import {afterEach,expect,it,vi} from 'vitest';
import {loadCatalog,loadChampionAbilities} from './catalog';
afterEach(()=>vi.unstubAllGlobals());
it('ne convertit pas une erreur réseau ou une page HTML en catalogue vide',async()=>{
 vi.stubGlobal('fetch',vi.fn(async()=>({ok:false})));
 await expect(loadCatalog('fr')).rejects.toThrow();
 vi.stubGlobal('fetch',vi.fn(async()=>({ok:true,json:async()=>({version:'16.19.1',records:[]})})));
 await expect(loadCatalog('en')).rejects.toThrow();
});
it('charge les compétences uniquement pour le champion et le patch demandés',async()=>{
 const record={kind:'ability',id:'103:Q',name:'Orbe',locale:'fr_FR',namespace:'standard',description:null,icon:null,fields:{},stats:{},coverage:{},effects:[]};
 const fetch=vi.fn(async()=>({ok:true,json:async()=>({version:'16.19.1',records:[record]})}));vi.stubGlobal('fetch',fetch);
 expect((await loadChampionAbilities('fr',103,'16.19.1'))[0]?.id).toBe('103:Q');
 expect(fetch).toHaveBeenCalledWith('/game-data/catalog/champions/103/fr_FR.json');
 await expect(loadChampionAbilities('fr',103,'16.18.1')).rejects.toThrow();
 await expect(loadChampionAbilities('fr',222,'16.19.1')).rejects.toThrow();
});
it('lit la locale demandée et rejette une fiche incohérente',async()=>{
 const fetch=vi.fn(async()=>({ok:true,json:async()=>({version:'16.19.1',records:[{kind:'item',id:'1001'}]})}));
 vi.stubGlobal('fetch',fetch);
 await expect(loadCatalog('fr')).rejects.toThrow();
 expect(fetch).toHaveBeenCalledWith('/game-data/catalog/fr_FR.json');
});
