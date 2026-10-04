import type {ApiAccessError,ApiAccessInput,ApiAccessStatus} from '@olc/shared';
export interface ApiAccessAdapter {native:boolean;read:()=>Promise<ApiAccessStatus>;save:(input:ApiAccessInput)=>Promise<ApiAccessStatus>;clear:()=>Promise<ApiAccessStatus>}
/** Le jeton ne figure jamais dans cet état : il ne transite que par l’appel d’enregistrement. */
interface Snapshot {native:boolean;status:ApiAccessStatus|null;pending:boolean;error:ApiAccessError|null;result:'saved'|'cleared'|null}
const errors:ApiAccessError[]=['invalid_configuration','too_long','storage_unavailable','read_failed','write_failed'];
const errorCode=(error:unknown,fallback:ApiAccessError):ApiAccessError=>errors.includes(error as ApiAccessError)?error as ApiAccessError:fallback;
export const canSaveApiAccess=(url:string,token:string)=>url.trim().length>0&&token.trim().length>0;
export function createApiAccessStore(adapter:ApiAccessAdapter){
 let reloadPending=false;
 let state:Snapshot={native:adapter.native,status:null,pending:false,error:null,result:null};const listeners=new Set<()=>void>();
 const update=(patch:Partial<Snapshot>)=>{state={...state,...patch};listeners.forEach(fn=>fn())};
 // Les variables d’environnement restent prioritaires : écrire le trousseau n’aurait aucun effet visible.
 const writable=()=>adapter.native&&!state.pending&&state.status!==null&&state.status.source!=='environment';
 async function run(call:()=>Promise<ApiAccessStatus>,result:'saved'|'cleared'){
  update({pending:true,error:null,result:null});
  try{update({status:await call(),result});return true}
  catch(error){update({error:errorCode(error,'write_failed')});return false}
  finally{finish()}
 }
 function finish(){update({pending:false});if(reloadPending){reloadPending=false;void load()}}
 async function load(){
  if(!adapter.native)return;
  if(state.pending){reloadPending=true;return}
  update({pending:true,error:null});
  try{update({status:await adapter.read()})}catch(error){update({status:null,error:errorCode(error,'read_failed')})}
  finally{finish()}
 }
 return {getSnapshot:()=>state,subscribe:(fn:()=>void)=>{listeners.add(fn);return()=>{listeners.delete(fn)}},load,
  /** Renvoie `true` si le cœur Rust a enregistré : l’appelant efface alors le champ du jeton. */
  save:async(url:string,token:string)=>writable()&&canSaveApiAccess(url,token)?run(()=>adapter.save({url,token}),'saved'):false,
  clear:async()=>{if(writable()&&state.status?.source==='keychain')await run(adapter.clear,'cleared')},
 };
}
export type ApiAccessStore=ReturnType<typeof createApiAccessStore>;
