import type {BuildPatchContext,BuildReport,BuildRequest} from '@olc/shared';
import {buildRequestKey} from './buildModel';
export type PatchKind='current'|'fallback'|'client_unavailable'|'manifest_unavailable'|'unavailable'|'loading';
export interface PatchChoice {patch:string|null;requested:string|null;previous:string|null;kind:PatchKind;canImport:boolean;catalogPatch:string|null}
export function technicalPatch(version:string):string|null {
 const match=/^(\d{1,3})\.(\d{1,3})(?:\.\d{1,4})?$/.exec(version);
 return match?`${Number(match[1])}.${Number(match[2])}`:null;
}
const number=(p:string)=>{const [a,b]=p.split('.').map(Number);return a!*1000+b!};
export function resolveBuildPatch(value:BuildPatchContext|null,catalog:string):PatchChoice {
 const catalogPatch=technicalPatch(catalog),requested=value?.client?.patch??catalogPatch;
 const base={requested,catalogPatch,previous:null,canImport:false};
 if(!value)return {...base,patch:null,kind:'loading'};
 if(!requested||!technicalPatch(requested))return {...base,patch:null,kind:'unavailable'};
 if(!value.manifest)return {...base,patch:requested,kind:value.client?'manifest_unavailable':'client_unavailable'};
 const patches=[...new Set(value.manifest.versions.map(technicalPatch).filter((p):p is string=>!!p))].filter(p=>number(p)<=number(requested)).sort((a,b)=>number(b)-number(a));
 const patch=patches[0]??null;
 return {...base,patch,previous:patches[1]??null,kind:!patch?'unavailable':!value.client?'client_unavailable':patch===requested?'current':'fallback',canImport:!!patch&&!!value.client&&patch===requested&&patch===catalogPatch};
}
/** Un seul chargement partagé ; une invalidation écarte les réponses de l'ancienne session. */
export function createPatchStore(read:()=>Promise<BuildPatchContext>){
 let state:{value:BuildPatchContext|null;pending:boolean;revision:number;publicationRevision:number}={value:null,pending:false,revision:0,publicationRevision:0};
 let generation=0,flight:Promise<void>|null=null;
 const listeners=new Set<()=>void>();
 const publish=(next:typeof state)=>{state=next;listeners.forEach(fn=>fn())};
 const store={getSnapshot:()=>state,subscribe:(fn:()=>void)=>{listeners.add(fn);return()=>{listeners.delete(fn)}},
  invalidate(){generation++;flight=null;publish({value:null,pending:false,revision:state.revision+1,publicationRevision:state.publicationRevision+1})},
  load():Promise<void>{
   if(flight)return flight;
   const epoch=generation;publish({...state,pending:true});
   flight=read().then(value=>{if(epoch===generation)publish({...state,value,pending:false,revision:state.revision+1})},()=>{if(epoch===generation)publish({...state,value:{client:null,clientError:'unavailable',manifest:null,manifestError:'unavailable'},pending:false,revision:state.revision+1})}).finally(()=>{if(epoch===generation)flight=null});
   return flight;
  },
 };return store;
}
const populated=(r:BuildReport)=>r.builds.length>0||!!r.summary||(r.skill_levels?.length??0)>0||(r.item_events?.length??0)>0;
/** Un seul repli, même champion/region/file/poste/rang ; jamais sur erreur ni pendant l'import. */
export async function readPatchBuilds(request:BuildRequest,previous:string|null,read:(r:BuildRequest)=>Promise<BuildReport>):Promise<BuildReport>{
 const checked=async(r:BuildRequest)=>{const report=await read(r);if(buildRequestKey(report.request)!==buildRequestKey(r))throw 'invalid_response';return report};
 const current=await checked(request);
 if(populated(current)||!previous||number(previous)>=number(request.patch))return current;
 const older=await checked({...request,patch:previous});
 return populated(older)?older:current;
}
