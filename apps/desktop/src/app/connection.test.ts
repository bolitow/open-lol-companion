import { it, expect, vi } from 'vitest';
import { connectSession } from './connection';
import type { LcuSession } from '@olc/shared';
it('abonne avant de lire et libère une écoute résolue après démontage', async () => {
    const stop = vi.fn();
    let resolve!: (v: () => void) => void;
    const listen = vi.fn(() => new Promise<() => void>(r => { resolve = r; }));
    const read = vi.fn();
    const close = connectSession({ listen, read }, vi.fn(), vi.fn());
    close();
    resolve(stop);
    await Promise.resolve();
    expect(stop).toHaveBeenCalledOnce();
    expect(read).not.toHaveBeenCalled();
});
it('transmet les événements et le snapshot sans masquer une erreur', async () => {
    const stop = vi.fn();
    const receive = vi.fn();
    const error = vi.fn();
    let event!: (s: LcuSession) => void;
    const close = connectSession({ listen: async (cb) => { event = cb; return stop; }, read: async () => ({ revision: 0, connected: false, phase: null, draft:null, runePage:null, account:null }) }, receive, error);
    await new Promise(r => setTimeout(r, 0));
    event({ revision: 1, connected: true, phase: 'Lobby', draft:null, runePage:null, account:null });
    expect(receive).toHaveBeenCalledTimes(2);
    expect(error).not.toHaveBeenCalled();
    close();
    expect(stop).toHaveBeenCalledOnce();
});
it('signale les erreurs sans exposer leur texte brut', async () => {
    const error = vi.fn();
    const close = connectSession({ listen: async () => { throw new Error('private'); }, read: vi.fn() }, vi.fn(), error);
    await new Promise(r => setTimeout(r, 0));
    expect(error).toHaveBeenCalledWith();
    close();
});
