import {describe,it,expect,vi} from 'vitest';
import type {DraftSession,DraftStatsReport,DraftStatsRequest,DraftChampionStats} from '@olc/shared';
import {createDraftStatsStore,estimateDraft,draftStatsRequest} from './draftEstimate';
const request:DraftStatsRequest={patch:'16.19',platform:'EUW1',queue:420,rank:'ALL'};
const stat=(champion_id:number,wins:number):DraftChampionStats=>({...request,platform_id:request.platform,queue_id:420,role:'MIDDLE',champion_id,games:100,wins,win_rate:wins,pick_rate:10,win_rate_lower_bound:40});
const report:DraftStatsReport={meta:{source_snapshot_at:'2026-10-04',published_at:'2026-10-04',min_games:30},entries:[stat(103,60),stat(99,40)]};
const draft:DraftSession={supported:true,queueId:420,allySide:'blue',allies:[{cellId:0,championId:103,locked:false,local:true,position:'middle',acting:true}],enemies:[{cellId:5,championId:99,locked:true,local:false,position:null,acting:false}],allyBans:[],enemyBans:[],timer:null,localSpells:null};
describe('estimation de draft',()=>{
 it('inclut le prépick allié mais uniquement les picks adverses verrouillés, sans rôle adverse',()=>{
  const value=estimateDraft(true,draft,request,report)!;
  expect(value.teams.ally.estimated_win_rate).toBe(60);
  expect(value.teams.enemy.eligible).toBe(1);
  expect(estimateDraft(true,{...draft,enemies:draft.enemies.map(p=>({...p,locked:false}))},request,report)?.teams.ally.estimated_win_rate).toBeNull();
 });
 it('ne calcule rien si désactivé, hors sélection supportée ou sans données',()=>{
  const score=vi.fn();
  expect(estimateDraft(false,draft,request,report,score)).toBeNull();
  expect(estimateDraft(true,{...draft,supported:false},request,report,score)).toBeNull();
  expect(estimateDraft(true,draft,request,null,score)).toBeNull();
  expect(score).not.toHaveBeenCalled();
 });
 it('recalcule les bans et picks localement sans inférer les postes',()=>{
  const score=vi.fn();
  estimateDraft(true,{...draft,allyBans:[1],enemyBans:[2]},request,report,score);
  expect(score.mock.calls[0]![0]).toMatchObject({bans:[1,2],enemies:[99],allies:[{champion_id:103,role:'MIDDLE',local:true}]});
 });
 it('ne produit pas de faux 50% quand les deux camps ne sont pas couverts',()=>{
  const result=estimateDraft(true,draft,request,{...report,entries:[]})!;
  expect(result.teams.ally.estimated_win_rate).toBeNull();
  expect(result.teams.ally).toMatchObject({champions:1,eligible:0});
 });
 it('borne cette première version aux drafts classées dont la population est connue',()=>{
  expect(draftStatsRequest(true,draft,'16.19','EUW1','ALL',true)).toEqual(request);
  for(const queueId of [undefined,400,1700,0])expect(draftStatsRequest(true,{...draft,queueId},'16.19','EUW1','ALL',true)).toBeNull();
  expect(draftStatsRequest(true,{...draft,customGame:true},'16.19','EUW1','ALL',true)).toBeNull();
  expect(draftStatsRequest(false,draft,'16.19','EUW1','ALL',true)).toBeNull();
  expect(draftStatsRequest(true,draft,null,'EUW1','ALL',true)).toBeNull();
  expect(draftStatsRequest(true,draft,'16.19','EUW1','ALL',false)).toBeNull();
 });
});
describe('chargement de population',()=>{
 it('partage une seule lecture pour les picks, puis recharge au changement de publication',async()=>{
  const read=vi.fn().mockResolvedValue(report),store=createDraftStatsStore(read);
  await store.select(request,1);await store.select({...request},1);
  expect(read).toHaveBeenCalledTimes(1);
  expect(store.getSnapshot().status).toBe('ready');
  await store.select(request,2);expect(read).toHaveBeenCalledTimes(2);
 });
 it('écarte une réponse tardive après désactivation ou changement de population',async()=>{
  let finish!:(r:DraftStatsReport)=>void;
  const store=createDraftStatsStore(()=>new Promise(r=>{finish=r}));
  const first=store.select(request,1);await store.select(null,1);finish(report);await first;
  expect(store.getSnapshot().status).toBe('idle');
 });
 it('rend les erreurs récupérables sans données périmées',async()=>{
  const read=vi.fn().mockRejectedValueOnce('unauthorized').mockResolvedValueOnce(report),store=createDraftStatsStore(read);
  await store.select(request,1);expect(store.getSnapshot().status).toBe('error');
  await store.retry();expect(store.getSnapshot().status).toBe('ready');
 });
});
