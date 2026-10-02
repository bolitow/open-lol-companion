import {useEffect,useRef,useState} from 'react';
import type {createCompanion,CompanionOptions} from './renderer';
import './companion.css';
import {companionCopy} from './copy';
type Props={motion:boolean;theme:'light'|'dark';locale:'fr'|'en';visible?:boolean};
export function FlameCompanion({motion,theme,locale,visible=true}:Props){
 const canvas=useRef<HTMLCanvasElement>(null),controller=useRef<ReturnType<typeof createCompanion>|null>(null);
 const options=useRef<CompanionOptions>({motion,light:theme==='light',visible});options.current={motion,light:theme==='light',visible};
 const [failed,setFailed]=useState(false);
 useEffect(()=>{
  let cancelled=false;
  void import('./renderer').then(({createCompanion})=>{
   if(cancelled||!canvas.current)return;
   try{controller.current=createCompanion(canvas.current,options.current,()=>setFailed(true));}catch{setFailed(true);}
  }).catch(()=>{if(!cancelled)setFailed(true);});
  return()=>{cancelled=true;controller.current?.dispose();controller.current=null;};
 },[]);
 useEffect(()=>{controller.current?.configure(options.current);},[motion,theme,visible]);
 const label=companionCopy[locale].play;
 return <button type="button" className="flame-companion" aria-label={label} title={label} hidden={!visible} disabled={failed} onClick={()=>controller.current?.interact()}>
  <canvas ref={canvas} aria-hidden="true" hidden={failed}/>
  {failed&&<svg viewBox="0 0 64 80" aria-hidden="true"><path d="M32 3C48 22 22 28 44 32c0-8 7-12 7-12 23 35 1 56-19 56S-5 57 12 33c-1 13 8 13 8 3C20 22 36 18 32 3Z" fill="currentColor"/></svg>}
 </button>;
}
