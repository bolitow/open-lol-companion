import {createContext,useContext,useEffect,useRef,useState,useSyncExternalStore,type ReactNode} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import type {DesktopSettings} from '@olc/shared';
import {createSettingsStore,type SettingsStore} from './settingsStore';
import {createDesktopSettingsStore,type DesktopSettingsStore} from './desktopSettings';
interface SettingsContextValue {store:SettingsStore;desktopStore:DesktopSettingsStore;localeError:boolean;retryLocale:()=>void}
const Context=createContext<SettingsContextValue|null>(null);
export function SettingsProvider({children}:{children:ReactNode}){
 const [stores]=useState(()=>{
  const store=createSettingsStore({getItem:key=>localStorage.getItem(key),setItem:(key,value)=>localStorage.setItem(key,value)},()=>desktopStore.clearUndo());
  const desktopStore=createDesktopSettingsStore({native:isTauri(),read:()=>invoke<DesktopSettings>('desktop_settings'),change:(key,value)=>invoke<DesktopSettings>('set_desktop_setting',{key,value})},()=>store.clearUndo());
  return {store,desktopStore};
 });
 const state=useSyncExternalStore(stores.store.subscribe,stores.store.getSnapshot,stores.store.getSnapshot);
 const [localeError,setLocaleError]=useState(false),[localeRetry,setLocaleRetry]=useState(0);
 const localeQueue=useRef<Promise<void>>(Promise.resolve());
 useEffect(()=>{void stores.desktopStore.load()},[stores]);
 useEffect(()=>{
  if(!stores.desktopStore.getSnapshot().native)return;
  let active=true;
  // Sérialiser évite qu'un ancien changement de langue arrive après le plus récent.
  const operation=localeQueue.current.catch(()=>{}).then(()=>invoke<void>('set_desktop_locale',{locale:state.values.locale}));
  localeQueue.current=operation;
  operation.then(()=>{if(active)setLocaleError(false)},()=>{if(active)setLocaleError(true)});
  return()=>{active=false};
 },[stores,state.values.locale,localeRetry]);
 return <Context.Provider value={{...stores,localeError,retryLocale:()=>setLocaleRetry(value=>value+1)}}>{children}</Context.Provider>;
}
export function useSettings(){
 const context=useContext(Context);
 if(!context)throw new Error('SettingsProvider required');
 const {store,desktopStore}=context;
 const state=useSyncExternalStore(store.subscribe,store.getSnapshot,store.getSnapshot);
 const desktopState=useSyncExternalStore(desktopStore.subscribe,desktopStore.getSnapshot,desktopStore.getSnapshot);
 return {...context,state,desktopState};
}
