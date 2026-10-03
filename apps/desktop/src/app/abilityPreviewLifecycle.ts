export type PlaybackState='loading'|'playing'|'paused'|'error';
let activePreview:(()=>void)|null=null;
/** Une seule démonstration ouverte, et aucun montage pour un survol accidentel. */
export function createPreviewGate(onChange:(open:boolean)=>void){
 let timer:ReturnType<typeof setTimeout>|undefined,open=false,disposed=false;
 const cancel=()=>clearTimeout(timer);
 const close=()=>{cancel();if(open){open=false;onChange(false)}if(activePreview===close)activePreview=null};
 return {
  enter(delay=300){cancel();if(disposed||open)return;timer=setTimeout(()=>{activePreview?.();activePreview=close;open=true;onChange(true)},delay)},
  leave(){cancel();timer=setTimeout(close,140)},
  retain:cancel,
  close,
  dispose(){disposed=true;close()},
 };
}
export interface PreviewVideo extends EventTarget {
 muted:boolean;loop:boolean;playsInline:boolean;preload:string;src:string;paused:boolean;
 play:()=>Promise<void>;pause:()=>void;load:()=>void;removeAttribute:(name:string)=>void;
}
export function attachAbilityPlayback(video:PreviewVideo,src:string,autoplay:boolean,onChange:(state:PlaybackState)=>void,retainSource=false){
 let disposed=false,failed=false,request=0,timeout:ReturnType<typeof setTimeout>|undefined;
 const clear=()=>clearTimeout(timeout);
 const release=()=>{video.pause();video.removeAttribute('src');video.load()};
 const error=()=>{if(disposed||failed)return;failed=true;request++;clear();release();onChange('error')};
 const waiting=()=>{if(disposed||failed)return;clear();onChange('loading');timeout=setTimeout(error,12000)};
 const playing=()=>{if(disposed||failed)return;clear();onChange('playing')};
 const paused=()=>{if(disposed||failed)return;clear();onChange('paused')};
 video.muted=true;video.loop=true;video.playsInline=true;if(!retainSource)video.preload='none';
 video.addEventListener('playing',playing);video.addEventListener('waiting',waiting);video.addEventListener('pause',paused);video.addEventListener('error',error);
 const play=async()=>{
  if(disposed)return;const current=++request;failed=false;waiting();
  if(!video.src)video.src=src;
  try{await video.play();if(!disposed&&current===request&&!failed)playing()}
  catch(reason){
   if(disposed||current!==request||failed)return;
   if(reason instanceof Error&&(reason.name==='NotAllowedError'||reason.name==='AbortError'))paused();else error();
  }
 };
 if(autoplay)void play();else onChange('paused');
 return {play,pause(){request++;video.pause();paused()},dispose(){
  disposed=true;request++;clear();
  video.removeEventListener('playing',playing);video.removeEventListener('waiting',waiting);video.removeEventListener('pause',paused);video.removeEventListener('error',error);if(retainSource)video.pause();else release();
 }};
}
