import {useCallback, useEffect, useState} from 'react';
import {invoke, isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {OverlayState} from '@olc/shared';
import {connectOverlay} from './connection';

export function useOverlayState() {
    const native = isTauri();
    const [state, setState] = useState<OverlayState | null>(null);
    const [error, setError] = useState(false);
    const receive = useCallback((next: OverlayState) => {
        setState(previous => previous && previous.revision > next.revision ? previous : next);
        setError(false);
    }, []);
    useEffect(() => {
        if (!native) return;
        return connectOverlay({listen: callback => listen<OverlayState>('overlay-state', event => callback(event.payload)), read: () => invoke<OverlayState>('overlay_state')}, receive, () => setError(true));
    }, [native, receive]);
    return {state, native, error, receive};
}
