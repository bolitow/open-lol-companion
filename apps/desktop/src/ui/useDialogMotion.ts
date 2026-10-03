import {useCallback,useEffect,useRef,useState} from 'react';
import {createDialogDeparture} from './dialogDeparture';
import './surfaceMotion.css';
/** Le parent reste monté et modal jusqu’à la fin de la sortie. */
export function useDialogMotion(onClose:()=>void,blocked=false){
 const [closing,setClosing]=useState(false),latest=useRef({onClose,blocked});latest.current={onClose,blocked};
 const [controller]=useState(()=>createDialogDeparture(()=>latest.current.onClose(),()=>setClosing(true),()=>document.documentElement.dataset.motion==='reduced'||window.matchMedia('(prefers-reduced-motion: reduce)').matches));
 useEffect(()=>()=>controller.cancel(),[controller]);
 const close=useCallback(()=>{if(!latest.current.blocked)controller.close()},[controller]);
 const reopen=useCallback(()=>{controller.cancel();setClosing(false)},[controller]);
 return {closing,state:closing?'closed' as const:'open' as const,close,reopen};
}
