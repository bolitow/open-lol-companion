import type {BuildError,BuildReport,BuildRequest} from '@olc/shared';
import {buildRequestKey} from './buildModel';
export type BuildLoadState={status:'loading'}|{status:'ready';report:BuildReport}|{status:'error';error:BuildError};
const errors:BuildError[]=['not_configured','invalid_configuration','invalid_request','unauthorized','unavailable','rate_limited','invalid_response','changed_snapshot','desktop_required'];
/** Une réponse tardive ne remplace jamais le champion ou les filtres courants. */
export function connectBuilds(request:BuildRequest,read:(request:BuildRequest)=>Promise<BuildReport>,receive:(state:BuildLoadState)=>void):()=>void {
 let active=true;receive({status:'loading'});
 Promise.resolve().then(()=>active?read(request):undefined).then(report=>{
  if(!active)return;
  receive(report&&buildRequestKey(report.request)===buildRequestKey(request)?{status:'ready',report}:{status:'error',error:'invalid_response'});
 }).catch(error=>{if(active)receive({status:'error',error:errors.includes(error as BuildError)?error as BuildError:'unavailable'})});
 return()=>{active=false};
}
