import {useCallback, useEffect, useMemo, useState, useSyncExternalStore} from 'react';
import {invoke, isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {CollectionState} from '../../../../../packages/shared/src/collection';
import {collectionAccountKey, collectionViewForAccount, patchCollectionView, initialCollectionView, type CollectionView} from './collectionModel';
import {createCollectionController, type CollectionControllerSnapshot} from './collectionController';

export interface CollectionHookResult extends CollectionControllerSnapshot {
    refresh: () => Promise<void>;
    setWish: (skinId: number, wished: boolean) => Promise<void>;
}
export function useCollection(): CollectionHookResult {
    const native = isTauri();
    const controller = useMemo(() => createCollectionController({
        listen: receive => listen<CollectionState>('collection-state', event => receive(event.payload)),
        read: () => invoke<CollectionState>('collection_state'),
        refresh: () => invoke<void>('collection_refresh'),
        setWish: input => invoke<CollectionState>('collection_set_wish', input),
    }, native), [native]);
    const snapshot = useSyncExternalStore(controller.subscribe, controller.getSnapshot, controller.getSnapshot);
    useEffect(() => {controller.start(); return controller.stop;}, [controller]);
    return {...snapshot, refresh: controller.refresh, setWish: controller.setWish};
}

/** À monter avec useCollection dans App, pour invalider la vue même lorsque la page est fermée. */
export function useCollectionView(state: CollectionState): readonly [CollectionView, (patch: Partial<CollectionView>) => void] {
    const [stored, setStored] = useState(initialCollectionView), key = collectionAccountKey(state);
    const view = collectionViewForAccount(stored, key);
    const update = useCallback((patch: Partial<CollectionView>) => {
        setStored(current => patchCollectionView(collectionViewForAccount(current, key), {...patch, accountKey: key}));
    }, [key]);
    // Enregistrer le changement même si l'utilisateur ne touche pas cette page.
    useEffect(() => {setStored(current => collectionViewForAccount(current, key));}, [key]);
    return [view, update];
}
