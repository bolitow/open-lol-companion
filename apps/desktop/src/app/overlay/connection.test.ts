import {expect, it, vi} from 'vitest';
import type {OverlayState} from '@olc/shared';
import {connectOverlay} from './connection';

const snapshot: OverlayState = {material: 'solid', revision: 0, available: true, visible: false, preview: false, editSession: null, error: null, preferences: {enabled: false, exclusiveFullscreen: false, monitor: 0, x: .02, y: .18, width: .2, opacity: .9, height:0, style:'dark', locale: 'fr'}};
const flush = () => new Promise(resolve => setTimeout(resolve, 0));

it('conserve le dernier événement si la lecture initiale répond en retard', async () => {
    let receive!: (state: OverlayState) => void;
    let resolve!: (state: OverlayState) => void;
    const states: OverlayState[] = [];
    const close = connectOverlay({listen: async callback => {receive = callback; return () => {};}, read: () => new Promise(done => {resolve = done;})}, value => states.push(value), vi.fn());
    await flush();
    expect(receive).toBeTypeOf('function');
    receive({...snapshot, revision: 2, preview: true, visible: true});
    resolve(snapshot);
    await flush();
    expect(states.map(state => [state.revision, state.preview])).toEqual([[2, true]]);
    receive({...snapshot, revision: 1});
    expect(states).toHaveLength(1);
    close();
});

it('libère une écoute résolue après démontage sans lire ni publier', async () => {
    let resolve!: (stop: () => void) => void;
    const stop = vi.fn(), read = vi.fn(), receive = vi.fn();
    const close = connectOverlay({listen: () => new Promise(done => {resolve = done;}), read}, receive, vi.fn());
    expect(resolve).toBeTypeOf('function');
    close(); resolve(stop); await flush();
    expect(stop).toHaveBeenCalledOnce();
    expect(read).not.toHaveBeenCalled();
    expect(receive).not.toHaveBeenCalled();
});

it('signale une erreur publique sans propager le texte brut', async () => {
    let reject!: (reason: Error) => void;
    const error = vi.fn(), receive = vi.fn(), stop = vi.fn();
    const close = connectOverlay({listen: async () => stop, read: () => new Promise((_resolve, fail) => {reject = fail;})}, receive, error);
    await flush();
    expect(reject).toBeTypeOf('function');
    reject(new Error('private transport detail')); await flush();
    expect(error).toHaveBeenCalledWith();
    expect(receive).not.toHaveBeenCalled();
    close(); expect(stop).toHaveBeenCalledOnce();
});
