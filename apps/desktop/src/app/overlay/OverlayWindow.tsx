import {useCallback, useEffect, useRef, useState} from 'react';
import type {OverlayEditAction, OverlayState} from '@olc/shared';
import {invoke} from '@tauri-apps/api/core';
import {useLiveSession} from '../live/useLiveSession';
import {OverlayView} from './OverlayView';
import {useOverlayState} from './useOverlayState';
import {createContentSizer} from './contentSize';

/** Fenêtre passive hors édition ; gestes autorisés uniquement pendant une session native. */
export function OverlayWindow() {
    const {state,receive} = useOverlayState(), {session, error} = useLiveSession();
    const [editError,setEditError]=useState(false);
    const queue=useRef(Promise.resolve()),movePending=useRef(false);
    const edit=useCallback((action:OverlayEditAction)=>{
        if(action.type==='move'&&movePending.current)return;
        if(action.type==='move')movePending.current=true;
        queue.current=queue.current.then(async()=>{
            try{receive(await invoke<OverlayState>('overlay_edit',{action}));setEditError(false)}
            catch(error){if(!String(error).includes('stale'))setEditError(true)}
            finally{if(action.type==='move')movePending.current=false}
        });
    },[receive]);
    const lastHeight = useRef(0), sizer = useRef<ReturnType<typeof createContentSizer> | null>(null);
    useEffect(() => {
        const current = createContentSizer(height => invoke('overlay_content_height', {height}));
        sizer.current = current;
        current.update(lastHeight.current);
        return () => {current.dispose(); if (sizer.current === current) sizer.current = null;};
    }, []);
    const resize = useCallback((height: number) => {
        lastHeight.current = height;
        sizer.current?.update(height);
    }, []);
    useEffect(() => {if (state) document.documentElement.lang = state.preferences.locale;}, [state?.preferences.locale]);
    return state ? <OverlayView onEdit={edit} editError={editError} onHeight={resize} state={state} session={error ? null : session}/> : null;
}
