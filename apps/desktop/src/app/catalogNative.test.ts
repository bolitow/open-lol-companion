import {expect,it,vi} from 'vitest';
const {invoke}=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>({invoke,isTauri:()=>true}));
import {catalogRuntime,refreshCatalog,receiveCatalogState} from './catalogRuntime';
import {loadCatalog,loadChampionAbilities} from './catalog';
const state=(snapshotId:string)=>({status:'ready' as const,version:'16.19.1',snapshotId,assetBase:`https://local/${snapshotId}/`,error:null});
const directory={version:'16.19.1',champions:[{id:103,key:'Ahri',names:{fr:'Ahri',en:'Ahri'},titles:{fr:'Titre',en:'Title'},categories:['Mage']}]};
const base={locale:'fr_FR',namespace:'standard',description:null,fields:{},stats:{},coverage:{},effects:[]};
it('épingle racine, fiche et images au même snapshot ; une ancienne racine ne lit jamais la nouvelle fiche',async()=>{
 invoke.mockImplementation(async(_command,args)=>{
  if(args.path==='cosmetics.json')return null;
  if(args.path==='champions.json')return {'103':{key:'Ahri',fr:'Ahri',en:'Ahri'}};
  if(args.path==='champion-directory.json')return directory;
  return {version:'16.19.1',records:[{...base,id:args.path.includes('/champions/')?'103:Q':'1001',name:'Test',kind:args.path.includes('/champions/')?'ability':'item',icon:'/game-data/catalog/icons/item/1001.png'}]};
 });
 await catalogRuntime.accept(state('a'));
 const catalog=await loadCatalog('fr');expect(catalog.snapshotId).toBe('a');expect(catalog.records[0]?.icon).toBe('https://local/a/catalog/icons/item/1001.png');
 await loadChampionAbilities('fr',103,catalog.version,catalog.snapshotId);
 expect(invoke).toHaveBeenLastCalledWith('catalog_read',{snapshotId:'a',path:'catalog/champions/103/fr_FR.json'});
 await catalogRuntime.accept(state('b'));
 await expect(loadChampionAbilities('fr',103,catalog.version,catalog.snapshotId)).rejects.toThrow('catalog-stale');
});

it('déduplique la synchronisation et ignore sa réponse dépassée par un événement',async()=>{
 const read=invoke.getMockImplementation()!;
 let release!:(value:ReturnType<typeof state>)=>void;
 const pending=new Promise<ReturnType<typeof state>>(resolve=>{release=resolve});
 invoke.mockImplementation((command,args)=>command==='catalog_state'?Promise.resolve(state('b')):command==='catalog_sync'?pending:read(command,args));
 const first=refreshCatalog();const second=refreshCatalog();expect(second).toBe(first);
 await new Promise(resolve=>setTimeout(resolve,0));
 await receiveCatalogState(state('c'));release(state('b'));await first;
 expect(catalogRuntime.getSnapshot().active?.snapshotId).toBe('c');
});
