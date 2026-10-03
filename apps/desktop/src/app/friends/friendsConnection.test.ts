import {describe, expect, it, vi} from 'vitest';
import type {FriendsState} from '@olc/shared';
import {connectFriends, initialFriendsState} from './friendsConnection';

const state = (revision: number, status: FriendsState['status'] = 'ready'): FriendsState => ({revision, status, items: [{name: 'Local Friend', game_name: 'Test', tag_line: 'TAG', platform: 'EUW1', icon_id: null, presence: 'online'}]});
const flush = async () => {for (let i = 0; i < 5; i++) await Promise.resolve();};
function deferred<T>() {let resolve!: (value: T) => void, reject!: (error: unknown) => void; const promise = new Promise<T>((yes, no) => {resolve = yes; reject = no;}); return {promise, resolve, reject};}

describe('connexion de la liste d’amis', () => {
    it('attend le client dans le navigateur et charge au démarrage du desktop', () => {
        expect(initialFriendsState(false)).toEqual({revision: 0, status: 'disconnected', items: []});
        expect(initialFriendsState(true)).toEqual({revision: 0, status: 'loading', items: []});
    });

    it('écoute avant de lire et ignore le GET tardif plus ancien que la déconnexion', async () => {
        let event = (_state: FriendsState) => {};
        const pending = deferred<FriendsState>(), order: string[] = [], received: FriendsState[] = [];
        const stop = connectFriends({listen: async receive => {order.push('listen'); event = receive; return () => {};}, read: () => {order.push('read'); return pending.promise;}}, next => received.push(next));
        await flush();
        event(state(5, 'disconnected'));
        pending.resolve(state(4));
        await flush();
        event(state(5)); event(state(3));
        expect(order).toEqual(['listen', 'read']);
        expect(received).toEqual([{revision: 5, status: 'disconnected', items: []}]);
        stop();
    });

    it('accepte la révision initiale zéro puis efface les items des états non prêts', async () => {
        let event = (_state: FriendsState) => {};
        const received: FriendsState[] = [];
        const stop = connectFriends({listen: async receive => {event = receive; return () => {};}, read: async () => state(0)}, next => received.push(next));
        await flush();
        expect(received).toEqual([state(0)]);
        event(state(1, 'loading')); event(state(2, 'unavailable'));
        expect(received.slice(1)).toEqual([{revision: 1, status: 'loading', items: []}, {revision: 2, status: 'unavailable', items: []}]);
        stop();
    });

    it('efface les amis sur erreur de lecture puis reprend au prochain événement récent', async () => {
        let event = (_state: FriendsState) => {};
        const pending = deferred<FriendsState>(), received: FriendsState[] = [];
        const read = vi.fn(() => pending.promise);
        const stop = connectFriends({listen: async receive => {event = receive; return () => {};}, read}, next => received.push(next));
        await flush(); expect(read).toHaveBeenCalledOnce(); event(state(3)); pending.reject(new Error('transport'));
        await flush();
        expect(received.at(-1)).toEqual({revision: 3, status: 'unavailable', items: []});
        event(state(2)); event(state(3));
        expect(received.at(-1)?.status).toBe('unavailable');
        event(state(4)); expect(received.at(-1)).toEqual(state(4));
        stop();
    });

    it('signale un abonnement refusé sans lancer de GET ni afficher d’anciens amis', async () => {
        const read = vi.fn(async () => state(1)), received: FriendsState[] = [];
        const stop = connectFriends({listen: async () => {throw new Error('listen');}, read}, next => received.push(next));
        await flush();
        expect(read).not.toHaveBeenCalled();
        expect(received).toEqual([{revision: 0, status: 'unavailable', items: []}]);
        stop();
    });

    it('détache une écoute arrivée après démontage sans lancer de GET', async () => {
        const attached = deferred<() => void>(), detach = vi.fn(), read = vi.fn(async () => state(1)), receive = vi.fn();
        const stop = connectFriends({listen: () => attached.promise, read}, receive);
        stop(); attached.resolve(detach); await flush();
        expect(detach).toHaveBeenCalledOnce(); expect(read).not.toHaveBeenCalled(); expect(receive).not.toHaveBeenCalled();
    });

    it('ignore événements, réponses et erreurs après démontage, avec nettoyage unique', async () => {
        for (const reject of [false, true]) {
            let event = (_state: FriendsState) => {};
            const pending = deferred<FriendsState>(), detach = vi.fn(), receive = vi.fn();
            const stop = connectFriends({listen: async callback => {event = callback; return detach;}, read: () => pending.promise}, receive);
            await flush(); stop(); stop();
            event(state(2));
            if (reject) pending.reject(new Error('late')); else pending.resolve(state(1));
            await flush();
            expect(receive).not.toHaveBeenCalled(); expect(detach).toHaveBeenCalledOnce();
        }
    });
});
it('reprend un premier abonnement refusé avec un délai borné et annule la relance au démontage',async()=>{
 vi.useFakeTimers();
 try {
  let calls=0;const receive=vi.fn();
  const stop=connectFriends({listen:async()=>{calls++;if(calls===1)throw 'unavailable';return ()=>{}},read:async()=>state(4)},receive);
  await flush();expect(calls).toBe(1);
  await vi.advanceTimersByTimeAsync(1000);expect(calls).toBe(2);expect(receive).toHaveBeenLastCalledWith(state(4));stop();
  const refused=vi.fn(async()=>{throw 'unavailable'});
  const cancel=connectFriends({listen:refused,read:async()=>state(1)},()=>{});await flush();cancel();
  await vi.advanceTimersByTimeAsync(5000);expect(refused).toHaveBeenCalledTimes(1);
 }finally{vi.useRealTimers()}
});
