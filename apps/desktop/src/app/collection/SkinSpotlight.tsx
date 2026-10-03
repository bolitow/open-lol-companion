import {useCallback,useEffect,useMemo,useRef,useState} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import type {Locale} from '../state';
import {openSpotlightViewer} from './SpotlightViewer';
import {inlineSpotlightStore} from './spotlightInline';
import {SkinImage} from './SkinImage';
import {Icon} from '../../ui/Icon';
import {loadSkinSpotlight,type SpotlightSegment,type SkinSpotlight as Video} from './skinSpotlights';
const copy={
 fr:{title:'Aperçu en jeu',watch:'Lire la vidéo',thumbnail:'Aperçu indisponible',external:'Ouvrir sur YouTube',opening:'Ouverture…',missing:'Vidéo non référencée',loading:'Recherche de la vidéo…',error:'Catalogue vidéo indisponible.',retry:'Réessayer',failed:'Impossible d’ouvrir le lecteur. Réessaie ou ouvre YouTube.',hint:'Visionneuse intégrée · détachable',published:'Vidéo publiée en'},
 en:{title:'In-game preview',watch:'Play video',thumbnail:'Preview unavailable',external:'Open on YouTube',opening:'Opening…',missing:'Video not listed',loading:'Finding video…',error:'Video catalog unavailable.',retry:'Try again',failed:'Could not open the player. Try again or open YouTube.',hint:'In-app viewer · detachable',published:'Video published in'},
} as const;
export async function openSkinSpotlight(video:Video,external:boolean,segment?:SpotlightSegment,locale:Locale='fr'):Promise<void>{
 if(isTauri()){if(external)await invoke('open_skin_spotlight',{videoId:video.videoId,external:true,...(segment?{startSeconds:segment.start,endSeconds:segment.end}:{})});else await openSpotlightViewer(video.skinId,video.championId,locale,segment?.kind)}
 else window.open(video.source+(segment?`&t=${segment.start}s`:''),'_blank','noopener,noreferrer');
}
export function createSpotlightPlayback(video:Video,transport= openSkinSpotlight){
 let lastSegment:SpotlightSegment|undefined;
 return async(external:boolean,segment?:SpotlightSegment)=>{
  if(!external)lastSegment=segment;
  await transport(video,external,external?(segment??lastSegment):segment);
 };
}
interface ViewProps{locale:Locale;status:'loading'|'ready'|'error';video:Video|null;busy:boolean;openError:boolean;onOpen:(external:boolean,segment?:SpotlightSegment)=>void;onRetry:()=>void}
export function SkinSpotlightView({locale,status,video,busy,openError,onOpen,onRetry}:ViewProps){
 const t=copy[locale];
 return <section className="collection-spotlight" aria-label={t.title}>

  {status==='loading'?<p role="status">{t.loading}</p>:status==='error'?<><p role="alert">{t.error}</p><button className="button" onClick={onRetry}>{t.retry}</button></>:!video?<p>{t.missing}</p>:<>
   <div className="collection-video-surface"><button className="collection-video-preview" disabled={busy} title={t.hint} aria-label={busy?t.opening:t.watch} onClick={()=>onOpen(false)}>
    <SkinImage url={`https://i.ytimg.com/vi/${video.videoId}/hqdefault.jpg`} label={t.thumbnail}/>
    <span className="collection-video-play"><Icon name="play" size={25}/></span>
    <span className="collection-video-caption">{busy?t.opening:t.watch}<Icon name="expand" size={15}/></span>
   </button>
    <button className="collection-spotlight-external" aria-label={t.external} title={`SkinSpotlights · ${t.published} ${video.publishedAt.slice(0,4)} · ${t.external}`} disabled={busy} onClick={()=>onOpen(true)}><Icon name="arrow" size={16}/></button>
   </div>
   {openError&&<p role="alert">{t.failed}</p>}
  </>}
 </section>;
}
export function SkinSpotlight({skinId,championId,locale,enabled=true}:{skinId:number;championId:number;locale:Locale;enabled?:boolean}){
 const slot=useCallback((element:HTMLDivElement|null)=>{if(element&&enabled)return inlineSpotlightStore.register({skinId,element});},[skinId,enabled]);
 const [state,setState]=useState<{status:ViewProps['status'];video:Video|null}>({status:'loading',video:null});
 const generation=useRef(0);
 const [attempt,setAttempt]=useState(0),[busy,setBusy]=useState(false),[openError,setOpenError]=useState(false);
 useEffect(()=>{const current=++generation.current;let active=true;setBusy(false);setState({status:'loading',video:null});setOpenError(false);
  void loadSkinSpotlight(skinId,championId).then(video=>{if(active)setState({status:'ready',video})},()=>{if(active)setState({status:'error',video:null})});
  return()=>{active=false;if(generation.current===current)generation.current++};
 },[skinId,championId,attempt]);
 const playback=useMemo(()=>state.video?createSpotlightPlayback(state.video,(video,external,segment)=>openSkinSpotlight(video,external,segment,locale)):null,[state.video,locale]);
 const open=async(external:boolean,segment?:SpotlightSegment)=>{if(!playback||busy)return;const current=generation.current;setBusy(true);setOpenError(false);try{await playback(external,segment)}catch{if(current===generation.current)setOpenError(true)}finally{if(current===generation.current)setBusy(false)}};
 return <><div className="collection-inline-player" ref={slot}/><SkinSpotlightView locale={locale} {...state} busy={busy} openError={openError} onOpen={(external,segment)=>void open(external,segment)} onRetry={()=>setAttempt(value=>value+1)}/></>;
}
