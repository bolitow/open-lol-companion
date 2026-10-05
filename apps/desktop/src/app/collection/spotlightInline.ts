export interface InlineSpotlightTarget {skinId:number;element:HTMLElement}
/** Un nettoyage tardif ne peut pas supprimer la fiche suivante. */
export function createInlineSpotlightStore(){
 let current:InlineSpotlightTarget|null=null,openOwner:InlineSpotlightTarget|null=null;const listeners=new Set<()=>void>();
 const notify=()=>listeners.forEach(listener=>listener());
 return {
  getSnapshot:()=>current,
  beginOpen:(skinId:number)=>{openOwner=current?.skinId===skinId?current:null;return openOwner;},
  getOpenOwner:()=>openOwner,
  releaseOpenOwner:()=>{openOwner=null;},
  subscribe:(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener);};},
  register:(target:InlineSpotlightTarget)=>{current=target;notify();return()=>{if(current===target){current=null;notify();}};},
 };
}
export const inlineSpotlightStore=createInlineSpotlightStore();
export type {SpotlightVideoBounds as SpotlightBounds} from '../../../../../packages/shared/src/spotlight';
import type {SpotlightVideoBounds as SpotlightBounds} from '../../../../../packages/shared/src/spotlight';
export function visibleSpotlightBounds(rect:SpotlightBounds,clip:SpotlightBounds,viewport:SpotlightBounds,occluded:boolean):SpotlightBounds|null{
 const inside=(box:SpotlightBounds)=>rect.x>=box.x&&rect.y>=box.y&&rect.x+rect.width<=box.x+box.width+1&&rect.y+rect.height<=box.y+box.height+1;
 return !occluded&&Object.values(rect).every(Number.isFinite)&&rect.width>=200&&rect.height>=200&&inside(clip)&&inside(viewport)?{x:rect.x,y:rect.y,width:rect.width,height:rect.height}:null;
}

/** Une réponse tardive ne ferme que sa révision ; Rust refuse une session plus récente. */
export async function guardInlineOpen<T extends {revision:number;detached:boolean}>(request:Promise<T>,stillPresent:()=>boolean,close:(revision:number)=>Promise<unknown>):Promise<T>{
 const result=await request;
 if(!stillPresent()&&!result.detached)await close(result.revision);
 return result;
}
