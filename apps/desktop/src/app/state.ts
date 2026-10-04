import { screenForPhase, type AppScreen, type LcuSession, type Role } from '@olc/shared';
import {rankForQueue} from './buildRanks';
import {draftDefaults} from './buildContext';
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
    defaultRank:string;
    lastDraftId?:string;
    lastAccountKey?:string;
}
export interface PreparationState {manual:number|null;manualCell:number|null;customRole:Role|null;matchup:{championId:number;kind:'chosen'|'assumed'}|null;platformOverride?:boolean;queueOverride?:boolean;roleOverride:Role|null;platform:string;queue:number;rank:string;rankOverride?:string;mode:'community'|'equipped'}
export const initialPreparation:PreparationState={manual:null,manualCell:null,customRole:null,matchup:null,roleOverride:null,platform:'EUW1',queue:420,rank:'EMERALD_PLUS',mode:'community'};
export interface ChampionsState {selected:number|null;query:string;category:string;descending:boolean;scrollTop:number;tab:'abilities'|'builds'|'catalog';role:Role;roleExplicit?:boolean;platformOverride?:boolean;queueOverride?:boolean;platform:string;queue:number;rank:string;rankOverride?:string}
export const initialChampions:ChampionsState={selected:null,query:'',category:'ALL',descending:false,scrollTop:0,tab:'abilities',role:'UNKNOWN',platform:'EUW1',queue:420,rank:'EMERALD_PLUS'};
export const initialState: AppState = { session: { revision: -1, connected: false, phase: null, draft:null, runePage:null, account:null }, screen: 'dashboard', history: [],preparation:initialPreparation,champions:initialChampions,lastConnectedPhase:null,defaultRank:'EMERALD_PLUS' };
export type AppAction = {type:'default-rank';rank:string} | {type:'champions';patch:Partial<ChampionsState>} | {
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
    if(action.type==='default-rank')return action.rank===state.defaultRank?state:{...state,defaultRank:action.rank,
      preparation:{...state.preparation,rank:rankForQueue(state.preparation.queue,state.preparation.rankOverride??action.rank)},
      champions:{...state.champions,rank:rankForQueue(state.champions.queue,state.champions.rankOverride??action.rank)}};
    if(action.type==='champions'){
      const champions={...state.champions,...action.patch,...('rank' in action.patch?{rankOverride:action.patch.rank}:{}),...('platform' in action.patch?{platformOverride:true}:{}),...('queue' in action.patch?{queueOverride:true}:{}),...('role' in action.patch?{roleExplicit:true}:{})};
      champions.rank=rankForQueue(champions.queue,champions.rankOverride??state.defaultRank);
      return {...state,champions};
    }
    if(action.type==='preparation'){
      const preparation={...state.preparation,...action.patch,...('rank' in action.patch?{rankOverride:action.patch.rank}:{}),...('platform' in action.patch?{platformOverride:true}:{}),...('queue' in action.patch?{queueOverride:true}:{})};
      preparation.rank=rankForQueue(preparation.queue,preparation.rankOverride??state.defaultRank);
      return {...state,preparation};
    }
    if (action.type === 'back')
        return { ...state, screen: state.history.at(-1) ?? 'dashboard', history: state.history.slice(0, -1) };
    if (action.type === 'navigate')
        return action.screen === state.screen ? state : { ...state, screen: action.screen, history: [...state.history.slice(-19), state.screen] };
    const session = action.session;
    if (session.revision <= state.session.revision)
        return state;
    // La coupure conserve le contexte ; une autre identité de draft le réinitialise.
    const freshDraft=session.connected&&session.phase==='ChampSelect'&&(state.lastConnectedPhase!=='ChampSelect'||!!(session.draftId&&state.lastDraftId&&session.draftId!==state.lastDraftId));
    const accountKey=session.account?JSON.stringify(session.account):state.lastAccountKey;
    const accountChanged=!!(accountKey&&state.lastAccountKey&&accountKey!==state.lastAccountKey);
    const defaults=draftDefaults(session,state.preparation.customRole);
    const reset=freshDraft||accountChanged;
    const preparation={...state.preparation,...(reset?{manual:null,manualCell:null,roleOverride:null,matchup:null,platformOverride:false,queueOverride:false}:{}),
      ...((reset||!state.preparation.platformOverride)&&defaults.platform?{platform:defaults.platform}:{}),
      ...((reset||!state.preparation.queueOverride)&&defaults.queue!==null?{queue:defaults.queue}:{}),
    };
    if(session.connected&&session.draft&&preparation.matchup&&!session.draft.enemies.some(p=>p.locked&&p.championId===preparation.matchup?.championId))preparation.matchup=null;
    const champions={...state.champions,
      ...(!state.champions.platformOverride&&defaults.platform?{platform:defaults.platform}:{}),
      ...(!state.champions.queueOverride&&defaults.queue!==null?{queue:defaults.queue}:{}),
      ...(!state.champions.roleExplicit&&(defaults.role||reset||(session.connected&&session.draft))?{role:defaults.role??'UNKNOWN' as Role}:{}),
    };
    preparation.rank=rankForQueue(preparation.queue,preparation.rankOverride??state.defaultRank);
    champions.rank=rankForQueue(champions.queue,champions.rankOverride??state.defaultRank);
    const next={...state,session,preparation,champions,lastAccountKey:accountKey,lastDraftId:session.draftId??state.lastDraftId,lastConnectedPhase:session.connected&&session.phase?session.phase:state.lastConnectedPhase};
    const changed = session.connected && session.phase !== null && (!state.session.connected || session.phase !== state.session.phase);
    const target = session.phase ? screenForPhase(session.phase) : state.screen;
    // Un passage GameStart → InProgress ne ramène pas de force un joueur déjà revenu à l'accueil.
    const previous = state.session.phase ? screenForPhase(state.session.phase) : null;
    if (changed && state.screen !== 'settings' && (target !== previous || !state.session.connected) && target !== state.screen)
        return { ...next, screen: target, history: [...state.history.slice(-19), state.screen] };
    return next;
}
