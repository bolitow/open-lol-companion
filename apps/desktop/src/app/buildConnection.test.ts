import {describe,it,expect} from 'vitest';
import {connectBuilds} from './buildConnection';
import type {BuildRequest,BuildReport} from '@olc/shared';
const request:BuildRequest={champion_id:103,patch:'16.19',platform:'EUW1',queue:420,role:'MIDDLE',rank:'ALL'};
const report:BuildReport={request,meta:{source_snapshot_at:'2026-10-01T12:00:00Z',published_at:'2026-10-01T12:05:00Z',min_games:100},builds:[]};
const flush=()=>new Promise(resolve=>setTimeout(resolve,0));
describe('chargement des builds',()=>{
 it('ignore une réponse arrivée après changement de champion ou démontage',async()=>{
  let resolve!:(value:BuildReport)=>void;const states:unknown[]=[];
  const stop=connectBuilds(request,()=>new Promise(done=>{resolve=done}),state=>states.push(state));
  await Promise.resolve();stop();resolve(report);await flush();expect(states).toEqual([{status:'loading'}]);
 });
 it('refuse une réponse pour un autre filtre',async()=>{
  const states:unknown[]=[];connectBuilds(request,async()=>({...report,request:{...request,champion_id:222}}),state=>states.push(state));
  await flush();expect(states.at(-1)).toEqual({status:'error',error:'invalid_response'});
 });
 it('conserve une population vide comme résultat et nettoie les erreurs inconnues',async()=>{
  const states:unknown[]=[];connectBuilds(request,async()=>report,state=>states.push(state));await flush();
  expect(states.at(-1)).toEqual({status:'ready',report});
  connectBuilds(request,async()=>{throw new Error('détail privé')},state=>states.push(state));await flush();
  expect(states.at(-1)).toEqual({status:'error',error:'unavailable'});
 });
});
