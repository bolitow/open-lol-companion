import { screenForPhase, type AppScreen, type LcuSession, type Role } from '@olc/shared';
export type Screen = AppScreen | 'settings' | 'champions' | 'players';
export type Locale = 'fr' | 'en';
export interface Preferences {
    theme: 'dark' | 'light';
    locale: Locale;
    motion: boolean;
}
export function parsePreferences(raw: string | null): Preferences {
    const defaults: Preferences = { theme: 'dark', locale: 'fr', motion: true };
    try {
        const value: unknown = JSON.parse(raw ?? 'null');
        if (!value || typeof value !== 'object')
            return defaults;
        const data = value as Record<string, unknown>;
        return { theme: data.theme === 'light' ? 'light' : 'dark', locale: data.locale === 'en' ? 'en' : 'fr', motion: typeof data.motion === 'boolean' ? data.motion : true };
    }
    catch {
        return defaults;
    }
}
export interface AppState {
    session: LcuSession;
    screen: Screen;
    history: Screen[];
    preparation: PreparationState;
    champions: ChampionsState;
    lastConnectedPhase: LcuSession['phase'];
}
export interface PreparationState {manual:number|null;roleOverride:Role|null;platform:string;queue:number;rank:string;mode:'community'|'equipped'}
export const initialPreparation:PreparationState={manual:null,roleOverride:null,platform:'EUW1',queue:420,rank:'ALL',mode:'community'};
export interface ChampionsState {selected:number|null;query:string;category:string;descending:boolean;scrollTop:number;tab:'abilities'|'builds'|'catalog';role:Role;platform:string;queue:number;rank:string}
export const initialChampions:ChampionsState={selected:null,query:'',category:'ALL',descending:false,scrollTop:0,tab:'abilities',role:'MIDDLE',platform:'EUW1',queue:420,rank:'ALL'};
export const initialState: AppState = { session: { revision: -1, connected: false, phase: null, draft:null, runePage:null, account:null }, screen: 'dashboard', history: [],preparation:initialPreparation,champions:initialChampions,lastConnectedPhase:null };
export type AppAction = {type:'champions';patch:Partial<ChampionsState>} | {
    type:'preparation';patch:Partial<PreparationState>;
} | {
    type: 'session';
    session: LcuSession;
} | {
    type: 'navigate';
    screen: Screen;
} | {
    type: 'back';
};
export function reduceApp(state: AppState, action: AppAction): AppState {
    if(action.type==='champions')return {...state,champions:{...state.champions,...action.patch}};
    if(action.type==='preparation')return {...state,preparation:{...state.preparation,...action.patch}};
    if (action.type === 'back')
        return { ...state, screen: state.history.at(-1) ?? 'dashboard', history: state.history.slice(0, -1) };
    if (action.type === 'navigate')
        return action.screen === state.screen ? state : { ...state, screen: action.screen, history: [...state.history.slice(-19), state.screen] };
    const session = action.session;
    if (session.revision <= state.session.revision)
        return state;
    // Une coupure de connexion ne constitue pas à elle seule une nouvelle draft.
    const next={...state,session,lastConnectedPhase:session.connected&&session.phase?session.phase:state.lastConnectedPhase,
        preparation:session.connected&&session.phase==='ChampSelect'&&state.lastConnectedPhase!=='ChampSelect'?{...state.preparation,manual:null,roleOverride:null}:state.preparation};
    const changed = session.connected && session.phase !== null && (!state.session.connected || session.phase !== state.session.phase);
    const target = session.phase ? screenForPhase(session.phase) : state.screen;
    // Un passage GameStart → InProgress ne ramène pas de force un joueur déjà revenu à l'accueil.
    const previous = state.session.phase ? screenForPhase(state.session.phase) : null;
    if (changed && state.screen !== 'settings' && (target !== previous || !state.session.connected) && target !== state.screen)
        return { ...next, screen: target, history: [...state.history.slice(-19), state.screen] };
    return next;
}
