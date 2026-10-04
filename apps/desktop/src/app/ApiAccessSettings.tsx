import {useEffect,useState,useSyncExternalStore,type FormEvent} from 'react';
import {listen} from '@tauri-apps/api/event';
import {invoke,isTauri} from '@tauri-apps/api/core';
import type {ApiAccessStatus} from '@olc/shared';
import {Icon} from '../ui/Icon';
import {apiAccessCopy} from './apiAccessCopy';
import {canSaveApiAccess,createApiAccessStore,type ApiAccessStore} from './apiAccess';

/** Saisie de l’URL et du jeton : le jeton part une fois vers le cœur Rust puis le champ est vidé. */
export function ApiAccessSettings({locale,storeOverride}:{locale:'fr'|'en';storeOverride?:ApiAccessStore}){
 const t=apiAccessCopy[locale];
 const [store]=useState(()=>storeOverride??createApiAccessStore({native:isTauri(),read:()=>invoke<ApiAccessStatus>('api_access_status'),save:access=>invoke<ApiAccessStatus>('save_api_access',{access}),clear:()=>invoke<ApiAccessStatus>('clear_api_access')}));
 const state=useSyncExternalStore(store.subscribe,store.getSnapshot,store.getSnapshot);
 const [url,setUrl]=useState(''),[token,setToken]=useState(''),[urlTouched,setUrlTouched]=useState(false);
 useEffect(()=>{
  if(!isTauri()||storeOverride){void store.load();return}
  let disposed=false,unlisten:(()=>void)|undefined;
  // S’abonner avant la lecture initiale ; aucune interrogation périodique.
  void listen('api-access-rejected',()=>{void store.load()}).then(stop=>{
   if(disposed){stop();return}unlisten=stop;void store.load();
  }).catch(()=>{if(!disposed)void store.load()});
  return()=>{disposed=true;unlisten?.()};
 },[store,storeOverride]);
 const status=state.status,environment=status?.source==='environment';
 useEffect(()=>{if(!urlTouched&&status?.source==='keychain'&&status.url)setUrl(status.url)},[status,urlTouched]);
 const save=async(event:FormEvent)=>{
  event.preventDefault();
  if(await store.save(url,token)){setToken('');setUrlTouched(false)}
 };
 const message=state.error?t.errors[state.error]:status?.authorizationRejected?(environment?t.rejectedEnvironment:t.rejected):state.result?t[state.result]:status?.error?t.errors[status.error]:status?t[status.source??'none']:state.native?t.loading:null;
 return <section className="surface setting-card api-access-settings" aria-labelledby="api-access-title">
  <header><span className="setting-symbol"><Icon name="shield" size={21}/></span><div><span className="setting-path">{t.path}</span><h2 id="api-access-title">{t.title}</h2></div></header>
  <p>{t.description}</p>
  <form onSubmit={event=>{void save(event)}}>
   <fieldset disabled={!state.native||!status||state.pending||environment}>
    <label>{t.url}<input type="url" value={environment?status?.url??'':url} placeholder={t.urlPlaceholder} autoComplete="off" spellCheck={false} onChange={event=>{setUrl(event.target.value);setUrlTouched(true)}}/></label>
    <label>{t.token}<input type="password" value={token} placeholder={status?.source==='keychain'?t.tokenKept:t.tokenPlaceholder} autoComplete="off" spellCheck={false} onChange={event=>setToken(event.target.value)}/></label>
    <div className="api-access-actions">
     <button type="submit" className="button" disabled={!canSaveApiAccess(url,token)}>{state.pending?t.saving:t.save}</button>
     {status?.source==='keychain'&&<button type="button" className="button" onClick={()=>{void store.clear()}}>{t.clear}</button>}
    </div>
   </fieldset>
  </form>
  {message&&<small className="api-access-status" role={state.error||status?.authorizationRejected?'alert':'status'}>{message}</small>}
  {!state.native&&<small className="api-access-status">{t.desktop}</small>}
 </section>;
}
