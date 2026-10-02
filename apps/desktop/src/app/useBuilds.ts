import {useEffect,useState} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import type {BuildReport,BuildRequest} from '@olc/shared';
import {connectBuilds,type BuildLoadState} from './buildConnection';
import {buildRequestKey} from './buildModel';

export function useBuilds(request:BuildRequest|null){
 const key=request?buildRequestKey(request):'', [attempt,setAttempt]=useState(0);
 const [result,setResult]=useState<{key:string;value:BuildLoadState}|null>(null);
 useEffect(()=>{
  if(!request)return;
  return connectBuilds(request,req=>isTauri()?invoke<BuildReport>('community_builds',{request:req}):Promise.reject('desktop_required'),value=>setResult({key,value}));
  // La clé contient toutes les dimensions de la requête.
 },[key,attempt]);
 return {state:!request?null:result?.key===key?result.value:{status:'loading'} as BuildLoadState,retry:()=>setAttempt(n=>n+1)};
}
