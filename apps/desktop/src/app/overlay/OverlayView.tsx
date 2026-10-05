import {useLayoutEffect, useRef, type PointerEvent} from 'react';
import {LiveBuildSummary} from '../live/LiveBuildSummary';
import type {LiveSession, OverlayState, OverlayEditAction} from '@olc/shared';
import {LiveSummary} from '../live/LiveSummary';
import {Icon} from '../../ui/Icon';
import {overlayCopy} from './copy';

export function OverlayView({state, session, onHeight, onEdit, editError}: {state: OverlayState; session: LiveSession | null; onHeight?: (height:number)=>void; onEdit?:(action:OverlayEditAction)=>void; editError?:boolean}) {
    const {preferences, preview} = state, t = overlayCopy[preferences.locale];
    const editing=state.editSession!==null, sessionId=state.editSession;
    const dragging=useRef(false),lastMove=useRef(0);
    const gesture=(kind:'move'|'resize')=>({
        onPointerDown:(event:PointerEvent<HTMLButtonElement>)=>{if(event.button!==0||sessionId===null)return;event.preventDefault();event.currentTarget.setPointerCapture(event.pointerId);dragging.current=true;onEdit?.({type:'begin',session:sessionId,kind})},
        onPointerMove:()=>{if(!dragging.current||sessionId===null||performance.now()-lastMove.current<33)return;lastMove.current=performance.now();onEdit?.({type:'move',session:sessionId})},
        onPointerUp:()=>{if(!dragging.current||sessionId===null)return;dragging.current=false;onEdit?.({type:'end',session:sessionId})},
        onLostPointerCapture:()=>{if(!dragging.current||sessionId===null)return;dragging.current=false;onEdit?.({type:'end',session:sessionId})},
    });
    const panel = useRef<HTMLElement>(null);
    const shown = state.visible && state.available && !preferences.exclusiveFullscreen && (preview || preferences.enabled);
    useLayoutEffect(()=>{
        const node=panel.current;if(!shown||!node||!onHeight)return;
        const report=()=>onHeight(Math.ceil(node.getBoundingClientRect().height));
        const observer=new ResizeObserver(report);observer.observe(node);report();
        return ()=>observer.disconnect();
    },[shown,onHeight]);
    if (!shown) return null;
    return <aside ref={panel} className="game-overlay-panel" data-editing={editing} data-fixed={preferences.height>0} data-style={preferences.style} data-material={state.material} style={{opacity: state.material === 'solid' ? (editing?Math.max(.8,preferences.opacity):preferences.opacity) : 1}} lang={preferences.locale} aria-label={preview ? t.previewTitle : t.title}>
        {editing&&sessionId!==null&&<div className="overlay-edit-toolbar"><button type="button" title={t.move} aria-label={t.move} {...gesture('move')}><Icon name="move" size={16}/></button><span>{t.editing}</span><button type="button" title={t.save} aria-label={t.save} onClick={()=>onEdit?.({type:'commit',session:sessionId})}><Icon name="check" size={16}/></button><button type="button" title={t.cancel} aria-label={t.cancel} onClick={()=>onEdit?.({type:'cancel',session:sessionId})}><Icon name="close" size={16}/></button></div>}
        {editError&&editing&&<p role="alert">{t.failed}</p>}
        <div className="overlay-panel-content"><span className="game-overlay-brand">Open LoL Companion</span>
        {preview ? <div className="game-overlay-preview"><h1>{t.previewTitle}</h1><p>{t.previewBody}</p></div> : <><LiveSummary session={session} locale={preferences.locale}/><LiveBuildSummary session={session} locale={preferences.locale}/></>}
        </div>
        <p className="overlay-legal" tabIndex={editing?0:undefined}>{t.legal}</p>
        {editing&&<button className="overlay-resize" type="button" title={t.resize} aria-label={t.resize} {...gesture('resize')}><Icon name="expand" size={16}/></button>}
    </aside>;
}
