import {useLayoutEffect, useRef} from 'react';
import {LiveBuildSummary} from '../live/LiveBuildSummary';
import type {LiveSession, OverlayState} from '@olc/shared';
import {LiveSummary} from '../live/LiveSummary';
import {overlayCopy} from './copy';

export function OverlayView({state, session, onHeight}: {state: OverlayState; session: LiveSession | null; onHeight?: (height:number)=>void}) {
    const {preferences, preview} = state, t = overlayCopy[preferences.locale];
    const panel = useRef<HTMLElement>(null);
    const shown = state.visible && state.available && !preferences.exclusiveFullscreen && (preview || preferences.enabled);
    useLayoutEffect(()=>{
        const node=panel.current;if(!shown||!node||!onHeight)return;
        const report=()=>onHeight(Math.ceil(node.getBoundingClientRect().height));
        const observer=new ResizeObserver(report);observer.observe(node);report();
        return ()=>observer.disconnect();
    },[shown,onHeight]);
    if (!shown) return null;
    return <aside ref={panel} className="game-overlay-panel" data-material={state.material} style={{opacity: state.material === 'solid' ? preferences.opacity : 1}} lang={preferences.locale} aria-label={preview ? t.previewTitle : t.title}>
        <span className="game-overlay-brand">Open LoL Companion</span>
        {preview ? <div className="game-overlay-preview"><h1>{t.previewTitle}</h1><p>{t.previewBody}</p></div> : <><LiveSummary session={session} locale={preferences.locale}/><LiveBuildSummary session={session} locale={preferences.locale}/></>}
    </aside>;
}
