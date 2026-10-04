import {describe,expect,it,vi} from 'vitest';
import type {CollectionState} from '../../../../../packages/shared/src/collection';
import {createCollectionController, type CollectionTransport} from './collectionController';
import {initialCollectionState} from './collectionModel';
const account={platform:'EUW1',game_name:'Test',tag_line:'EUW'};
const state=(revision:number,patch:Partial<CollectionState>={}):CollectionState=>({...initialCollectionState(false),revision,status:'ready',account,skins:[{id:103001,champion_id:103,name:'Ahri',ownership:'missing',tile_url:null,splash_url:null,obtainable:true,rarity:null,series_ids:[]}],...patch});
function deferred<T>(){let resolve!:(v:T)=>void,reject!:(e:unknown)=>void;const promise=new Promise<T>((a,b)=>{resolve=a;reject=b});return{promise,resolve,reject}}
const flush=async()=>{for(let i=0;i<8;i++)await Promise.resolve()};
function setup(read:CollectionTransport['read']=async()=>state(1)){
 let event=(_state:CollectionState)=>{};const detach=vi.fn();
 const transport={listen:vi.fn(async(receive:(s:CollectionState)=>void):Promise<()=>void>=>{event=receive;return detach}),read:vi.fn(read),refresh:vi.fn(async()=>{}),setWish:vi.fn(async()=>state(2,{wishes:[103001]}))};
 const controller=createCollectionController(transport,true);return{controller,transport,event:(s:CollectionState)=>event(s),detach};
}
describe('collection : transport et isolation du compte',()=>{
 it('écoute avant lecture et refuse un GET ou une mutation plus anciens',async()=>{
  const read=deferred<CollectionState>(),s=setup(()=>read.promise);s.controller.start();await flush();
  s.event(state(4,{status:'disconnected',stale:true}));read.resolve(state(2));await flush();
  expect(s.controller.getSnapshot().state.revision).toBe(4);expect(s.controller.getSnapshot().state.status).toBe('disconnected');s.controller.stop();
 });
 it('ne publie jamais une réponse de souhait de l’ancien compte après changement',async()=>{
  const pending=deferred<CollectionState>(),s=setup();s.transport.setWish.mockImplementation(()=>pending.promise);s.controller.start();await flush();
  const write=s.controller.setWish(103001,true);expect(s.controller.getSnapshot().state.wishes).toEqual([]);
  s.event(state(3,{account:{...account,game_name:'Autre'},status:'loading',skins:[],wishes:[]}));
  pending.resolve(state(2,{wishes:[103001]}));await write;
  expect(s.controller.getSnapshot().state.account?.game_name).toBe('Autre');expect(s.controller.getSnapshot().state.skins).toEqual([]);expect(s.controller.getSnapshot().pending).toBe(false);s.controller.stop();
 });
 it('garde seulement les données explicitement périmées du même compte pendant une panne',async()=>{
  const s=setup();s.controller.start();await flush();
  s.event(state(2,{status:'loading',stale:true}));expect(s.controller.getSnapshot().state.skins).toHaveLength(1);
  s.event(state(3,{status:'unavailable',stale:true}));expect(s.controller.getSnapshot().state.skins).toHaveLength(1);
  s.event(state(4,{status:'unavailable',stale:false}));expect(s.controller.getSnapshot().state.skins).toEqual([]);
  s.event(state(5,{status:'loading',stale:true,account:{...account,game_name:'Autre'}}));expect(s.controller.getSnapshot().state.skins).toEqual([]);s.controller.stop();
 });
 it('accepte au démarrage le dernier compte explicitement conservé par Rust',async()=>{
  const s=setup(async()=>state(0,{status:'disconnected',stale:true}));s.controller.start();await flush();expect(s.controller.getSnapshot().state.skins).toHaveLength(1);expect(s.controller.getSnapshot().state.stale).toBe(true);s.controller.stop();
 });
 it('transmet la révision, attend la sauvegarde puis expose une erreur générique en cas de refus',async()=>{
  const s=setup();s.controller.start();await flush();await s.controller.setWish(103001,true);
  expect(s.transport.setWish).toHaveBeenCalledWith({revision:1,skinId:103001,wished:true});expect(s.controller.getSnapshot().state.wishes).toEqual([103001]);
  s.transport.setWish.mockRejectedValue(new Error('secret network detail'));await s.controller.setWish(103001,false);
  expect(s.controller.getSnapshot().error).toBe('wish');expect(JSON.stringify(s.controller.getSnapshot())).not.toContain('secret');expect(s.controller.getSnapshot().state.wishes).toEqual([103001]);s.controller.stop();
 });
 it('bloque une mutation hors catalogue, périmée ou avant connexion',async()=>{
  const s=setup();await s.controller.setWish(103001,true);s.controller.start();await flush();await s.controller.setWish(999,true);
  s.event(state(2,{stale:true}));await s.controller.setWish(103001,true);expect(s.transport.setWish).not.toHaveBeenCalled();s.controller.stop();
 });
 it('une erreur de lecture tardive ne détruit pas un événement plus récent',async()=>{
  const read=deferred<CollectionState>(),s=setup(()=>read.promise);s.controller.start();await flush();s.event(state(4));read.reject(new Error('late'));await flush();
  expect(s.controller.getSnapshot().state).toEqual(state(4));expect(s.controller.getSnapshot().error).toBeNull();s.controller.stop();
 });
 it('permet de réessayer un abonnement refusé et n’envoie pas deux souhaits concurrents',async()=>{
  const s=setup();s.transport.listen.mockRejectedValueOnce(new Error('temporary'));s.controller.start();await flush();
  await s.controller.refresh();await flush();expect(s.transport.listen).toHaveBeenCalledTimes(2);expect(s.controller.getSnapshot().state.status).toBe('ready');
  const pending=deferred<CollectionState>();s.transport.setWish.mockImplementation(()=>pending.promise);
  const first=s.controller.setWish(103001,true);await s.controller.setWish(103001,false);expect(s.transport.setWish).toHaveBeenCalledOnce();pending.resolve(state(2,{wishes:[103001]}));await first;s.controller.stop();
 });
 it('ignore la mutation arrivée après démontage et borne les erreurs de rafraîchissement',async()=>{
  const s=setup();s.controller.start();await flush();s.transport.refresh.mockRejectedValueOnce(new Error('private'));await s.controller.refresh();expect(s.controller.getSnapshot().error).toBe('refresh');
  const pending=deferred<CollectionState>();s.transport.setWish.mockImplementation(()=>pending.promise);const write=s.controller.setWish(103001,true);s.controller.stop();pending.resolve(state(2,{wishes:[103001]}));await write;expect(s.controller.getSnapshot().state.wishes).toEqual([]);
 });
 it('signale une écoute indisponible et détache une écoute résolue après arrêt',async()=>{
  const s=setup();s.transport.listen.mockRejectedValue(new Error('private'));s.controller.start();await flush();expect(s.controller.getSnapshot().error).toBe('connection');expect(s.transport.read).not.toHaveBeenCalled();s.controller.stop();
  const late=deferred<()=>void>(),b=setup();b.transport.listen.mockImplementation(()=>late.promise);b.controller.start();b.controller.stop();late.resolve(b.detach);await flush();expect(b.detach).toHaveBeenCalledOnce();expect(b.transport.read).not.toHaveBeenCalled();
 });
});
