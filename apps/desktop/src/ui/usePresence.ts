import {useLayoutEffect,useState} from 'react';
import {scheduleDeparture} from './presence';
import './surfaceMotion.css';
export const SURFACE_EXIT_MS=180;
/** Garder le dernier contenu pour la sortie ; une réouverture annule son retrait. */
export function usePresence<T>(value:T|null,duration=SURFACE_EXIT_MS){
 const [retained,setRetained]=useState<T|null>(value);
 useLayoutEffect(()=>{
  if(value!==null){setRetained(value);return}
  if(retained===null)return;
  const reduced=document.documentElement.dataset.motion==='reduced'||window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  return scheduleDeparture(()=>setRetained(null),duration,reduced);
 },[value,retained,duration]);
 return {value:value??retained,present:value!==null||retained!==null,closing:value===null&&retained!==null};
}
