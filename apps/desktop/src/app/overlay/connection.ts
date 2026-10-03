import type {OverlayState} from '@olc/shared';

export interface OverlayTransport {
    listen: (receive: (state: OverlayState) => void) => Promise<() => void>;
    read: () => Promise<OverlayState>;
}

/** La lecture initiale ne peut pas écraser un événement plus récent. */
export function connectOverlay(transport: OverlayTransport, receive: (state: OverlayState) => void, onError: () => void): () => void {
    let disposed = false, revision = -1, unlisten: (() => void) | undefined;
    const publish = (state: OverlayState) => {
        if (disposed || state.revision <= revision) return;
        revision = state.revision;
        receive(state);
    };
    void (async () => {
        try {
            const stop = await transport.listen(publish);
            if (disposed) {stop(); return;}
            unlisten = stop;
            publish(await transport.read());
        } catch {
            if (!disposed) onError();
        }
    })();
    return () => {disposed = true; unlisten?.();};
}
