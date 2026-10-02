import { useEffect, useState, type Dispatch } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { LcuSession } from '@olc/shared';
import { connectSession } from './connection';
import type { AppAction } from './state';
export function useSession(dispatch: Dispatch<AppAction>) {
    const [error, setError] = useState(false), [attempt, setAttempt] = useState(0);
    const native = isTauri();
    useEffect(() => {
        if (!native)
            return;
        setError(false);
        return connectSession({ listen: receive => listen<LcuSession>('lcu-session', event => receive(event.payload)), read: () => invoke<LcuSession>('lcu_session') }, session => { setError(false); dispatch({ type: 'session', session }); }, () => setError(true));
    }, [dispatch, native, attempt]);
    return { native, error, retry: () => setAttempt(value => value + 1) };
}
