import {createCatalogRevalidation} from './catalogRevalidation';
import {useEffect,useSyncExternalStore,useMemo} from 'react';
import {isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {CatalogRuntimeState} from '@olc/shared';
import {catalogRuntime,refreshCatalog,receiveCatalogState} from './catalogRuntime';
export function useCatalogRuntime(sessionKey:string){
 const state=useSyncExternalStore(catalogRuntime.subscribe,catalogRuntime.getSnapshot,catalogRuntime.getSnapshot);
 const policy=useMemo(()=>createCatalogRevalidation(()=>{void refreshCatalog()}),[]);
 useEffect(()=>{policy.force()},[sessionKey,policy]);
 useEffect(()=>{
  if(!isTauri())return;
  let disposed=false,stop:(()=>void)|undefined;
  void listen<CatalogRuntimeState>('catalog-state',event=>{void receiveCatalogState(event.payload)}).then(fn=>{if(disposed)fn();else stop=fn}).catch(()=>{});
  let timer:ReturnType<typeof setInterval>|undefined;
  const visibility=()=>{clearInterval(timer);if(document.visibilityState==='visible'){policy.focus();timer=setInterval(policy.periodic,1800000)}};
  visibility();window.addEventListener('focus',policy.focus);document.addEventListener('visibilitychange',visibility);
  return()=>{disposed=true;stop?.();clearInterval(timer);window.removeEventListener('focus',policy.focus);document.removeEventListener('visibilitychange',visibility)};
 },[policy]);
 return state;
}
export function CatalogStatus({locale}:{locale:'fr'|'en'}){
 const state=useSyncExternalStore(catalogRuntime.subscribe,catalogRuntime.getSnapshot,catalogRuntime.getSnapshot);
 if(state.status!=='updating'&&state.status!=='error')return null;
 return <span role="status">{state.status==='updating'?(locale==='fr'?'Catalogue : mise à jour…':'Catalog: updating…'):(locale==='fr'?'Catalogue précédent conservé.':'Previous catalog retained.')} {state.status==='error'&&<button onClick={()=>{void refreshCatalog()}}>{locale==='fr'?'Réessayer':'Retry'}</button>}</span>;
}
