import {useEffect, useState} from 'react';
import {invoke, isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {FriendsState} from '@olc/shared';
import {connectFriends, initialFriendsState} from './friendsConnection';

/** Le rafraîchissement et le changement de compte appartiennent au cœur Rust. */
export function useFriends(): FriendsState {
    const native = isTauri();
    const [state, setState] = useState<FriendsState>(() => initialFriendsState(native));
    useEffect(() => {
        setState(initialFriendsState(native));
        if (!native) return;
        return connectFriends({
            listen: receive => listen<FriendsState>('friends-state', event => receive(event.payload)),
            read: () => invoke<FriendsState>('friends_state'),
        }, setState);
    }, [native]);
    return state;
}
