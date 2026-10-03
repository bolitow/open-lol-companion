import type {FriendsState} from '@olc/shared';

export interface FriendsTransport {
    listen: (receive: (state: FriendsState) => void) => Promise<() => void>;
    read: () => Promise<FriendsState>;
}
export function initialFriendsState(native: boolean): FriendsState {
    return {revision: 0, status: native ? 'loading' : 'disconnected', items: []};
}
/** Écoute avant lecture ; les révisions précédentes ne peuvent pas restaurer une ancienne liste. */
export function connectFriends(transport: FriendsTransport, receive: (state: FriendsState) => void): () => void {
    let active = true, revision = -1;
    let retries=0;
    let retryTimer:ReturnType<typeof setTimeout>|undefined;
    let unlisten: (() => void) | undefined;
    const accept = (state: FriendsState) => {
        if (!active || state.revision <= revision) return;
        revision = state.revision;
        receive(state.status === 'ready' ? state : {...state, items: []});
    };
    const start=async () => {
        try {
            const detach = await transport.listen(accept);
            if (!active) {detach(); return;}
            unlisten = detach;
            accept(await transport.read());
        } catch {
            if (active) {
                receive({revision: Math.max(0, revision), status: 'unavailable', items: []});
                if(!unlisten&&retries++<2)retryTimer=setTimeout(()=>{void start()},1000);
            }
        }
    };
    void start();
    return () => {
        if (!active) return;
        active = false;
        if(retryTimer!==undefined)clearTimeout(retryTimer);
        unlisten?.();
    };
}
