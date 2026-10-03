import {loadAbilityVideo,type AbilityVideo} from './abilityVideos';
import type {PreviewVideo} from './abilityPreviewLifecycle';

export interface WarmVideo extends PreviewVideo {readyState:number;canPlayType:(type:string)=>CanPlayTypeResult}
/** Au plus cinq lecteurs muets, appartenant uniquement à la fiche ouverte. */
export function createAbilityVideoSession<V extends WarmVideo>(championId:number,createVideo:()=>V,load:(id:string)=>Promise<AbilityVideo|null>=loadAbilityVideo){
 const ids=['Q','W','E','R','passive'].map(slot=>`${championId}:${slot}`);
 const videos=new Map<string,V>(),watchers=new Map<string,()=>void>();let disposed=false;
 const reset=(video:V)=>{video.pause();video.removeAttribute('src');video.load()};
 const ensure=(id:string,media:AbilityVideo):V|null=>{
  if(disposed||!ids.includes(id)||!media.sources.length)return null;
  const video=videos.get(id)??createVideo();
  const source=media.sources.find(candidate=>video.canPlayType(candidate.type)!=='');
  if(!source)return null;
  videos.set(id,video);
  if(!video.src){
   watchers.get(id)?.();
   video.muted=true;video.loop=true;video.playsInline=true;video.preload='auto';
   const stop=()=>{clearTimeout(timeout);video.removeEventListener('canplay',ready);video.removeEventListener('error',failed);watchers.delete(id)};
   const ready=()=>stop(),failed=()=>{stop();reset(video)};
   const timeout=setTimeout(failed,12000);
   watchers.set(id,stop);video.addEventListener('canplay',ready);video.addEventListener('error',failed);
   video.src=source.src;video.load();
  }
  return video;
 };
 for(const id of ids)void load(id).then(media=>{if(media&&!disposed)ensure(id,media)},()=>{});
 return {
  championId,
  get disposed(){return disposed},
  get(id:string,media:AbilityVideo){const video=ensure(id,media);watchers.get(id)?.();return video},
  dispose(){if(disposed)return;disposed=true;for(const stop of watchers.values())stop();for(const video of videos.values())reset(video);videos.clear()},
 };
}
