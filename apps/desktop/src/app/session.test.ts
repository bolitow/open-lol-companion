import { describe, it, expect } from 'vitest';
import { initialState, reduceApp, parsePreferences } from './state';
import type { LcuSession } from '@olc/shared';
const snapshot = (revision: number, phase: LcuSession['phase'], connected = true): LcuSession => ({ revision, phase, connected, draft:null, runePage:null, account:null });
describe('navigation du client réel', () => {
    it('suit draft, chargement, partie et bilan', () => {
        let state = initialState;
        for (const [revision, phase, screen] of [[1, 'ChampSelect', 'champ-select'], [2, 'GameStart', 'in-game'], [3, 'InProgress', 'in-game'], [4, 'EndOfGame', 'post-game']] as const) {
            state = reduceApp(state, { type: 'session', session: snapshot(revision, phase) });
            expect(state.screen).toBe(screen);
        }
    });
    it('ignore un snapshot initial obsolète et les doublons', () => {
        const draft = reduceApp(initialState, { type: 'session', session: snapshot(4, 'ChampSelect') });
        expect(reduceApp(draft, { type: 'session', session: snapshot(2, 'Lobby') })).toBe(draft);
        const home = reduceApp(draft, { type: 'navigate', screen: 'dashboard' });
        expect(reduceApp(home, { type: 'session', session: snapshot(4, 'ChampSelect') })).toBe(home);
    });
    it('ne vole pas les réglages, mais actualise la session', () => {
        const settings = reduceApp(initialState, { type: 'navigate', screen: 'settings' });
        const draft = reduceApp(settings, { type: 'session', session: snapshot(1, 'ChampSelect') });
        expect(draft.screen).toBe('settings');
        expect(draft.session.phase).toBe('ChampSelect');
    });
    it('conserve la page à la déconnexion et reprend au changement réel', () => {
        let s = reduceApp(initialState, { type: 'session', session: snapshot(1, 'ChampSelect') });
        s = reduceApp(s, { type: 'session', session: snapshot(2, null, false) });
        expect(s.screen).toBe('champ-select');
        expect(s.session.phase).toBeNull();
        s = reduceApp(s, { type: 'session', session: snapshot(3, 'Lobby') });
        expect(s.screen).toBe('dashboard');
    });
    it('revient à la page précédente sans fabriquer une phase', () => {
        let s = reduceApp(initialState, { type: 'navigate', screen: 'champ-select' });
        s = reduceApp(s, { type: 'navigate', screen: 'settings' });
        s = reduceApp(s, { type: 'back' });
        expect(s.screen).toBe('champ-select');
        expect(s.session.connected).toBe(false);
    });
    it('tolère les préférences corrompues et valide chaque champ', () => {
        expect(parsePreferences('{')).toEqual({ theme: 'dark', locale: 'fr', motion: true });
        expect(parsePreferences('{"theme":"light","locale":"en","motion":false}')).toEqual({ theme: 'light', locale: 'en', motion: false });
        expect(parsePreferences('{"theme":12,"locale":"xx"}').locale).toBe('fr');
    });
});

it('conserve le champion consulté et les filtres en quittant puis en revenant à la préparation',()=>{
 let s=reduceApp(initialState,{type:'navigate',screen:'champ-select'});
 s=reduceApp(s,{type:'preparation',patch:{manual:103,roleOverride:'UTILITY',queue:440,rank:'DIAMOND',mode:'equipped'}});
 s=reduceApp(s,{type:'navigate',screen:'settings'});s=reduceApp(s,{type:'back'});
 expect(s.screen).toBe('champ-select');
 expect(s.preparation).toMatchObject({manual:103,roleOverride:'UTILITY',queue:440,rank:'DIAMOND',mode:'equipped'});
});
it('une nouvelle draft reprend le champion local, une simple reconnexion conserve la consultation',()=>{
 let s=reduceApp(initialState,{type:'session',session:snapshot(1,'ChampSelect')});
 s=reduceApp(s,{type:'preparation',patch:{manual:222,roleOverride:'BOTTOM',rank:'GOLD'}});
 s=reduceApp(s,{type:'session',session:snapshot(2,null,false)});
 s=reduceApp(s,{type:'session',session:snapshot(3,'ChampSelect')});
 expect(s.preparation.manual).toBe(222);
 s=reduceApp(s,{type:'session',session:snapshot(4,'Lobby')});
 s=reduceApp(s,{type:'session',session:snapshot(5,'ChampSelect')});
 expect(s.preparation).toMatchObject({manual:null,roleOverride:null,rank:'GOLD'});
});
