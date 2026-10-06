import {publicClientPatch} from './clientPatch';
import {useEffect,useId,useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import type {CatalogRecord} from '@olc/shared';
import {Icon} from '../ui/Icon';
import {useDialogMotion} from '../ui/useDialogMotion';
import {CatalogIcon} from './GameDetails';
import {AbilityMetrics} from './AbilityInfo';
import {abilityCopy,abilityMetrics} from './abilityPresentation';
import {AbilityEffects,effectCopy} from './AbilityEffects';
import {AbilityVideoPreview} from './AbilityVideoPreview';
import {abilitySequence,adjacentAbility} from './abilityDetailModel';
import type {Locale} from './state';
import './abilityDetails.css';

const copy={fr:{close:'Fermer la fiche du sort',previous:'Compétence précédente',next:'Compétence suivante',abilities:'Compétences du champion',passive:'Passif',patch:'Patch'},en:{close:'Close ability details',previous:'Previous ability',next:'Next ability',abilities:'Champion abilities',passive:'Passive',patch:'Patch'}};
const slotLabel=(record:CatalogRecord,locale:Locale)=>({passive:copy[locale].passive,Q:locale==='fr'?'A':'Q',W:locale==='fr'?'Z':'W',E:'E',R:'R'}[record.id.split(':')[1]??'']??'');

export function AbilityDetails({record,records,locale,version,onClose,onOpen}:{record:CatalogRecord;records:CatalogRecord[];locale:Locale;version:string;onClose:()=>void;onOpen:(record:CatalogRecord)=>void}){
 const dialog=useRef<HTMLDialogElement>(null),close=useRef<HTMLButtonElement>(null),previous=useRef<HTMLButtonElement>(null),next=useRef<HTMLButtonElement>(null),body=useRef<HTMLDivElement>(null),outside=useRef(false),title=useId();
 const [trigger]=useState(()=>document.activeElement instanceof HTMLElement?document.activeElement:null),motion=useDialogMotion(onClose),t=copy[locale];
 const sequence=abilitySequence(record,records),champion=records.find(candidate=>candidate.kind==='champion'&&candidate.id===record.id.split(':')[0]&&candidate.namespace===record.namespace&&candidate.locale===record.locale);
 useEffect(()=>{const node=dialog.current;node?.showModal();close.current?.focus({preventScroll:true});return()=>{
  // close() peut lui-même rendre le focus, y compris pendant le replay StrictMode.
  if(trigger)trigger.dataset.abilityPreviewReturn='true';node?.close();
  if(trigger?.isConnected)trigger.focus({preventScroll:true});
  if(trigger)delete trigger.dataset.abilityPreviewReturn;
 }},[trigger]);
 useEffect(()=>{body.current?.scrollTo(0,0)},[record.id]);
 const move=(direction:1|-1)=>{const ability=adjacentAbility(record,records,direction);if(ability){(direction===1?next:previous).current?.focus({preventScroll:true});onOpen(ability)}};
 return createPortal(<dialog ref={dialog} className="game-detail ability-detail motion-surface" data-state={motion.state} inert={motion.closing} aria-labelledby={title}
  onCancel={event=>{event.preventDefault();motion.close()}}
  onPointerDown={event=>{outside.current=event.target===event.currentTarget}}
  onClick={event=>{if(outside.current&&event.target===event.currentTarget)motion.close();outside.current=false}}
  onKeyDown={event=>{if(event.altKey||event.metaKey||event.ctrlKey)return;if(event.key==='ArrowLeft'||event.key==='ArrowRight'){event.preventDefault();move(event.key==='ArrowLeft'?-1:1)}}}>
  <div className="game-detail-shell">
   <header><CatalogIcon key={record.id} record={record}/><div><small>{champion?.name} · {slotLabel(record,locale)} · {t.patch} {publicClientPatch(version)??'—'}</small><h2 id={title} aria-live="polite">{record.name}</h2></div><button ref={close} className="icon-button" onClick={motion.close} aria-label={t.close}><Icon name="close"/></button></header>
   <div ref={body} className="game-detail-body ability-detail-grid">
    <div className="ability-detail-media"><AbilityVideoPreview key={record.id} abilityId={record.id} locale={locale} onBeforeRetry={()=>close.current?.focus({preventScroll:true})}/></div>
    <div className="ability-detail-parameters">
     {abilityMetrics(record,locale,champion).length>0&&<section className="ability-detail-basics"><h3>{abilityCopy[locale].parameters}</h3><AbilityMetrics record={record} locale={locale} champion={champion}/></section>}
     <section className="ability-detail-effects"><h3>{effectCopy[locale].title}</h3><AbilityEffects key={record.id} record={record} version={version} locale={locale}/></section>
    </div>
   </div>
   <nav className="ability-detail-navigation" aria-label={t.abilities}>
    <button ref={previous} className="icon-button" disabled={sequence.length<2} aria-label={t.previous} onClick={()=>move(-1)}><Icon name="back"/></button>
    <div>{sequence.map(ability=><button key={ability.id} aria-label={`${slotLabel(ability,locale)} · ${ability.name}`} aria-pressed={ability.id===record.id} onClick={()=>onOpen(ability)}><CatalogIcon record={ability}/><span>{slotLabel(ability,locale)}</span></button>)}</div>
    <button ref={next} className="icon-button" disabled={sequence.length<2} aria-label={t.next} onClick={()=>move(1)}><Icon name="arrow"/></button>
   </nav>
  </div>
 </dialog>,document.body);
}
