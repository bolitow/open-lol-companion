import {createContext,useContext,useState,useSyncExternalStore,type ReactNode} from 'react';
import {createSettingsStore,type SettingsStore} from './settingsStore';
const Context=createContext<SettingsStore|null>(null);
export function SettingsProvider({children}:{children:ReactNode}){
 const [store]=useState(()=>createSettingsStore({getItem:key=>localStorage.getItem(key),setItem:(key,value)=>localStorage.setItem(key,value)}));
 return <Context.Provider value={store}>{children}</Context.Provider>;
}
export function useSettings(){
 const store=useContext(Context);
 if(!store)throw new Error('SettingsProvider required');
 const state=useSyncExternalStore(store.subscribe,store.getSnapshot,store.getSnapshot);
 return {store,state};
}
