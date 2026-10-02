import {useEffect, useState} from 'react';
import {invoke, isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {LiveSession} from '@olc/shared';
import {connectLiveSession, type LiveConnectionState} from './liveSession';

export function useLiveSession() {
  const native = isTauri();
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<LiveConnectionState>({session: null, error: false});
  useEffect(() => {
    setState({session: null, error: false});
    if (!native) return;
    return connectLiveSession({
      listen: receive => listen<LiveSession>('live-session', event => receive(event.payload)),
      read: () => invoke<LiveSession>('live_session'),
    }, setState);
  }, [native, attempt]);
  return {...state, native, retry: () => { setState({session: null, error: false}); setAttempt(value => value + 1); }};
}
