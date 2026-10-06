import {it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {PlayerHistory,PlayerIdentity} from './PlayersScreen';
import {createPlayerStore,playerKey} from './playerStore';
import type {PlayerProfile,PlayerHistory as History} from '@olc/shared';
const identity={platform:'EUW1',game_name:'Test Player',tag_line:'TEST'};
const profile:PlayerProfile={...identity,source:'lcu',profile_icon_id:1,summoner_level:20,ranks:[],fetched_at:100};
const page:History={...identity,source:'lcu',fetched_at:100,start:0,count:10,next_start:null,omitted_matches:0,matches:[{match_id:'EUW1_123',queue_id:0,patch:null,game_start_ms:1_000_000,duration_s:1200,champion_id:432,win:true,kills:1,deaths:2,assists:3,items:[],role:null}]};
it('annonce la provenance locale et un historique incomplet en français et anglais',async()=>{
 const store=createPlayerStore({profile:async()=>profile,matches:async()=>page},null,()=>{});
 store.syncAccount(true,identity);await new Promise(resolve=>setTimeout(resolve,0));
 const entry=store.getSnapshot().entries[playerKey(identity)]!;
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<><PlayerIdentity profile={profile} identity={identity} locale={locale}/><PlayerHistory entry={entry} locale={locale} store={store} onChampion={()=>{}}/></>);
  expect(html).toContain(locale==='fr'?'Source : client LoL':'Source: League client');
  expect(html).toContain(locale==='fr'?'il peut être incomplet':'it may be incomplete');
  expect(html).toContain(locale==='fr'?'Personnalisée':'Custom game');
  expect(html).not.toContain(locale==='fr'?'API publique':'public API');
 }
});

it('identifie les parties publiques conservées pendant un échec de recharge locale',async()=>{
 const store=createPlayerStore({profile:async()=>profile,matches:async()=>page},null,()=>{});
 store.syncAccount(true,identity);await new Promise(resolve=>setTimeout(resolve,0));
 const entry={...store.getSnapshot().entries[playerKey(identity)]!,historySource:'api' as const,historyError:'unavailable' as const};
 const html=renderToStaticMarkup(<PlayerHistory entry={entry} locale="fr" store={store} onChampion={()=>{}}/>);
 expect(html).toContain('Source : API publique');expect(html).not.toContain('il peut être incomplet');
});

it('conserve le code Riot occupé et son message distinct dans les profils FR/EN',async()=>{
 const store=createPlayerStore({profile:async()=>profile,matches:async()=>{throw 'riot_busy'}},null,()=>{});
 store.syncAccount(true,identity);await new Promise(resolve=>setTimeout(resolve,0));
 const entry=store.getSnapshot().entries[playerKey(identity)]!;
 expect(entry.historyError).toBe('riot_busy');
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<PlayerHistory entry={entry} locale={locale} store={store} onChampion={()=>{}}/>);
  expect(html).toContain(locale==='fr'?'Riot est occupé':'Riot is busy');
  expect(html).not.toContain(locale==='fr'?'ne répond pas':'not responding');
 }
});

it('termine la pagination locale sans effacer les parties ni les omissions et sans réessayer',async()=>{
 const starts:number[]=[];
 const store=createPlayerStore({profile:async()=>profile,matches:async request=>{
  starts.push(request.start);
  return request.start===0?{...page,next_start:10,omitted_matches:1}:{...page,start:10,matches:[]};
 }},null,()=>{});
 store.syncAccount(true,identity);await new Promise(resolve=>setTimeout(resolve,0));
 await store.more(identity);await store.more(identity);
 const entry=store.getSnapshot().entries[playerKey(identity)]!;
 expect(starts).toEqual([0,10]);expect(entry.matches).toEqual(page.matches);
 expect(entry.omitted).toBe(1);expect(entry.next).toBeNull();expect(entry.historyError).toBeNull();
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<PlayerHistory entry={entry} locale={locale} store={store} onChampion={()=>{}}/>);
  expect(html).toContain(locale==='fr'?'Fin des parties fournies par le client':'End of matches provided by the client');
  expect(html).toContain(locale==='fr'?'Certaines parties du client sont incomplètes':'Some client matches are incomplete');
  expect(html).not.toContain(locale==='fr'?'Afficher plus de parties':'Load more matches');
  expect(html).not.toContain(locale==='fr'?'Réessayer':'Try again');
 }
});
