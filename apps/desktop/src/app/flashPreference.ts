import {useSyncExternalStore} from 'react';
import type {FlashSlot} from '@olc/shared';
import {parseFlashSlot} from './spellEditing';
const key='olc.flash-slot',changed='olc-flash-slot-changed';
export function readFlashPreference():FlashSlot|null {
    try{return parseFlashSlot(localStorage.getItem(key));}catch{return null;}
}
export function subscribeFlashPreference(listener:()=>void) {
    const onStorage=(event:StorageEvent)=>{if(event.key===key||event.key===null)listener();};
    window.addEventListener(changed,listener);window.addEventListener('storage',onStorage);
    return ()=>{window.removeEventListener(changed,listener);window.removeEventListener('storage',onStorage);};
}
export function saveFlashPreference(slot:FlashSlot):boolean {
    try{localStorage.setItem(key,JSON.stringify(slot));window.dispatchEvent(new Event(changed));return true;}
    catch{return false;}
}
/** Même préférence explicite dans la préparation manuelle et les imports au prépick. */
export function useFlashPreference() {
    return useSyncExternalStore(subscribeFlashPreference,readFlashPreference,()=>null);
}
