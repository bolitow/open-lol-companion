import type {LiveSession} from '@olc/shared';
export interface LiveConnectionState {session: LiveSession | null; error: boolean}
export interface LiveTransport {
  listen: (receive: (session: LiveSession) => void) => Promise<() => void>;
  read: () => Promise<LiveSession>;
}
/** L'abonnement précède la lecture ; une révision ancienne ne remplace jamais un événement. */
export function connectLiveSession(transport: LiveTransport, receive: (state: LiveConnectionState) => void): () => void {
  let active = true;
  let revision = -1;
  let unlisten: (() => void) | undefined;
  const accept = (session: LiveSession) => {
    if (!active || session.revision <= revision) return;
    revision = session.revision;
    receive({session, error: false});
  };
  void (async () => {
    try {
      const detach = await transport.listen(accept);
      if (!active) { detach(); return; }
      unlisten = detach;
      accept(await transport.read());
    } catch {
      if (active) receive({session: null, error: true});
    }
  })();
  return () => {
    if (!active) return;
    active = false;
    unlisten?.();
  };
}
