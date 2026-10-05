import {useEffect,useState} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import type {BuildReport,BuildRequest} from '@olc/shared';
import type {BuildLoadState} from './buildConnection';
import {buildRequestKey} from './buildModel';
import {readPatchBuilds} from './buildPatch';
import {useBuildPatch,buildPatchStore} from './useBuildPatch';

export function useBuilds(request:BuildRequest|null){
 const original=request?buildRequestKey(request):'';
 const choice=useBuildPatch(request?.patch??'',original);
 const resolved=request&&choice.patch?{...request,patch:choice.patch}:null;
 const key=resolved?`${buildRequestKey(resolved)}:${choice.previous??''}:${choice.revision}`:'', [attempt,setAttempt]=useState(0);
 const [result,setResult]=useState<{key:string;value:BuildLoadState}|null>(null);
 useEffect(()=>{
  if(!resolved)return;
  let active=true;setResult({key,value:{status:'loading'}});
  void readPatchBuilds(resolved,choice.previous,req=>isTauri()?invoke<BuildReport>('community_builds',{request:req}):Promise.reject('desktop_required'))
   .then(report=>{if(active)setResult({key,value:{status:'ready',report}})},error=>{if(active)setResult({key,value:{status:'error',error:typeof error==='string'&&['not_configured','invalid_configuration','invalid_request','unauthorized','unavailable','rate_limited','invalid_response','changed_snapshot','desktop_required'].includes(error)?error as Extract<BuildLoadState,{status:'error'}>['error']:'unavailable'}})});
  return()=>{active=false};
 },[key,attempt]);
 const state:BuildLoadState|null=!request?null:!resolved?(choice.kind==='unavailable'?{status:'error',error:'unavailable'}:{status:'loading'}):result?.key===key?result.value:{status:'loading'};
 return {state,choice,canImport:choice.canImport&&state?.status==='ready'&&state.report.request.patch===choice.requested,retry:()=>{void buildPatchStore.load().then(()=>setAttempt(n=>n+1))}};
}
