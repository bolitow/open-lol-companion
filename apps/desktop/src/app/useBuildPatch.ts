import {refreshCatalog} from './catalogRuntime';
import {connectPatchPublications} from './patchPublication';
import {useEffect,useSyncExternalStore,useState} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {BuildPatchContext,PublicationState} from '@olc/shared';
import {createPatchStore,resolveBuildPatch} from './buildPatch';
export const buildPatchStore=createPatchStore(()=>isTauri()?invoke<BuildPatchContext>('build_patch_context'):Promise.resolve({client:null,clientError:'unavailable',manifest:null,manifestError:'not_configured'}));
export function useBuildPatch(catalog:string,selection:string){
 const snapshot=useSyncExternalStore(buildPatchStore.subscribe,buildPatchStore.getSnapshot,buildPatchStore.getSnapshot);
 const [verified,setVerified]=useState<string|null>(null);
 useEffect(()=>{let active=true;if(selection)void buildPatchStore.load().then(()=>{if(active)setVerified(selection)});return()=>{active=false}},[selection]);
 return {...resolveBuildPatch(snapshot.pending||verified!==selection?null:snapshot.value,catalog),revision:snapshot.revision};
}
/** Une seule écoute par fenêtre principale ou overlay, aucune boucle de polling. */
export function usePatchRefresh(sessionKey:string){
 useEffect(()=>{buildPatchStore.invalidate();void buildPatchStore.load();void refreshCatalog()},[sessionKey]);
 useEffect(()=>{
  if(!isTauri())return;
  return connectPatchPublications(receive=>listen<PublicationState>('publication-state',event=>receive(event.payload)),()=>invoke<PublicationState>('publication_state'),()=>{buildPatchStore.invalidate();void buildPatchStore.load();void refreshCatalog()});
 },[]);
}
