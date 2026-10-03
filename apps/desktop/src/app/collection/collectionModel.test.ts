import skinLines from './skinLines.json';
import {describe, expect, it} from 'vitest';
import type {CollectionSkin, CollectionState} from '../../../../../packages/shared/src/collection';
import {filterCollection, canEditWishes, collectionSummary, initialCollectionState, initialCollectionView, collectionViewForAccount, collectionVisibleCount, nextCollectionCount, patchCollectionView} from './collectionModel';
const skin=(id:number,ownership:CollectionSkin['ownership'],name='Étoile'):CollectionSkin=>({id,champion_id:103,name,ownership,tile_url:null,splash_url:null,obtainable:null,rarity:null,series_ids:[]});
const state:CollectionState={...initialCollectionState(false),status:'ready',account:{platform:'EUW1',game_name:'Test',tag_line:'EUW'},skins:[skin(1,'owned'),skin(2,'missing'),skin(3,'temporary'),skin(4,'unknown')],wishes:[2,4]};
describe('collection : filtrage fidèle aux possessions du client',()=>{
 it('ne classe jamais les locations ou possessions inconnues parmi les skins manquants',()=>{
  expect(filterCollection(state,{query:'',filter:'missing',championId:null},'fr').map(s=>s.id)).toEqual([2]);
  expect(filterCollection(state,{query:'',filter:'owned',championId:null},'fr').map(s=>s.id)).toEqual([1]);
  expect(collectionSummary(state)).toEqual({owned:1,total:4,temporary:1,unknown:1});
 });
 it('croise souhaits, champion et recherche accentuée sans modifier le catalogue',()=>{
  expect(filterCollection(state,{query:'etoile',filter:'wishes',championId:103},'fr').map(s=>s.id)).toEqual([2,4]);
  expect(filterCollection(state,{query:'',filter:'all',championId:99},'en')).toEqual([]);
  expect(state.skins).toHaveLength(4);
 });
 it('oublie sélection, filtres et défilement quand on revient sur un autre compte',()=>{
  const view={...initialCollectionView,accountKey:'EUW1:A#EUW',selectedId:103001,scrollTop:450,query:'Ahri',championId:103};
  expect(collectionViewForAccount(view,'EUW1:A#EUW')).toBe(view);
  expect(collectionViewForAccount(view,'EUW1:B#EUW')).toEqual({...initialCollectionView,accountKey:'EUW1:B#EUW'});
  expect(collectionViewForAccount(view,null).selectedId).toBeNull();
 });
 it('borne le premier lot à 60 et agrandit sans dépasser les résultats',()=>{
  expect(initialCollectionView.visibleCount).toBe(60);
  expect(collectionVisibleCount(60,2390)).toBe(60);
  expect(collectionVisibleCount(60,23)).toBe(23);
  expect(nextCollectionCount(60,2390)).toBe(120);
  expect(nextCollectionCount(120,127)).toBe(127);
  expect(nextCollectionCount(127,127)).toBe(127);
  expect(collectionVisibleCount(Number.NaN,2390)).toBe(60);
 });
 it('restaure le lot visité, puis réduit à 60 au changement de filtre ou de compte',()=>{
  const view={...initialCollectionView,accountKey:'A',visibleCount:180,scrollTop:850};
  expect(collectionViewForAccount(view,'A')).toBe(view);
  expect(collectionViewForAccount(view,'B').visibleCount).toBe(60);
  expect(patchCollectionView(view,{selectedId:4}).visibleCount).toBe(180);
  for(const patch of [{query:'ahri'},{filter:'owned' as const},{championId:103}])expect(patchCollectionView(view,patch)).toMatchObject({visibleCount:60,scrollTop:0});
 });
 it('interdit les écritures déconnectées, périmées, sans compte, en erreur de stockage ou de chargement',()=>{
  expect(canEditWishes(state)).toBe(true);
  for(const patch of [{status:'disconnected' as const},{status:'loading' as const},{status:'unavailable' as const},{stale:true},{storage_error:true},{account:null}])expect(canEditWishes({...state,...patch})).toBe(false);
 });
});

it('croise rareté et séries multiples sans transformer une valeur inconnue',()=>{
 const source={...state,skins:[{...skin(1,'owned'),rarity:'kEpic' as const,series_ids:[10,20]},{...skin(2,'owned'),rarity:null,series_ids:[]}]};
 expect(filterCollection(source,{...initialCollectionView,rarity:'kEpic',seriesId:20},'fr').map(s=>s.id)).toEqual([1]);
 expect(filterCollection(source,{...initialCollectionView,rarity:'kLegendary'},'fr')).toEqual([]);
 for(const patch of [{rarity:'kEpic' as const},{seriesId:20}])expect(patchCollectionView({...initialCollectionView,scrollTop:500,visibleCount:180},patch)).toMatchObject({scrollTop:0,visibleCount:60});
});

it('cherche les raretés et séries FR/EN, combinées au champion, sans inférence par nom',()=>{
 const source={...state,skins:[{...skin(103027,'owned','Ahri fleur spirituelle'),rarity:'kLegendary' as const,series_ids:[30]},{...skin(103001,'missing','Ahri légendaire inventée'),rarity:null,series_ids:[]}]};
 const series=skinLines.entries.find(line=>line.names.en==='Spirit Blossom')!;
 source.skins[0]!.series_ids=[series.id];
 for(const query of ['Spirit Blossom','fleur spirituelle','legendary','légendaire','ahri legendary','AHRI spirit blossom']){
  const ids=filterCollection(source,{...initialCollectionView,query},'fr').map(s=>s.id);
  expect(ids).toContain(103027);
 }
 expect(filterCollection(source,{...initialCollectionView,query:'Ahri Spirit Blossom',filter:'missing'},'fr')).toEqual([]);
 expect(filterCollection(source,{...initialCollectionView,query:'lux legendary'},'fr')).toEqual([]);
});
