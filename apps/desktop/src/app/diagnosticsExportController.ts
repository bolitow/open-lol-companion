import type {DiagnosticExportResult,DiagnosticExportError,DiagnosticExportRequest} from '@olc/shared';
export type {DiagnosticLocale,DiagnosticExportResult,DiagnosticExportError,DiagnosticExportRequest} from '@olc/shared';
export interface DiagnosticExportAdapter {native:boolean;export:(request:DiagnosticExportRequest)=>Promise<DiagnosticExportResult>}
export interface DiagnosticExportSnapshot {status:'idle'|'pending'|'exported'|'cancelled'|'error';error:DiagnosticExportError|null}
const errors:readonly DiagnosticExportError[]=['busy','unavailable','read_failed','write_failed','too_large','unsupported'];
export function createDiagnosticsExport(adapter:DiagnosticExportAdapter){
 let state:DiagnosticExportSnapshot={status:'idle',error:null};
 const listeners=new Set<()=>void>();
 const publish=(next:DiagnosticExportSnapshot)=>{state=next;listeners.forEach(listener=>listener())};
 return {
  native:adapter.native,getSnapshot:()=>state,
  subscribe:(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener)}},
  reset:()=>{if(state.status!=='pending')publish({status:'idle',error:null})},
  run:async(request:DiagnosticExportRequest)=>{
   if(state.status==='pending')return;
   if(!adapter.native){publish({status:'error',error:'unavailable'});return}
   publish({status:'pending',error:null});
   try{
    const result=await adapter.export({...request});
    if(result!=='exported'&&result!=='cancelled')throw 'unavailable';
    publish({status:result,error:null});
   }catch(error){publish({status:'error',error:errors.includes(error as DiagnosticExportError)?error as DiagnosticExportError:'unavailable'})}
  },
 };
}
export type DiagnosticsExportController=ReturnType<typeof createDiagnosticsExport>;
