import {renderToStaticMarkup} from 'react-dom/server';
import {describe,expect,it,vi} from 'vitest';
import {CollectionScreen} from './CollectionScreen';
import {initialCollectionState,initialCollectionView} from './collectionModel';
import type {CollectionHookResult} from './useCollection';
const collection:CollectionHookResult={state:{...initialCollectionState(false),revision:2,status:'ready',account:{platform:'EUW1',game_name:'Test',tag_line:'EUW'},skins:[{id:103001,champion_id:103,name:'Ahri du client',ownership:'unknown',tile_url:null,splash_url:null,obtainable:null,rarity:null,series_ids:[]}],wishes:[]},pending:false,error:null,refresh:vi.fn(async()=>{}),setWish:vi.fn(async()=>{})};
const render=(patch:Partial<CollectionHookResult>={},locale:'fr'|'en'='fr')=>renderToStaticMarkup(<CollectionScreen locale={locale} collection={{...collection,...patch}} view={{...initialCollectionView,selectedId:103001}} update={()=>{}}/>);
describe('collection : présentation des données réelles',()=>{
 it('garde le nom du client, distingue une possession inconnue et utilise le sélecteur partagé',()=>{
  const html=render();expect(html).toContain('Ahri du client');expect(html).toContain('Possession inconnue');expect(html).toContain('role="combobox"');expect(html).not.toContain('<select');expect(html).toContain('Visuel indisponible');expect(html).toContain('Ajouter aux souhaits');expect(html).not.toContain('Indisponible à l’achat');
 });
 it('limite le DOM initial à 60 cartes sans tronquer le compteur ni la recherche',()=>{
  const state={...collection.state,skins:Array.from({length:2390},(_,i)=>({...collection.state.skins[0]!,id:1000+i,name:`Skin ${i}`}))};
  const html=render({state});expect(html.match(/data-skin=/g)).toHaveLength(60);expect(html).toContain('Afficher plus');expect(html).toContain(new Intl.NumberFormat('fr').format(2390));
 });
 it('ne transforme pas stillObtainable en disponibilité boutique',()=>{
  const html=render({state:{...collection.state,skins:[{...collection.state.skins[0]!,obtainable:false}]}});
  expect(html).not.toContain('Ne peut plus être obtenu');
 });
 it('priorise le splash sélectionné et réutilise sa miniature',()=>{
  const html=render({state:{...collection.state,skins:[{...collection.state.skins[0]!,tile_url:'https://raw.communitydragon.org/tile.jpg',splash_url:'https://raw.communitydragon.org/splash.jpg'}]}});
  expect(html).toContain('fetchPriority="high"');expect(html).toContain('decoding="async"');expect(html).toContain('collection-skin-placeholder');
 });
 it('affiche le compte conservé et bloque les souhaits hors connexion',()=>{
  const html=render({state:{...collection.state,status:'disconnected',stale:true}});expect(html).toContain('hors connexion');expect(html).not.toContain('Test');expect(html).toMatch(/disabled=""[^>]*aria-pressed="false"/);
 });
 it('ne remplace pas une panne de stockage par une liste de souhaits prétendument sauvegardée',()=>{
  const html=render({state:{...collection.state,storage_error:true}},'en');expect(html).toContain('Local wishes are unavailable');expect(html).toMatch(/disabled=""[^>]*aria-pressed="false"/);expect(html).not.toContain('Possession inconnue');
 });
});
it('présente les raretés et séries sourcées avec les sélecteurs partagés',()=>{
 const enriched={...collection.state,skins:[{...collection.state.skins[0]!,rarity:'kEpic' as const,series_ids:[10]}]};
 const html=render({state:enriched});expect(html).toContain('Rareté');expect(html).toContain('Épique');expect(html).toContain('Série');expect(html).toContain('Arcade');
 const en=render({state:enriched},'en');expect(en).toContain('Rarity');expect(en).toContain('Epic');
});

it('désactive correction, suggestions et capitalisation de la recherche',()=>{
 const html=render();expect(html).toContain('autoCorrect="off"');expect(html).toContain('autoComplete="off"');expect(html).toContain('autoCapitalize="off"');expect(html).toContain('spellCheck="false"');
});

it('réserve la hauteur à la galerie et réunit les compteurs sous les filtres',()=>{
 const html=render();
 expect(html).not.toContain('collection-header');
 expect(html).not.toContain('Client League');
 expect(html).not.toContain('Test');
 expect(html).toContain('Rechercher un skin, un champion, une rareté, une série…');
 expect(html).toContain('collection-counts');
 expect(html).toContain('Actualiser la collection');
 expect(html).not.toContain('Souhait enregistré sur cet appareil');
});
it('distingue le résultat filtré du total de la collection',()=>{
 const html=renderToStaticMarkup(<CollectionScreen locale="en" collection={collection} view={{...initialCollectionView,query:'absent'}} update={()=>{}}/>);
 expect(html).toContain('0 results');
 expect(html).toContain('0 owned / 1 skins');
});
