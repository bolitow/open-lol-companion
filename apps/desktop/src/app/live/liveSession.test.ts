import {describe, expect, it, vi} from 'vitest';
import type {LiveSession} from '@olc/shared';
import {connectLiveSession, type LiveConnectionState} from './liveSession';

const session = (revision: number): LiveSession => ({revision, generation: 1, status: 'idle', context: null, game: null});
const flush = async () => { await Promise.resolve(); await Promise.resolve(); await Promise.resolve(); };

describe('connexion Live', () => {
  it('écoute avant de lire et ignore la lecture initiale plus ancienne', async () => {
    let receive = (_: LiveSession) => {};
    let finish = (_: LiveSession) => {};
    const states: LiveConnectionState[] = [];
    const order: string[] = [];
    const stop = connectLiveSession({
      listen: async callback => { order.push('listen'); receive = callback; return () => {}; },
      read: () => { order.push('read'); return new Promise(resolve => { finish = resolve; }); },
    }, state => states.push(state));
    await flush();
    receive(session(3));
    finish(session(2));
    await flush();
    receive(session(3));
    receive(session(1));
    expect(order).toEqual(['listen', 'read']);
    expect(states.map(state => state.session?.revision)).toEqual([3]);
    stop();
  });

  it('ferme une écoute attachée après démontage sans lancer de lecture', async () => {
    let attach = (_: () => void) => {};
    const read = vi.fn(async () => session(0));
    const unlisten = vi.fn();
    const states: LiveConnectionState[] = [];
    const stop = connectLiveSession({listen: () => new Promise(resolve => { attach = resolve; }), read}, state => states.push(state));
    await flush();
    stop();
    attach(unlisten);
    await flush();
    expect(unlisten).toHaveBeenCalledOnce();
    expect(read).not.toHaveBeenCalled();
    expect(states).toEqual([]);
  });

  it('n’affiche aucune réponse ou événement après démontage', async () => {
    let receive = (_: LiveSession) => {};
    let finish = (_: LiveSession) => {};
    const unlisten = vi.fn();
    const states: LiveConnectionState[] = [];
    const stop = connectLiveSession({
      listen: async callback => { receive = callback; return unlisten; },
      read: () => new Promise(resolve => { finish = resolve; }),
    }, state => states.push(state));
    await flush();
    stop(); stop();
    finish(session(1)); receive(session(2));
    await flush();
    expect(states).toEqual([]);
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('efface les données si la lecture échoue et reprend sur un nouvel événement', async () => {
    let receive = (_: LiveSession) => {};
    let fail = (_: unknown) => {};
    const states: LiveConnectionState[] = [];
    const stop = connectLiveSession({
      listen: async callback => { receive = callback; return () => {}; },
      read: () => new Promise((_, reject) => { fail = reject; }),
    }, state => states.push(state));
    await flush(); receive(session(1)); fail(new Error('transport'));
    await flush();
    expect(states.at(-1)).toEqual({session: null, error: true});
    receive(session(2));
    expect(states.at(-1)).toEqual({session: session(2), error: false});
    stop();
  });

  it('signale l’échec d’abonnement sans lire un instantané isolé', async () => {
    const read = vi.fn(async () => session(0));
    const states: LiveConnectionState[] = [];
    connectLiveSession({listen: async () => { throw new Error('transport'); }, read}, state => states.push(state));
    await flush();
    expect(read).not.toHaveBeenCalled();
    expect(states).toEqual([{session: null, error: true}]);
  });
});
