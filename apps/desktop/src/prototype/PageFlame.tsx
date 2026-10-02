import {useLayoutEffect,useRef,type RefObject} from "react";
import {createFireRenderer} from "./fireShader";
import {renderSize} from "./ignition";
import {pageFlameState} from "./pageMotion";
import type {Theme} from "./model";

/** Passage bref : même turbulence que l’ouverture, voile quasi transparent. */
export function PageFlame({root,theme,cue,phase,enabled,onActive}:{root:RefObject<HTMLDivElement|null>;theme:Theme;cue:string;phase:boolean;enabled:boolean;onActive:(value:boolean)=>void}){
 const played=useRef("");
 useLayoutEffect(()=>{
  const host=root.current;if(!enabled){played.current=cue;return;}if(!host||played.current===cue)return;
  const canvas=document.createElement("canvas");canvas.className="page-flame";canvas.setAttribute("aria-hidden","true");
  const size=renderSize(innerWidth,innerHeight,1);canvas.width=size.width;canvas.height=size.height;host.append(canvas);
  const renderer=createFireRenderer(canvas,theme==="light",true);
  if(!renderer){canvas.remove();return;}
  onActive(true);let raf=0,ended=false;
  const finish=()=>{if(ended)return;ended=true;cancelAnimationFrame(raf);canvas.removeEventListener("webglcontextlost",finish);renderer.dispose();canvas.remove();onActive(false);};
  const start=performance.now();
  const draw=(now:number)=>{const state=pageFlameState(now-start,phase);if(state.done){played.current=cue;finish();return;}canvas.style.opacity=String(state.opacity*.85);renderer.draw(state.progress,now-start);raf=requestAnimationFrame(draw);};
  const hide=()=>{if(document.hidden)finish();};
  canvas.addEventListener("webglcontextlost",finish);window.addEventListener("resize",finish);document.addEventListener("visibilitychange",hide);
  draw(start);
  return()=>{finish();window.removeEventListener("resize",finish);document.removeEventListener("visibilitychange",hide);};
 },[root,theme,cue,phase,enabled,onActive]);
 return null;
}
