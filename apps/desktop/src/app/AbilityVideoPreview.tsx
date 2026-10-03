import {useEffect,useId,useLayoutEffect,useRef,useState,type FocusEvent} from 'react';
import {createPortal} from 'react-dom';
import type {CatalogRecord} from '@olc/shared';
import {Icon} from '../ui/Icon';
import {CatalogIcon} from './GameDetails';
import {loadAbilityVideo,type AbilityVideo} from './abilityVideos';
import {useAbilityVideoSession} from './ChampionVideoScope';
import {attachAbilityPlayback,createPreviewGate,type PlaybackState} from './abilityPreviewLifecycle';
import type {Locale} from './state';
import './abilityVideoPreview.css';

const copy={
 fr:{preview:'Démonstration du sort',source:'Vidéo Riot Games',loading:'Chargement de la démonstration…',unavailable:'Démonstration indisponible',unavailableHint:'Les caractéristiques du sort restent disponibles dans la fiche.',play:'Lire la démonstration',pause:'Mettre en pause',retry:'Réessayer',close:'Fermer la démonstration'},
 en:{preview:'Ability preview',source:'Riot Games video',loading:'Loading preview…',unavailable:'Preview unavailable',unavailableHint:'Ability stats remain available in the profile.',play:'Play preview',pause:'Pause preview',retry:'Retry',close:'Close preview'},
};

/** Survol/focus pour l'aperçu ; le clic ouvre la fiche agrandie. */
export function AbilityPreviewButton({record,locale,onOpen}:{record:CatalogRecord;locale:Locale;onOpen:(record:CatalogRecord)=>void}){
 const anchor=useRef<HTMLButtonElement>(null),panel=useRef<HTMLDivElement>(null),gate=useRef<ReturnType<typeof createPreviewGate>|null>(null);
 const [open,setOpen]=useState(false),[position,setPosition]=useState({left:12,top:12}),id=useId(),title=useId(),t=copy[locale];
 useEffect(()=>{const value=createPreviewGate(setOpen);gate.current=value;return()=>{value.dispose();gate.current=null}},[record.id]);
 const close=(restoreFocus=false)=>{
  if(restoreFocus&&panel.current?.contains(document.activeElement))anchor.current?.focus({preventScroll:true});
  gate.current?.close();
 };
 const leaveFocus=(event:FocusEvent)=>{
  const target=event.relatedTarget;
  if(!(target instanceof Node)||!panel.current?.contains(target)&&!anchor.current?.contains(target))close();
 };
 const leavePointer=()=>{if(!panel.current?.contains(document.activeElement)&&document.activeElement!==anchor.current)gate.current?.leave()};
 useLayoutEffect(()=>{
  if(!open||!anchor.current||!panel.current)return;
  const positionPanel=()=>{
   const a=anchor.current!.getBoundingClientRect(),p=panel.current!.getBoundingClientRect(),gap=12;
   const left=a.left-p.width-gap>=gap?a.left-p.width-gap:a.right+p.width+gap<=window.innerWidth-gap?a.right+gap:Math.max(gap,Math.min(a.left,window.innerWidth-p.width-gap));
   setPosition({left,top:Math.max(gap,Math.min(a.top+a.height/2-p.height/2,window.innerHeight-p.height-gap))});
  };
  positionPanel();const observer=new ResizeObserver(positionPanel);observer.observe(panel.current);
  const hide=()=>close(),visibility=()=>{if(document.hidden)close()};
  const escape=(event:KeyboardEvent)=>{if(event.key==='Escape'){event.preventDefault();event.stopPropagation();close(true)}};
  window.addEventListener('keydown',escape,true);window.addEventListener('resize',hide);window.addEventListener('blur',hide);document.addEventListener('visibilitychange',visibility);document.addEventListener('scroll',hide,true);
  return()=>{observer.disconnect();window.removeEventListener('keydown',escape,true);window.removeEventListener('resize',hide);window.removeEventListener('blur',hide);document.removeEventListener('visibilitychange',visibility);document.removeEventListener('scroll',hide,true)};
 },[open]);
 return <>
  <button ref={anchor} type="button" className="catalog-button ability-preview-trigger" aria-label={`${record.name} · ${t.preview}`} aria-controls={open?id:undefined} aria-expanded={open}
   onMouseEnter={()=>gate.current?.enter()} onMouseLeave={leavePointer} onFocus={()=>{if(!anchor.current?.dataset.abilityPreviewReturn)gate.current?.enter()}} onBlur={leaveFocus}
   onKeyDown={event=>{if(open&&event.key==='Tab'&&!event.shiftKey){const control=panel.current?.querySelector<HTMLButtonElement>('button');if(control){event.preventDefault();control.focus({preventScroll:true})}}}}
   onClick={()=>{anchor.current?.focus({preventScroll:true});close();onOpen(record)}}><CatalogIcon key={record.id} record={record}/><span className="ability-preview-mark"><Icon name="play" size={9}/></span></button>
  {open&&createPortal(<div ref={panel} id={id} role="dialog" aria-modal="false" aria-labelledby={title} className="ability-video-popover" style={position}
   onMouseEnter={()=>gate.current?.retain()} onMouseLeave={leavePointer} onBlur={leaveFocus}
   onKeyDown={event=>{if(event.key==='Tab'&&event.shiftKey&&event.target===panel.current?.querySelector('button')){event.preventDefault();anchor.current?.focus({preventScroll:true})}}}>
   <header><div><small>{t.preview}</small><strong id={title}>{record.name}</strong></div><button type="button" className="icon-button" aria-label={t.close} onClick={()=>close(true)}><Icon name="close" size={16}/></button></header>
   <AbilityVideoPreview key={record.id} abilityId={record.id} locale={locale} onBeforeRetry={()=>panel.current?.querySelector<HTMLButtonElement>('button')?.focus({preventScroll:true})}/>
  </div>,anchor.current?.closest('dialog')??document.body)}
 </>;
}

export function AbilityVideoPreview({abilityId,locale,onBeforeRetry}:{abilityId:string;locale:Locale;onBeforeRetry:()=>void}){
 const [media,setMedia]=useState<AbilityVideo|null>(null),[loading,setLoading]=useState(true),[failed,setFailed]=useState(false),[attempt,setAttempt]=useState(0),t=copy[locale];
 useEffect(()=>{
  let current=true;setLoading(true);setFailed(false);
  loadAbilityVideo(abilityId).then(value=>{if(current){setMedia(value);setLoading(false)}},()=>{if(current){setFailed(true);setLoading(false)}});
  return()=>{current=false};
 },[abilityId,attempt]);
 const retry=()=>{onBeforeRetry();setAttempt(a=>a+1)};
 if(loading)return <div className="ability-video-empty" role="status"><Icon name="play" size={26}/><span>{t.loading}</span></div>;
 if(!media?.sources.length)return <div className="ability-video-empty" role="status"><Icon name="clip" size={26}/><strong>{t.unavailable}</strong><span>{t.unavailableHint}</span>{failed&&<button type="button" className="button" onClick={retry}>{t.retry}</button>}</div>;
 return <AbilityVideoPlayer key={`${abilityId}:${attempt}`} abilityId={abilityId} media={media} locale={locale} onRetry={retry}/>;
}

function AbilityVideoPlayer({abilityId,media,locale,onRetry}:{abilityId:string;media:AbilityVideo;locale:Locale;onRetry:()=>void}){
 const host=useRef<HTMLDivElement>(null),toggle=useRef<HTMLButtonElement>(null),playback=useRef<ReturnType<typeof attachAbilityPlayback>|null>(null),[state,setState]=useState<PlaybackState>('paused'),t=copy[locale],session=useAbilityVideoSession();
 useEffect(()=>{
  if(session===null||session?.disposed)return;
  const retained=session?.get(abilityId,media),element=retained??document.createElement('video');
  const source=media.sources.find(candidate=>element.canPlayType(candidate.type)!=='');
  if(!source){setState('error');return}
  element.poster=media.poster??'';element.disablePictureInPicture=true;element.setAttribute('aria-label',t.preview);element.dataset.abilityId=abilityId;
  host.current?.append(element);
  const reduced=()=>document.documentElement.dataset.motion==='reduced'||window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const controller=attachAbilityPlayback(element,source.src,!reduced(),setState,!!retained);playback.current=controller;
  const observer=new MutationObserver(()=>{if(reduced())controller.pause()});observer.observe(document.documentElement,{attributes:true,attributeFilter:['data-motion']});
  return()=>{observer.disconnect();controller.dispose();element.remove();playback.current=null};
 },[abilityId,media,session,t.preview]);
 return <>
  <div className="ability-video-stage" style={{aspectRatio:`${media.width} / ${media.height}`}} data-playback={state}>
   <div ref={host} className="ability-video-host"/>
   {state==='loading'&&<span className="ability-video-status" role="status">{t.loading}</span>}
   {state==='error'&&<div className="ability-video-failure" role="status"><strong>{t.unavailable}</strong></div>}
   {state==='paused'&&<button type="button" className="ability-video-play" aria-label={t.play} onClick={()=>{toggle.current?.focus({preventScroll:true});void playback.current?.play()}}><Icon name="play" size={26}/></button>}
  </div>
  <footer className="ability-video-footer"><span>{t.source}</span><button ref={toggle} type="button" className="ability-video-toggle" onClick={()=>state==='error'?onRetry():state==='playing'?playback.current?.pause():void playback.current?.play()}>{state==='error'?t.retry:state==='playing'?t.pause:t.play}</button></footer>
 </>;
}
