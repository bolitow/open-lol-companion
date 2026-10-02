import {describe,it,expect} from 'vitest';
import type {Profile,ProfileMatches,PlayerRequest} from '@olc/shared';
import {createPlayerStore,parsePlayerQuery,parseHomePlayer,playerKey} from './playerStore';
const a:PlayerRequest={platform:'EUW1',game_name:'First Player',tag_line:'TEST'};
const b:PlayerRequest={...a,game_name:'Second Player'};
const profile=(who=a):Profile=>({...who,puuid:'synthetic',profile_icon_id:1,summoner_level:42,ranks:[],fetched_at:1});
const page=(who=a,start=0,next:number|null=null):ProfileMatches=>({...who,start,count:10,next_start:next,fetched_at:1,omitted_matches:0,matches:[]});
const flush=()=>new Promise(resolve=>setTimeout(resolve,0));
describe('identité joueur et accueil',()=>{
 it('normalise la saisie et refuse les chemins ambigus, identités incomplètes ou trop longues',()=>{
  expect(parsePlayerQuery('  First Player # TEST  ','EUW1')).toEqual(a);
  for(const value of ['First Player','A#B#C','..#tag','a#..','a\n#tag',`${'é'.repeat(33)}#TAG`])expect(parsePlayerQuery(value,'EUW1')).toBeNull();
  expect(parsePlayerQuery('a#tag','INVALID')).toBeNull();
  expect(playerKey({...a,game_name:'FIRST PLAYER'})).toBe(playerKey(a));
 });
 it('restaure uniquement une identité valide, sans données de partie ni identifiant privé',()=>{
  expect(parseHomePlayer(JSON.stringify({...a,puuid:'not-persisted',matches:[]}))).toEqual(a);
  expect(parseHomePlayer('{bad')).toBeNull();
  expect(parseHomePlayer(JSON.stringify({...a,platform:'INVALID'}))).toBeNull();
 });
 it('consulter B ne remplace pas A à l’accueil ; le retour conserve historique et position',async()=>{
  const saved:unknown[]=[];
  const store=createPlayerStore({profile:async who=>profile(who),matches:async req=>page(req.player,req.start)},a,value=>saved.push(value));
  store.start();await flush();store.select(a);store.scroll(240);store.select(b);await flush();
  expect(store.getSnapshot().home).toEqual(a);expect(saved).toEqual([]);
  expect(store.getSnapshot().entries[playerKey(a)]!.scrollTop).toBe(240);
  store.pin();expect(store.getSnapshot().home).toEqual(b);expect(saved).toEqual([b]);
 });
 it('garde le profil quand la pagination échoue, puis reprend au même curseur, même sur une page vide',async()=>{
  let fail=true;const starts:number[]=[];
  const store=createPlayerStore({profile:async()=>profile(),matches:async req=>{starts.push(req.start);if(req.start===10&&fail)throw 'rate_limited';return page(a,req.start,req.start===0?10:null)}},null,()=>{});
  store.select(a);await flush();await store.more();
  let entry=store.getSnapshot().entries[playerKey(a)]!;expect(entry.profile).toEqual(profile());expect(entry.historyError).toBe('rate_limited');expect(entry.next).toBe(10);
  fail=false;await store.more();entry=store.getSnapshot().entries[playerKey(a)]!;expect(entry.next).toBeNull();expect(starts).toEqual([0,10,10]);
 });
 it('ignore les réponses tardives, refuse un historique incohérent et nettoie les erreurs',async()=>{
  let done!:(p:Profile)=>void;
  const store=createPlayerStore({profile:who=>who.game_name===a.game_name?new Promise(resolve=>{done=resolve}):Promise.resolve(profile(b)),matches:async()=>page(a)},null,()=>{});
  store.select(a);await flush();store.select(b);await flush();done(profile(a));await flush();
  expect(store.getSnapshot().viewed).toEqual(b);expect(store.getSnapshot().entries[playerKey(a)]).toBeUndefined();
  expect(store.getSnapshot().entries[playerKey(b)]!.historyError).toBe('invalid_response');
  const broken=createPlayerStore({profile:async()=>{throw new Error('private detail')},matches:async()=>page()},null,()=>{});
  broken.select(a);await flush();expect(broken.getSnapshot().entries[playerKey(a)]!.error).toBe('unavailable');
 });
 it('signale un stockage refusé tout en gardant le choix pendant la session',async()=>{
  const store=createPlayerStore({profile:async()=>profile(),matches:async()=>page()},null,()=>{throw new Error()});
  store.select(a);await flush();store.pin();expect(store.getSnapshot().home).toEqual(a);expect(store.getSnapshot().storageFailed).toBe(true);
 });
});
