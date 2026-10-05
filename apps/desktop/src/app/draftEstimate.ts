import {scoreDraft,roleFromDraftPosition,type DraftSession,type DraftStatsRequest,type DraftStatsReport} from '@olc/shared';
/** Interrupteur produit indépendant du réglage personnel (#181). */
export const DRAFT_ESTIMATE_ENABLED=true;
export function draftStatsRequest(enabled:boolean,draft:DraftSession|null,patch:string|null,platform:string|null,rank:string,rankReady:boolean):DraftStatsRequest|null {
 // Les modes non classés et personnalisés nécessitent une population explicitement validée.
 if(!DRAFT_ESTIMATE_ENABLED||!enabled||!rankReady||!draft?.supported||draft.customGame||![420,440].includes(draft.queueId??0)||!patch||!platform)return null;
 return {patch,platform,queue:draft.queueId!,rank};
}
export function estimateDraft(enabled:boolean,draft:DraftSession|null,request:DraftStatsRequest|null,report:DraftStatsReport|null,score=scoreDraft){
 if(!DRAFT_ESTIMATE_ENABLED||!enabled||!draft?.supported||!request||!report)return null;
 return score({population:{patch:request.patch,platform_id:request.platform,queue_id:request.queue,rank:request.rank},min_games:report.meta.min_games,role:roleFromDraftPosition(draft.allies.find(p=>p.local)?.position??null)??'UNKNOWN',stats:report.entries,
  allies:draft.allies.filter(p=>p.championId!==null).map(p=>({champion_id:p.championId!,role:roleFromDraftPosition(p.position),local:p.local})),
  enemies:draft.enemies.filter(p=>p.locked&&p.championId!==null).map(p=>p.championId!),bans:[...draft.allyBans,...draft.enemyBans]});
}
export type DraftStatsState={key:string;status:'idle'|'loading'|'error';report:null}|{key:string;status:'ready';report:DraftStatsReport};
export const draftStatsKey=(r:DraftStatsRequest,revision:number)=>JSON.stringify([r.patch,r.platform,r.queue,r.rank,revision]);
/** Une seule population conservée ; un changement de sélection n'entraîne aucun réseau. */
export function createDraftStatsStore(read:(request:DraftStatsRequest)=>Promise<DraftStatsReport>){
 let state:DraftStatsState={key:'',status:'idle',report:null},epoch=0,current:DraftStatsRequest|null=null,revision=0;
 const listeners=new Set<()=>void>();
 const publish=(next:DraftStatsState)=>{state=next;listeners.forEach(fn=>fn())};
 const store={getSnapshot:()=>state,subscribe:(fn:()=>void)=>{listeners.add(fn);return()=>{listeners.delete(fn)}},
  async select(request:DraftStatsRequest|null,nextRevision:number,force=false){
   const key=request?draftStatsKey(request,nextRevision):'';
   if(!force&&key===state.key)return;
   const generation=++epoch;current=request;revision=nextRevision;
   if(!request){publish({key,status:'idle',report:null});return}
   publish({key,status:'loading',report:null});
   try{const report=await read(request);if(generation===epoch)publish({key,status:'ready',report})}
   catch{if(generation===epoch)publish({key,status:'error',report:null})}
  },retry:()=>store.select(current,revision,true),
 };return store;
}
