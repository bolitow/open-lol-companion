import type {CollectionState} from '../../../../../packages/shared/src/collection';
import {canEditWishes, collectionAccountKey, initialCollectionState} from './collectionModel';

export interface CollectionTransport {
    listen: (receive: (state: CollectionState) => void) => Promise<() => void>;
    read: () => Promise<CollectionState>;
    refresh: () => Promise<void>;
    setWish: (input: {revision: number; skinId: number; wished: boolean}) => Promise<CollectionState>;
}
export interface CollectionControllerSnapshot {state: CollectionState; pending: boolean; error: 'connection' | 'refresh' | 'wish' | null}
/** Un seul arbitre de révision pour événements, GET initial et réponses de mutation. */
export function createCollectionController(transport: CollectionTransport, native: boolean) {
    let snapshot: CollectionControllerSnapshot = {state: initialCollectionState(native), pending: false, error: null};
    const listeners = new Set<() => void>();
    let active = false, lifecycle = 0, acceptedRevision = -1, command = 0, unlisten: (() => void) | undefined;
    const publish = (patch: Partial<CollectionControllerSnapshot>) => {snapshot = {...snapshot, ...patch}; listeners.forEach(listener => listener());};
    const accept = (state: CollectionState) => {
        if (!active || state.revision <= acceptedRevision) return;
        const changed = collectionAccountKey(state) !== collectionAccountKey(snapshot.state);
        const retained = state.stale && state.account !== null && (acceptedRevision < 0 || !changed);
        acceptedRevision = state.revision;
        // Rust peut conserver un instantané périmé du même compte, jamais celui d'un autre compte.
        const safe = state.status !== 'ready' && !retained ? {...state, skins: [], wishes: []} : state;
        if (changed || state.status !== 'ready' || state.stale) command++;
        publish({state: safe, error: null, ...((changed || state.status !== 'ready' || state.stale) ? {pending: false} : {})});
    };
    const start = () => {
        if (active) return;
        active = true;
        if (!native) return;
        const run = ++lifecycle;
        void (async () => {
            const before = acceptedRevision;
            try {
                const detach = await transport.listen(state => {if (run === lifecycle) accept(state);});
                if (!active || run !== lifecycle) {detach(); return;}
                unlisten = detach;
                const state = await transport.read();
                if (run === lifecycle) accept(state);
            } catch {
                if (active && run === lifecycle && acceptedRevision === before) publish({state: {...snapshot.state, status: 'unavailable', skins: [], wishes: []}, error: 'connection'});
            }
        })();
    };
    const stop = () => {if (!active) return; active = false; lifecycle++; command++; unlisten?.(); unlisten = undefined;};
    const refresh = async () => {
        if (!active || !native || snapshot.pending) return;
        if (!unlisten) {
            stop();
            publish({state: {...snapshot.state, status: 'loading', skins: [], wishes: []}, pending: false, error: null});
            start();
            return;
        }
        const token = ++command;
        publish({pending: true, error: null});
        try {await transport.refresh();} catch {if (active && token === command) publish({error: 'refresh'});}
        finally {if (active && token === command) publish({pending: false});}
    };
    const setWish = async (skinId: number, wished: boolean) => {
        if (!active || !native || snapshot.pending || !canEditWishes(snapshot.state) || !snapshot.state.skins.some(skin => skin.id === skinId)) return;
        const token = ++command, revision = snapshot.state.revision, account = collectionAccountKey(snapshot.state);
        publish({pending: true, error: null});
        try {
            const result = await transport.setWish({revision, skinId, wished});
            if (active && token === command && account === collectionAccountKey(snapshot.state)) accept(result);
        } catch {if (active && token === command) publish({error: 'wish'});}
        finally {if (active && token === command) publish({pending: false});}
    };
    return {start, stop, refresh, setWish, getSnapshot: () => snapshot, subscribe: (listener: () => void) => {listeners.add(listener); return () => {listeners.delete(listener);};}};
}
