import { useCallback, useEffect, useRef, useState, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { SpotlightAction, SpotlightState, SpotlightSelection, SpotlightMediaState } from '../../../../../packages/shared/src/spotlight';
import { parsePreferences } from '../state';
import { Icon } from '../../ui/Icon';
import './spotlightViewer.css';
import {acceptMediaState} from './spotlightMedia';
const initial: SpotlightState = { revision: 0, video: null, selected: 'full', detached: false, locale: 'fr' };
export const spotlightLabels = {
    fr: { full: 'Vidéo entière', passive: 'Passif', q: 'A', w: 'Z', e: 'E', r: 'R', emotes: 'Emotes', recall: 'Rappel', attack: 'Attaques', movement: 'Déplacements', death: 'Mort' },
    en: { full: 'Full video', passive: 'Passive', q: 'Q', w: 'W', e: 'E', r: 'R', emotes: 'Emotes', recall: 'Recall', attack: 'Attacks', movement: 'Movement', death: 'Death' },
};
const copy = { fr: { previous: 'Passage précédent', next: 'Passage suivant', detach: 'Détacher', attach: 'Rattacher à l’app', close: 'Fermer le lecteur', sections: 'Passages du skin', retry: 'Réessayer', error: 'Le lecteur ne répond pas. Réessaie ou ouvre YouTube.', youtube: 'Ouvrir sur YouTube', hint: 'Vidéo bloquée ? Réessaie ou ouvre YouTube.', loading: 'Ouverture du lecteur…', slow: 'Le chargement du lecteur prend plus de temps que prévu. Réessaie ou ouvre YouTube.', retryHint: 'Réinitialiser le lecteur en conservant le passage sélectionné'  }, en: { previous: 'Previous section', next: 'Next section', detach: 'Detach', attach: 'Attach to app', close: 'Close player', sections: 'Skin sections', retry: 'Try again', error: 'The player is not responding. Try again or open YouTube.', youtube: 'Open on YouTube', hint: 'Video stuck? Try again or open YouTube.', loading: 'Opening player…', slow: 'The player is taking longer than expected to load. Try again or open YouTube.', retryHint: 'Reset the player and keep the selected section'  } };
export function viewerAppearance(raw: string | null, reduced: boolean) { const p = parsePreferences(raw); return { theme: p.theme, locale: p.locale, motion: p.motion && !reduced ? 'full' : 'reduced' }; }
export function acceptSpotlightState(current: SpotlightState, next: SpotlightState) { return next.revision >= current.revision ? next : current; }
export async function openSpotlightViewer(skinId: number, championId: number, locale: 'fr' | 'en', kind: SpotlightSelection = 'full') {
    const state = await invoke<SpotlightState>('skin_spotlight_state');
    return invoke<SpotlightState>('skin_spotlight_control', { revision: state.revision, action: { type: 'open', skinId, championId, locale, kind } });
}
export function SpotlightViewerContent({ state, busy, error, onAction, mediaRef, mediaStatus = 'idle' }: {
    state: SpotlightState;
    busy: boolean;
    error: boolean;
    onAction: (action: SpotlightAction) => void;
    mediaRef?: RefObject<HTMLDivElement | null>;
    mediaStatus?: SpotlightMediaState["status"];
}) {
    const t = copy[state.locale], labels = spotlightLabels[state.locale];
    const selected = state.video?.segments.find(s => s.kind === state.selected);
    return <div className="spotlight-viewer-content">
  <header><div><small>SkinSpotlights</small><h2 id="spotlight-viewer-title">{state.video?.name}</h2></div><button className="button spotlight-dock" disabled={busy} onClick={() => onAction({ type: state.detached ? 'attach' : 'detach' })}><Icon name="expand" size={16}/>{state.detached ? t.attach : t.detach}</button><button className="icon-button" disabled={busy} aria-label={t.close} onClick={() => onAction({ type: 'close' })}><Icon name="close"/></button></header>
  <div className="spotlight-viewer-stage" ref={mediaRef} aria-label="YouTube"><span>{mediaStatus === 'loaded' ? 'YouTube' : t.loading}</span></div>
  <nav aria-label={t.sections} className="spotlight-viewer-navigation">
   <button className="icon-button" disabled={busy || !state.video?.segments.length} aria-label={t.previous} onClick={() => onAction({ type: 'step', direction: -1 })}><Icon name="back"/></button>
   <div aria-live="polite"><strong>{labels[state.selected]}</strong>{selected && <small>{Math.floor(selected.start / 60)}:{String(selected.start % 60).padStart(2, '0')} — {Math.floor(selected.end / 60)}:{String(selected.end % 60).padStart(2, '0')}</small>}</div>
   <button className="icon-button" disabled={busy || !state.video?.segments.length} aria-label={t.next} onClick={() => onAction({ type: 'step', direction: 1 })}><Icon name="arrow"/></button>
  </nav>
  <div className="spotlight-viewer-chapters" role="group" aria-label={t.sections}>{(['full', ...state.video?.segments.map(s => s.kind) ?? []] as SpotlightSelection[]).map(kind => <button key={kind} disabled={busy} aria-pressed={state.selected === kind} onClick={() => onAction({ type: 'select', kind })}>{labels[kind]}</button>)}</div>
  <footer><small>{t.hint}</small><button className="button" title={t.retryHint} disabled={busy} onClick={() => onAction({ type: 'retry' })}><Icon name="replay" size={14}/>{t.retry}</button><button className="button" disabled={busy} onClick={() => onAction({ type: 'external' })}>{t.youtube}<Icon name="arrow" size={14}/></button></footer>
  {(error || mediaStatus === 'slow' || mediaStatus === 'failed') && <p className="spotlight-viewer-error" role="alert">{!error && mediaStatus === 'slow' ? t.slow : t.error}</p>}
 </div>;
}
function ViewerSurface({ state, busy, error, onAction, onError, mediaStatus }: {
    state: SpotlightState;
    busy: boolean;
    error: boolean;
    onAction: (action: SpotlightAction) => void;
    onError: () => void;
    mediaStatus: SpotlightMediaState["status"];
}) {
    const dialog = useRef<HTMLDialogElement>(null), media = useRef<HTMLDivElement>(null), outside = useRef(false);
    useEffect(() => {
        if (state.detached)
            return;
        const trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        const node = dialog.current;
        node?.showModal();
        return () => { node?.close(); if (trigger?.isConnected)
            trigger.focus({ preventScroll: true }); };
    }, [state.detached]);
    useEffect(() => {
        let active = true, frame = 0, queue = Promise.resolve();
        const update = () => {
            cancelAnimationFrame(frame);
            frame = requestAnimationFrame(() => {
                const rect = media.current?.getBoundingClientRect();
                if (!rect || rect.width < 200 || rect.height < 200)
                    return;
                const bounds = { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
                // Sérialiser les tailles : aucun ancien redimensionnement ne dépasse le plus récent.
                queue = queue.then(async () => { if (!active)
                    return; try {
                    await invoke('skin_spotlight_layout', { revision: state.revision, bounds });
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
        update();
        return () => { active = false; cancelAnimationFrame(frame); observer.disconnect(); window.removeEventListener('resize', update); };
    }, [state.revision, error, onError]);
    const content = <SpotlightViewerContent state={state} busy={busy} error={error} onAction={onAction} mediaRef={media} mediaStatus={mediaStatus}/>;
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
    if (!state.video || state.detached !== detached)
        return null;
    return <ViewerSurface state={state} busy={busy} error={error} onAction={a => void action(a)} onError={onError} mediaStatus={mediaState.status}/>;
}
