import {useEffect,type RefObject} from "react";
import {createAmbientLoop,emberSeeds,orbSeeds,orbSurfaceState,ambientSizes,contactState,type EmberDepth} from "./ambient";
import {createEmberRenderer} from "./emberShader";
import type {Theme} from "./model";

/** Deux plans WebGL et des contacts locaux partagent une seule horloge suspendable. */
export function AmbientEmbers({root,theme,enabled,scene}:{scene:string;root:RefObject<HTMLDivElement|null>;theme:Theme;enabled:boolean}) {
  useEffect(()=>{
    const host=root.current;
    if(!host||!enabled)return;
    const panels=[...host.querySelectorAll<HTMLElement>(".profile-panel,.session-panel,.goals-panel,.suggestion-panel,.champion-preparation,.draft-build")].filter(p=>!p.closest('.page-hidden,[aria-hidden="true"]')).slice(0,3);
    if(!panels.length)return;
    const layers=(["back","front"] as EmberDepth[]).map(depth=>{
      const canvas=document.createElement("canvas");
      canvas.className=`ambient-embers ambient-${depth}`;canvas.setAttribute("aria-hidden","true");host.append(canvas);
      return {depth,canvas,renderer:createEmberRenderer(canvas,theme==="light",depth)};
    });
    if(layers.some(layer=>!layer.renderer)){
      layers.forEach(layer=>{layer.renderer?.dispose();layer.canvas.remove();});return;
    }
    let rects=panels.map(p=>p.getBoundingClientRect());
    const reflections=panels.flatMap((panel,group)=>[0,1].map(index=>{
      const node=document.createElement("i");node.className="orb-reflection";node.setAttribute("aria-hidden","true");panel.append(node);return {node,group,index};
    }));
    const contacts=panels.map((panel,i)=>{
      const contact=document.createElement("div");
      contact.className=`ambient-contact ${i===1?"contact-lower":"contact-upper"}`;
      contact.setAttribute("aria-hidden","true");
      contact.innerHTML='<i class="contact-bloom"></i><i class="contact-rim"></i>';
      panel.append(contact);return contact;
    });
    let dirty=true,lost=false,disposed=false;
    const disposeLayers=()=>{
      if(disposed)return;disposed=true;
      layers.forEach(layer=>{layer.canvas.removeEventListener("webglcontextlost",contextLost);layer.renderer!.dispose();layer.canvas.remove();});
      contacts.forEach(contact=>contact.remove());reflections.forEach(({node})=>node.remove());
    };
    const measure=()=>{
      const width=window.innerWidth,height=window.innerHeight,sizes=ambientSizes(width,height);
      rects=panels.map(panel=>panel.getBoundingClientRect());
      layers.forEach(({depth,canvas,renderer})=>{
        const size=sizes[depth];
        if(canvas.width!==size.width)canvas.width=size.width;
        if(canvas.height!==size.height)canvas.height=size.height;
        const sparks=emberSeeds(rects,depth),orbs=orbSeeds(rects,depth);
        const points=new Float32Array(sparks.length+orbs.length);points.set(sparks);points.set(orbs,sparks.length);
        renderer!.resize(width,height,points);
      });
      dirty=false;
    };
    let draws=0,total=0,maximum=0;
    const loop=createAmbientLoop(elapsed=>{
      const start=performance.now();
      if(dirty)measure();
      layers.forEach(layer=>layer.renderer!.draw(elapsed));
      contacts.forEach((contact,i)=>{
        const power=contactState(elapsed,i);
        contact.style.opacity=String(power);
      });
      reflections.forEach(({node,group,index})=>{
        const r=rects[group]!,state=orbSurfaceState(elapsed,r.width,r.height,group,index);
        node.style.transform=`translate3d(${state.x-65}px,${state.y-65}px,0)`;
        node.style.opacity=String(state.opacity*.55);
      });
      const duration=performance.now()-start;
      total+=duration;maximum=Math.max(maximum,duration);draws++;
      // Temps CPU des deux soumissions et contacts, sans attente du travail GPU.
      if(draws%30===0){
        const canvas=layers[1]!.canvas;
        canvas.dataset.draws=String(draws);canvas.dataset.submitMs=(total/draws).toFixed(3);canvas.dataset.maxSubmitMs=maximum.toFixed(3);
      }
    },requestAnimationFrame,cancelAnimationFrame);
    const update=()=>{dirty=true;};
    const visibility=()=>{
      if(document.hidden||lost){loop.stop();contacts.forEach(contact=>{contact.style.opacity="0";});}
      else{dirty=true;loop.start();}
      layers.forEach(layer=>{layer.canvas.dataset.state=document.hidden||lost?"paused":"running";});
    };
    const contextLost=()=>{lost=true;loop.stop();disposeLayers();};
    const observer=new ResizeObserver(update);observer.observe(host);panels.forEach(panel=>observer.observe(panel));
    document.addEventListener("visibilitychange",visibility);
    window.addEventListener("resize",update);window.addEventListener("scroll",update,{passive:true});
    layers.forEach(layer=>layer.canvas.addEventListener("webglcontextlost",contextLost));
    visibility();
    return()=>{
      loop.stop();observer.disconnect();document.removeEventListener("visibilitychange",visibility);
      window.removeEventListener("resize",update);window.removeEventListener("scroll",update);
      disposeLayers();
    };
  },[root,theme,enabled,scene]);
  return null;
}
