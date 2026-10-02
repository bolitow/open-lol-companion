import {useEffect,useState,useSyncExternalStore} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import type {PlayerProfile,PlayerHistory} from '@olc/shared';
import {createPlayerStore,parseHomePlayer,type PlayerTransport,type PlayerStore} from './playerStore';
const STORAGE_KEY='olc.app.home-player';
const transport:PlayerTransport={
    profile:request=>isTauri()?invoke<PlayerProfile>('player_profile',{request}):Promise.reject('desktop_required'),
    matches:request=>isTauri()?invoke<PlayerHistory>('player_matches',{request}):Promise.reject('desktop_required'),
};
export function usePlayers(override?:PlayerStore){
    const [store]=useState(()=>{
        if(override)return override;
        let saved=null;try{saved=localStorage.getItem(STORAGE_KEY)}catch{/* Le choix reste disponible en mémoire. */}
        return createPlayerStore(transport,parseHomePlayer(saved),value=>{if(value)localStorage.setItem(STORAGE_KEY,JSON.stringify(value));else localStorage.removeItem(STORAGE_KEY)});
    });
    const state=useSyncExternalStore(store.subscribe,store.getSnapshot);
    useEffect(()=>{store.start()},[store]);
    return {store,state};
}
