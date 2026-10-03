import { useCallback, useEffect, useRef, useState, useSyncExternalStore, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { SpotlightAction, SpotlightState, SpotlightSelection, SpotlightMediaState, SpotlightLayoutRequest } from '../../../../../packages/shared/src/spotlight';
import { parsePreferences } from '../state';
import { Icon } from '../../ui/Icon';
import './spotlightViewer.css';
import {inlineSpotlightStore,visibleSpotlightBounds,guardInlineOpen} from './spotlightInline';
import {acceptMediaState} from './spotlightMedia';
const initial: SpotlightState = { revision: 0, video: null, selected: 'full', detached: false, locale: 'fr' };
export const spotlightLabels = {
    fr: { full: 'Vidéo entière', passive: 'Passif', q: 'A', w: 'Z', e: 'E', r: 'R', emotes: 'Emotes', recall: 'Rappel', attack: 'Attaques', movement: 'Déplacements', death: 'Mort' },
    en: { full: 'Full video', passive: 'Passive', q: 'Q', w: 'W', e: 'E', r: 'R', emotes: 'Emotes', recall: 'Recall', attack: 'Attacks', movement: 'Movement', death: 'Death' },
};
const copy = {
 fr:{previous:'Passage précédent',next:'Passage suivant',detach:'Détacher',attach:'Rattacher à l’app',close:'Fermer le lecteur',sections:'Passages du skin',retry:'Recharger',error:'Le lecteur ne répond pas. Recharge la vidéo.',loading:'Ouverture du lecteur…',slow:'Le chargement du lecteur prend plus de temps que prévu.',retryHint:'Recharger en conservant le passage sélectionné',expand:'Agrandir',reduce:'Réduire'},
 en:{previous:'Previous section',next:'Next section',detach:'Detach',attach:'Attach to app',close:'Close player',sections:'Skin sections',retry:'Reload',error:'The player is not responding. Reload the video.',loading:'Opening player…',slow:'The player is taking longer than expected to load.',retryHint:'Reload and keep the selected section',expand:'Expand',reduce:'Reduce'},
};
const timeLabel=(seconds:number)=>`${Math.floor(seconds/60)}:${String(seconds%60).padStart(2,'0')}`;
export function viewerAppearance(raw: string | null, reduced: boolean) { const p = parsePreferences(raw); return { theme: p.theme, locale: p.locale, motion: p.motion && !reduced ? 'full' : 'reduced' }; }
export function acceptSpotlightState(current: SpotlightState, next: SpotlightState) { return next.revision >= current.revision ? next : current; }
export async function openSpotlightViewer(skinId: number, championId: number, locale: 'fr' | 'en', kind: SpotlightSelection = 'full') {
    const owner=inlineSpotlightStore.beginOpen(skinId);
    const state = await invoke<SpotlightState>('skin_spotlight_state');
    if(owner&&inlineSpotlightStore.getSnapshot()!==owner)return state;
    return guardInlineOpen(invoke<SpotlightState>('skin_spotlight_control', { revision: state.revision, action: { type: 'open', skinId, championId, locale, kind } }),()=>!owner||inlineSpotlightStore.getSnapshot()===owner,revision=>invoke('skin_spotlight_control',{revision,action:{type:'close'}}));
}
export function SpotlightViewerContent({state,busy,error,onAction,mediaRef,mediaStatus='idle',inline=false,onToggleSize}: {
 state:SpotlightState;busy:boolean;error:boolean;onAction:(action:SpotlightAction)=>void;
 mediaRef?:RefObject<HTMLDivElement|null>;mediaStatus?:SpotlightMediaState['status'];
 inline?:boolean;onToggleSize?:()=>void;
}) {
 const t=copy[state.locale],labels=spotlightLabels[state.locale];
 const selected=state.video?.segments.find(segment=>segment.kind===state.selected);
 return <div className={`spotlight-viewer-content ${inline?'is-inline':''}`}>
  <header><h2 id="spotlight-viewer-title" title={state.video?.name}>{state.video?.name}</h2><div className="spotlight-viewer-actions">
   {onToggleSize&&<button className="icon-button" aria-label={inline?t.expand:t.reduce} title={inline?t.expand:t.reduce} disabled={busy} onClick={onToggleSize}><Icon name="expand" size={16}/></button>}
   <button className="button spotlight-dock" title={state.detached?t.attach:t.detach} aria-label={state.detached?t.attach:t.detach} disabled={busy} onClick={()=>onAction({type:state.detached?'attach':'detach'})}><Icon name="clip" size={16}/><span>{state.detached?t.attach:t.detach}</span></button>
   <button className="icon-button" disabled={busy} aria-label={t.close} title={t.close} onClick={()=>onAction({type:'close'})}><Icon name="close" size={18}/></button>
  </div></header>
  <div className="spotlight-viewer-stage" ref={mediaRef} aria-label="YouTube"><span>{mediaStatus==='loaded'?'':t.loading}</span></div>
  <nav aria-label={t.sections} className="spotlight-viewer-navigation">
   <button className="icon-button" disabled={busy||!state.video?.segments.length} aria-label={t.previous} onClick={()=>onAction({type:'step',direction:-1})}><Icon name="back" size={16}/></button>
   <div aria-live="polite"><strong>{labels[state.selected]}</strong>{selected&&<small>{timeLabel(selected.start)} — {timeLabel(selected.end)}</small>}</div>
   <button className="icon-button" disabled={busy||!state.video?.segments.length} aria-label={t.next} onClick={()=>onAction({type:'step',direction:1})}><Icon name="arrow" size={16}/></button>
  </nav>
  <div className="spotlight-viewer-chapters" role="group" aria-label={t.sections}>{(['full',...state.video?.segments.map(segment=>segment.kind)??[]] as SpotlightSelection[]).map(kind=>{
   const segment=state.video?.segments.find(item=>item.kind===kind);
   const icon=kind==='full'?'play':kind==='recall'?'replay':kind==='attack'?'sword':kind==='emotes'?'sparkles':kind==='death'?'close':kind==='movement'?'arrow':'flash';
   return <button key={kind} title={labels[kind]} aria-label={labels[kind]} disabled={busy} aria-pressed={state.selected===kind} onClick={()=>onAction({type:'select',kind})}>
    <span className="spotlight-chapter-symbol">{['q','w','e','r'].includes(kind)?labels[kind]:<Icon name={icon} size={19}/>}</span>
    <span>{labels[kind]}</span>{segment&&<small>{timeLabel(segment.start)}</small>}
   </button>;
  })}</div>
  <footer><button className="button" title={t.retryHint} disabled={busy} onClick={()=>onAction({type:'retry'})}><Icon name="replay" size={14}/>{t.retry}</button></footer>
  {(error||mediaStatus==='slow'||mediaStatus==='failed')&&<p className="spotlight-viewer-error" role="alert">{!error&&mediaStatus==='slow'?t.slow:t.error}</p>}
 </div>;
}
function ViewerSurface({ state, busy, error, onAction, onError, mediaStatus, inlineTarget, onToggleSize }: {
    state: SpotlightState;
    busy: boolean;
    error: boolean;
    onAction: (action: SpotlightAction) => void;
    onError: () => void;
    mediaStatus: SpotlightMediaState["status"];
    inlineTarget?:HTMLElement;
    onToggleSize?:()=>void;
}) {
    const dialog = useRef<HTMLDialogElement>(null), media = useRef<HTMLDivElement>(null), outside = useRef(false);
    const layoutQueue=useRef(Promise.resolve());
    useEffect(() => {
        if (state.detached || inlineTarget)
            return;
        const trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        const node = dialog.current;
        node?.showModal();
        return () => { node?.close(); if (trigger?.isConnected)
            trigger.focus({ preventScroll: true }); };
    }, [state.detached,inlineTarget]);
    useEffect(() => {
        let active = true, frame = 0;
        const update = () => {
            cancelAnimationFrame(frame);
            frame = requestAnimationFrame(() => {
                const rect = media.current?.getBoundingClientRect();
                const viewport={x:0,y:0,width:window.innerWidth,height:window.innerHeight};
                const clip=inlineTarget?.closest('.collection-detail-content')?.getBoundingClientRect()??viewport;
                const occluded=[...document.querySelectorAll('dialog[open], [role="listbox"]:not([aria-hidden="true"])')].some(node=>node!==dialog.current&&!dialog.current?.contains(node));
                const bounds=rect?visibleSpotlightBounds(rect,clip,viewport,occluded):null;
                // Sérialiser les tailles : aucun ancien redimensionnement ne dépasse le plus récent.
                layoutQueue.current = layoutQueue.current.then(async () => { if (!active)
                    return; try {
                    await invoke('skin_spotlight_layout', { revision: state.revision, bounds } satisfies SpotlightLayoutRequest);
                }
                catch (e) {
                    if (active && !String(e).includes('stale'))
                        onError();
                } });
            });
        };
        const observer = new ResizeObserver(update);
        if (media.current)
            { observer.observe(media.current); if (media.current.parentElement) observer.observe(media.current.parentElement); }
        if (dialog.current)
            observer.observe(dialog.current);
        window.addEventListener('resize', update);
        document.addEventListener('scroll',update,true);
        const overlays=new MutationObserver(update);
        overlays.observe(document.body,{subtree:true,childList:true,attributes:true,attributeFilter:['open','aria-hidden']});
        update();
        return () => { active = false; cancelAnimationFrame(frame); observer.disconnect(); overlays.disconnect(); document.removeEventListener('scroll',update,true); window.removeEventListener('resize', update); };
    }, [state.revision, error, onError,inlineTarget]);
    const content = <SpotlightViewerContent state={state} busy={busy} error={error} onAction={onAction} mediaRef={media} mediaStatus={mediaStatus} inline={Boolean(inlineTarget)} onToggleSize={onToggleSize}/>;
    if(inlineTarget)return createPortal(content,inlineTarget);
    return state.detached ? <main className="spotlight-detached">{content}</main> : createPortal(<dialog ref={dialog} className="spotlight-viewer" aria-labelledby="spotlight-viewer-title" onCancel={e => { e.preventDefault(); onAction({ type: 'close' }); }} onPointerDown={e => { outside.current = e.target === e.currentTarget; }} onClick={e => { if (outside.current && e.target === e.currentTarget)
        onAction({ type: 'close' }); outside.current = false; }} onKeyDown={e => { if (e.target instanceof HTMLInputElement || e.altKey || e.metaKey || e.ctrlKey)
        return; if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
        e.preventDefault();
        onAction({ type: 'step', direction: e.key === 'ArrowLeft' ? -1 : 1 });
    } }}>{content}</dialog>, document.body);
}
export function SpotlightViewerHost({ detached = false }: {
    detached?: boolean;
}) {
    const slot=useSyncExternalStore(inlineSpotlightStore.subscribe,inlineSpotlightStore.getSnapshot,()=>null);
    const [expanded,setExpanded]=useState(false), inlineOwner=useRef<number|null>(null);
    const matchingSlot=slot?.element.isConnected?slot:null;
    const [state, setState] = useState(initial), [busy, setBusy] = useState(false), [error, setError] = useState(false), pending = useRef(false);
    const [mediaState,setMediaState]=useState<SpotlightMediaState>({attempt:0,status:'idle'});
    useEffect(() => {
        if (!isTauri())
            return;
        let active = true, unlisten: (() => void) | undefined;
        const accept = (value: SpotlightState) => { if (active)
            setState(old => acceptSpotlightState(old, value)); };
        void listen<SpotlightState>('skin-spotlight-state', event => accept(event.payload)).then(stop => { if (!active) {
            stop();
            return;
        } unlisten = stop; void invoke<SpotlightState>('skin_spotlight_state').then(accept, () => { if (active)
            setError(true); }); }, () => { if (active)
            setError(true); });
        return () => { active = false; unlisten?.(); };
    }, []);
    useEffect(() => {
        if (!isTauri()) return;
        let active=true, stop:(()=>void)|undefined;
        const accept=(next:SpotlightMediaState)=>{if(active)setMediaState(old=>acceptMediaState(old,next));};
        void listen<SpotlightMediaState>('skin-spotlight-media',event=>accept(event.payload)).then(unlisten=>{
            if(!active){unlisten();return;} stop=unlisten;
            void invoke<SpotlightMediaState>('skin_spotlight_media').then(accept,()=>{if(active)setError(true);});
        },()=>{if(active)setError(true);});
        return()=>{active=false;stop?.();};
    }, []);
    useEffect(() => {
        if (!detached)
            return;
        const media = window.matchMedia('(prefers-reduced-motion: reduce)');
        const apply = () => { let raw = null; try {
            raw = localStorage.getItem('olc.app.preferences');
        }
        catch { /* Thème sombre de repli si stockage inaccessible. */ } const p = viewerAppearance(raw, media.matches); document.documentElement.dataset.theme = p.theme; document.documentElement.dataset.motion = p.motion; document.documentElement.lang = state.locale; };
        apply();
        window.addEventListener('storage', apply);
        media.addEventListener('change', apply);
        return () => { window.removeEventListener('storage', apply); media.removeEventListener('change', apply); };
    }, [detached, state.locale]);
    const action = async (action: SpotlightAction) => {
        if (pending.current)
            return;
        pending.current = true;
        setBusy(true);
        setError(false);
        try {
            const next = await invoke<SpotlightState>('skin_spotlight_control', { revision: state.revision, action });
            setState(old => acceptSpotlightState(old, next));
        }
        catch {
            setError(true);
            void invoke<SpotlightState>('skin_spotlight_state').then(next => setState(old => acceptSpotlightState(old, next)), () => { });
        }
        finally {
            pending.current = false;
            setBusy(false);
        }
    };
    const onError = useCallback(() => setError(true), []);
    const target=matchingSlot?.skinId===state.video?.skinId?matchingSlot?.element:undefined;
    useEffect(()=>{
        if(!state.video){inlineOwner.current=null;setExpanded(false);return;}
        if(detached||state.detached){inlineOwner.current=null;inlineSpotlightStore.releaseOpenOwner();return;}
        if(target){inlineOwner.current=state.video.skinId;return;}
        if(inlineOwner.current!==null&&!pending.current){inlineOwner.current=null;void action({type:'close'});}
    },[target,state.video,state.detached,detached,busy]);
    const openOwner=inlineSpotlightStore.getOpenOwner();
    const abandonedOpen=Boolean(openOwner&&openOwner.skinId===state.video?.skinId&&openOwner!==slot);
    if (!state.video || state.detached !== detached || (!state.detached&&abandonedOpen) || (!state.detached && inlineOwner.current!==null && !target))
        return null;
    return <ViewerSurface state={state} busy={busy} error={error} onAction={a => void action(a)} onError={onError} mediaStatus={mediaState.status} inlineTarget={!detached&&!expanded?target:undefined} onToggleSize={!detached&&target?()=>setExpanded(value=>!value):undefined}/>;
}
