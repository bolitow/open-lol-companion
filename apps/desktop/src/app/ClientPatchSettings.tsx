import {useEffect,useState,useSyncExternalStore} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import type {ClientPatch} from '@olc/shared';
import {Icon} from '../ui/Icon';
import {clientPatchCopy,createClientPatchStore,type ClientPatchSnapshot} from './clientPatch';
export function ClientPatchView({locale,state,reload}:{locale:'fr'|'en';state:ClientPatchSnapshot;reload:()=>void}){
 const t=clientPatchCopy[locale];
 return <section className="surface setting-card" aria-labelledby="client-patch-title">
  <header><span className="setting-symbol"><Icon name="info" size={21}/></span><div><span className="setting-path">{t.path}</span><h2 id="client-patch-title">{t.title}</h2></div></header>
  <p role={state.error?'alert':'status'}>{!state.native?t.desktop:state.pending?t.loading:state.error?t.errors[state.error]:state.patch??t.empty}</p>
  <div className="setting-control"><button className="button" disabled={!state.native||state.pending} onClick={reload}>{t.reload}</button></div>
 </section>;
}
export function ClientPatchSettings({locale}:{locale:'fr'|'en'}){
 const [store]=useState(()=>createClientPatchStore(isTauri(),()=>invoke<ClientPatch>('client_patch')));
 const state=useSyncExternalStore(store.subscribe,store.getSnapshot,store.getSnapshot);
 useEffect(()=>{void store.load()},[store]);
 return <ClientPatchView locale={locale} state={state} reload={()=>{void store.load()}}/>;
}
