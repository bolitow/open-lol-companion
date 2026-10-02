import type { LcuSession } from '@olc/shared';
export interface SessionTransport {
    listen: (receive: (session: LcuSession) => void) => Promise<() => void>;
    read: () => Promise<LcuSession>;
}
/** Écouter avant de lire évite de perdre un changement pendant l'initialisation. */
export function connectSession(transport: SessionTransport, receive: (session: LcuSession) => void, onError: () => void): () => void {
    let disposed = false, unlisten: (() => void) | undefined;
    void (async () => {
        try {
            const stop = await transport.listen(session => { if (!disposed)
                receive(session); });
            if (disposed) {
                stop();
                return;
            }
            unlisten = stop;
            const snapshot = await transport.read();
            if (!disposed)
                receive(snapshot);
        }
        catch {
            if (!disposed)
                onError();
        }
    })();
    return () => { disposed = true; unlisten?.(); };
}
