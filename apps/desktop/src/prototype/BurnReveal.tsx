import { useLayoutEffect, type RefObject } from "react";
import type { Theme } from "./model";
import { gutterPoints, openingState, renderSize, surfaceState, IGNITION_TIME_SCALE } from "./ignition";
import { createFireRenderer } from "./fireShader";

/** Combustion au premier plan, puis braises colorées dans les interstices. */
export function BurnReveal({theme,root,onDone}:{theme:Theme;root:RefObject<HTMLDivElement|null>;onDone:()=>void}) {
  useLayoutEffect(()=>{
    const host=root.current;
    if(!host){onDone();return;}
    const width=window.innerWidth,height=window.innerHeight;
    const light=theme==="light";
    const colors=light?["#6552ff","#238cff","#a3f7ff"]:["#ff251b","#ff8a24","#fff1b5"];
    const makeLayer=(name:string)=>{
      const canvas=document.createElement("canvas");
      canvas.className=`opening-layer ${name}`;canvas.setAttribute("aria-hidden","true");
      canvas.style.width=`${width}px`;canvas.style.height=`${height}px`;
      host.append(canvas);return canvas;
    };
    const front=makeLayer("opening-paper"),back=makeLayer("opening-gutters");
    const size=renderSize(width,height,window.devicePixelRatio||1);
    front.width=size.width;front.height=size.height;
    back.width=width;back.height=height;
    const renderer=createFireRenderer(front,light),ctx=back.getContext("2d");
    host.dataset.fireRenderer=renderer?"webgl":"static";
    const surfaces=[...host.querySelectorAll<HTMLElement>(".dashboard-grid .panel")].map(panel=>({
      panel,points:gutterPoints(panel.getBoundingClientRect()),previousHeat:panel.style.getPropertyValue("--heat"),
    }));
    const surface=host.querySelector<HTMLElement>(".session-material");
    const previousSurfaceStyle=surface?.getAttribute("style");
    let raf=0,ended=false,disposed=false;
    const clear=()=>{
      cancelAnimationFrame(raf);
      if(!disposed){disposed=true;renderer?.dispose();}
      front.remove();back.remove();
      surfaces.forEach(({panel,previousHeat})=>{
        if(previousHeat)panel.style.setProperty("--heat",previousHeat);else panel.style.removeProperty("--heat");
      });
      if(surface){
        if(previousSurfaceStyle===null||previousSurfaceStyle===undefined) surface.removeAttribute("style");
        else surface.setAttribute("style",previousSurfaceStyle);
      }
      delete host.dataset.igniting;
    };
    const finish=()=>{if(ended)return;ended=true;clear();onDone();};
    if(!renderer||!ctx){finish();return clear;}
    const start=performance.now(),radius=Math.hypot(width,height)*.62;
    const draw=(now:number)=>{
      const state=openingState(now-start);
      if(state.done){finish();return;}
      if(state.reveal<1) renderer.draw(state.reveal,(now-start)*IGNITION_TIME_SCALE);
      // Le voile disparaît entièrement avant la fin des braises, sans couture à la sortie.
      front.style.opacity=String(Math.min(1,(1-state.reveal)*10));
      const material=surfaceState(now-start);
      surface?.style.setProperty("--surface-progress",String(material.progress));
      surface?.style.setProperty("--surface-opacity",String(material.opacity));
      surface?.style.setProperty("--material-reveal",String(Math.min(1,material.progress*3)));
      ctx.clearRect(0,0,width,height);
      const wave=radius*state.follow;
      for(const {panel,points} of surfaces){
        const paths=Array.from({length:6},()=>new Path2D());
        let heat=0;
        for(let i=1;i<points.length;i++){
          const a=points[i-1]!,b=points[i]!;
          const distance=Math.hypot(a.x-width/2,a.y-height/2);
          const grain=.55+.45*Math.sin(i*1.71+distance*.03)**2;
          const strength=Math.max(0,1-Math.abs(distance-wave)/100)*state.opacity*grain;
          heat=Math.max(heat,strength);
          if(strength<.07)continue;
          const path=paths[Math.min(5,Math.floor(strength*6))]!;
          path.moveTo(a.x,a.y);path.lineTo(b.x,b.y);
        }
        paths.forEach((path,i)=>{
          const strength=(i+1)/6;
          // Trois températures et trois rayons de diffusion, pas un trait néon uniforme.
          for(let layer=0;layer<3;layer++){
            ctx.strokeStyle=colors[layer]!;ctx.shadowColor=colors[layer]!;
            ctx.globalAlpha=strength*[.24,.5,.65][layer]!;
            ctx.lineWidth=[6,2,.6][layer]!;ctx.shadowBlur=[28,12,3][layer]!;
            ctx.stroke(path);
          }
        });
        panel.style.setProperty("--heat",heat.toFixed(3));
      }
      ctx.globalAlpha=1;ctx.shadowBlur=0;
      raf=requestAnimationFrame(draw);
    };
    host.dataset.igniting="true";
    front.addEventListener("webglcontextlost",finish,{once:true});
    window.addEventListener("resize",finish,{once:true});
    window.addEventListener("scroll",finish,{once:true,passive:true});
    host.addEventListener("pointerdown",finish,{once:true});
    host.addEventListener("keydown",finish,{once:true});
    draw(start);
    return()=>{
      ended=true;front.removeEventListener("webglcontextlost",finish);clear();
      window.removeEventListener("resize",finish);window.removeEventListener("scroll",finish);
      host.removeEventListener("pointerdown",finish);host.removeEventListener("keydown",finish);
    };
  },[theme,root,onDone]);
  return null;
}
