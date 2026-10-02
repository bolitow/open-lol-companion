import {useCallback, useEffect, useRef} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {useLiveSession} from '../live/useLiveSession';
import {OverlayView} from './OverlayView';
import {useOverlayState} from './useOverlayState';
import {createContentSizer} from './contentSize';

/** Fenêtre passive : lecture de DTO locaux uniquement, aucune commande de configuration. */
export function OverlayWindow() {
    const {state} = useOverlayState(), {session, error} = useLiveSession();
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
    return state ? <OverlayView onHeight={resize} state={state} session={error ? null : session}/> : null;
}
