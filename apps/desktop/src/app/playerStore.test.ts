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

describe('compte League actif',()=>{
 it('suit A puis B sans changer le joueur consulté, persiste seulement l’identité et conserve B à la fermeture',async()=>{
  const saved:unknown[]=[],calls:PlayerRequest[]=[];
  const store=createPlayerStore({profile:async who=>{calls.push(who);return profile(who)},matches:async req=>page(req.player)},null,value=>saved.push(value));
  const c={...a,game_name:'Viewed'};store.select(c);await flush();
  store.syncAccount?.(true,a);await flush();
  expect(store.getSnapshot().home).toEqual(a);
  store.syncAccount(true,a);await flush();expect(calls.filter(who=>playerKey(who)===playerKey(a))).toHaveLength(1);
  store.syncAccount(true,b);await flush();store.syncAccount(false,null);
  expect(store.getSnapshot().home).toEqual(b);expect(store.getSnapshot().viewed).toEqual(c);expect(saved).toEqual([a,b]);
  expect(store.getSnapshot().entries[playerKey(b)]?.profile).toEqual(profile(b));
  store.syncAccount(true,b);await flush();expect(calls.filter(who=>playerKey(who)===playerKey(b))).toHaveLength(2);
 });
 it('ne laisse pas une réponse tardive de A remplacer B et garde l’identité sans service',async()=>{
  let resolveA!:(p:Profile)=>void;
  const store=createPlayerStore({profile:who=>playerKey(who)===playerKey(a)?new Promise(resolve=>{resolveA=resolve}):Promise.reject('not_configured'),matches:async req=>page(req.player)},null,()=>{});
  store.syncAccount?.(true,a);expect(store.getSnapshot().home).toEqual(a);
  store.syncAccount(true,b);await flush();resolveA(profile(a));await flush();
  expect(store.getSnapshot().home).toEqual(b);expect(store.getSnapshot().entries[playerKey(a)]).toBeUndefined();
  expect(store.getSnapshot().entries[playerKey(b)]?.error).toBe('not_configured');
  store.syncAccount(true,null);expect(store.getSnapshot().home).toEqual(b);expect(store.getSnapshot().active).toBeNull();
 });
 it('empêche un favori manuel de remplacer le compte actif mais autorise le choix hors connexion',async()=>{
  const store=createPlayerStore({profile:async who=>profile(who),matches:async req=>page(req.player)},null,()=>{});
  store.syncAccount?.(true,a);expect(store.getSnapshot().home).toEqual(a);store.select(b);await flush();store.pin();store.forget();
  expect(store.getSnapshot().home).toEqual(a);
  store.syncAccount(false,null);store.pin();expect(store.getSnapshot().home).toEqual(b);
 });
});
it('conserve profil, historique et position si le rafraîchissement de reconnexion échoue',async()=>{
 let fail=false;
 const store=createPlayerStore({profile:async who=>{if(fail)throw 'unavailable';return profile(who)},matches:async req=>page(req.player)},null,()=>{});
 store.syncAccount(true,a);await flush();store.select(a);store.scroll(240);
 const before=store.getSnapshot().entries[playerKey(a)]!;
 store.syncAccount(false,null);fail=true;store.syncAccount(true,a);await flush();
 const after=store.getSnapshot().entries[playerKey(a)]!;
 expect(after.profile).toEqual(before.profile);expect(after.historyFetchedAt).toBe(before.historyFetchedAt);expect(after.scrollTop).toBe(240);expect(after.error).toBe('unavailable');
 store.refresh(a);await flush();expect(store.getSnapshot().entries[playerKey(a)]!.profile).toEqual(before.profile);
});
it('reprend à zéro un historique dont la réactualisation a échoué, même après sa dernière page',async()=>{
 let fail=false;const starts:number[]=[];
 const store=createPlayerStore({profile:async who=>profile(who),matches:async req=>{starts.push(req.start);if(fail)throw 'unavailable';return page(req.player)}},null,()=>{});
 store.syncAccount(true,a);await flush();store.syncAccount(false,null);fail=true;store.syncAccount(true,a);await flush();
 expect(store.getSnapshot().entries[playerKey(a)]!.historyError).toBe('unavailable');
 fail=false;await store.more(a);
 expect(starts).toEqual([0,0,0]);expect(store.getSnapshot().entries[playerKey(a)]!.historyError).toBeNull();
});
it('empêche une ancienne pagination de prendre la place du rafraîchissement à la reconnexion',async()=>{
 let pending=false,resolveProfile!:(p:Profile)=>void;const starts:number[]=[];
 const store=createPlayerStore({profile:who=>pending?new Promise(resolve=>{resolveProfile=resolve}):Promise.resolve(profile(who)),matches:async req=>{starts.push(req.start);return page(req.player,req.start,req.start===0?10:null)}},null,()=>{});
 store.syncAccount(true,a);await flush();store.syncAccount(false,null);pending=true;store.syncAccount(true,a);
 await store.more(a);expect(starts).toEqual([0]);
 resolveProfile(profile(a));await flush();expect(starts).toEqual([0,0]);
});

it('conserve l’avatar public hors connexion et le remplace sans mélanger les comptes',()=>{
 const saved:unknown[]=[];
 const store=createPlayerStore({profile:async()=>{throw 'not_configured'},matches:async()=>page()},null,value=>saved.push(value));
 store.syncAccount(true,{...a,profile_icon_id:42});
 store.syncAccount(false,null);
 expect(store.getSnapshot().home).toEqual({...a,profile_icon_id:42});
 expect(parseHomePlayer(JSON.stringify(saved[0]))).toEqual({...a,profile_icon_id:42});
 store.syncAccount(true,{...b,profile_icon_id:0});
 expect(store.getSnapshot().home).toEqual({...b,profile_icon_id:0});
 store.syncAccount(false,null);store.forget();
 expect(store.getSnapshot().home).toBeNull();
 for(const id of [-1,1.5,4294967296,'42'])expect(parseHomePlayer(JSON.stringify({...a,profile_icon_id:id}))).toEqual(a);
});
it('ne transmet pas les métadonnées locales d’avatar au contrat strict PlayerRequest',async()=>{
 const requests:unknown[]=[];
 const store=createPlayerStore({profile:async who=>{requests.push(who);return profile(who)},matches:async req=>page(req.player)}, {...a,profile_icon_id:42},()=>{});
 store.start();await flush();
 store.syncAccount(true,{...b,profile_icon_id:43});await flush();
 expect(requests).toEqual([a,b]);
});
