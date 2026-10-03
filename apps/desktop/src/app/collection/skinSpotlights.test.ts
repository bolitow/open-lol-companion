import {afterEach,describe,expect,it,vi} from 'vitest';
const bundled=import.meta.glob('../../../public/game-data/skin-spotlights.json',{query:'?raw',import:'default',eager:true}) as Record<string,string>;
import {parseSkinSpotlights,findSkinSpotlight,loadSkinSpotlight} from './skinSpotlights';
const entry={skinId:103007,championId:103,name:'Arcade Ahri',videoId:'IPU9_WRcsj4',title:'Arcade Ahri Skin Spotlight - League of Legends',channelUrl:'https://www.youtube.com/@SkinSpotlights',publishedAt:'2023-02-05',checkedAt:'2026-10-03',source:'https://www.youtube.com/watch?v=IPU9_WRcsj4'};
const catalog=(entries:unknown[]=[entry])=>({schemaVersion:1,entries});
afterEach(()=>{vi.unstubAllGlobals();vi.restoreAllMocks()});
describe('index vidéo par identifiant Riot',()=>{
 it('retrouve le skin exact et refuse un autre champion ou un skin inconnu',()=>{
  const index=parseSkinSpotlights(catalog());expect(findSkinSpotlight(index,103007,103)).toEqual(entry);expect(findSkinSpotlight(index,103007,99)).toBeNull();expect(findSkinSpotlight(index,103008,103)).toBeNull();
 });
 it('refuse les doublons même identiques',()=>expect(()=>parseSkinSpotlights(catalog([entry,entry]))).toThrow());
 it.each([{skinId:103007.5},{championId:99},{videoId:'bad/id'},{source:'https://evil.example/'},{channelUrl:'https://www.youtube.com/@Other'},{publishedAt:'2023-02-30'},{checkedAt:'2020-01-01'}])('rejette les associations invalides %o',patch=>expect(()=>parseSkinSpotlights(catalog([{...entry,...patch}]))).toThrow());
 it('valide le catalogue livré et ses clés uniques',()=>{
  const data=JSON.parse(Object.values(bundled)[0]!);
  const index=parseSkinSpotlights(data);expect(index.size).toBeGreaterThanOrEqual(15);expect(index.has(103007)).toBe(true);
 });
 it('réessaie après erreur puis partage le catalogue entre consultations',async()=>{
  const fetcher=vi.fn().mockRejectedValueOnce(new Error('offline')).mockResolvedValue({ok:true,json:async()=>catalog()});vi.stubGlobal('fetch',fetcher);
  await expect(loadSkinSpotlight(103007,103)).rejects.toThrow('offline');
  expect(await loadSkinSpotlight(103007,103)).toEqual(entry);expect(await loadSkinSpotlight(103008,103)).toBeNull();expect(fetcher).toHaveBeenCalledTimes(2);
 });
});

describe('passages vidéo vérifiés',()=>{
 const segment={kind:'q',start:40,end:55};
 it('accepte les passages et conserve une vidéo sans repères',()=>{
  expect(parseSkinSpotlights(catalog([{...entry,segments:[segment]}])).get(103007)?.segments).toEqual([segment]);
 });
 it.each([
  [{kind:'q',start:-1,end:55}], [{kind:'q',start:40.5,end:55}],
  [{kind:'q',start:40,end:40}], [{kind:'q',start:40,end:7201}],
  [{kind:'fake',start:40,end:55}], [segment,segment],
  [segment,{kind:'w',start:54,end:70}], 'invalid'
 ])('rejette les repères invalides ou superposés %o',segments=>{
  expect(()=>parseSkinSpotlights(catalog([{...entry,segments}]))).toThrow();
 });
});

it('refuse une vidéo attribuée deux fois et les passages au-delà de sa durée',()=>{
 expect(()=>parseSkinSpotlights(catalog([entry,{...entry,skinId:103008}]))).toThrow();
});
it('refuse un passage au-delà de la durée connue',()=>{
 expect(()=>parseSkinSpotlights(catalog([{...entry,durationSeconds:50,segments:[{kind:'q',start:40,end:55}]}]))).toThrow();
});

it('expose les nouveaux passages du lot sans perdre la curation existante',()=>{
 const data=JSON.parse(Object.values(bundled)[0]!);
 const index=parseSkinSpotlights(data);
 expect(index.get(103002)?.segments).toContainEqual({kind:'q',start:90,end:99});
 expect(index.get(103007)?.segments).toContainEqual({kind:'q',start:88,end:97});
 expect(index.get(103027)?.segments).toContainEqual({kind:'movement',start:192,end:201});
 expect(index.get(81043)?.segments?.some(segment=>segment.kind==='passive')).toBe(false);
});

it('ajoute les références du lot multi-champions sans inventer les chapitres ambigus',()=>{
 const index=parseSkinSpotlights(JSON.parse(Object.values(bundled)[0]!));
 expect(findSkinSpotlight(index,157088,157)?.videoId).toBe('ZbBCiu0CxlY');
 expect(findSkinSpotlight(index,887048,887)?.videoId).toBe('h0tfygJVgrA');
 expect(findSkinSpotlight(index,875066,875)?.segments).toEqual([]);
 expect(findSkinSpotlight(index,157088,887)).toBeNull();
});
