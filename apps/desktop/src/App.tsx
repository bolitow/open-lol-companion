import {SettingsProvider,useSettings} from './app/SettingsContext';
import {SettingsScreen,type SettingsView} from './app/SettingsScreen';
import {usePlayers} from './app/usePlayers';
import {PlayersScreen,HomePlayer,PlayerHistory} from './app/PlayersScreen';
import {parsePlayerQuery,playerKey,type PlayerStore} from './app/playerStore';
import {ChampionsScreen} from './app/ChampionsScreen';
import {ChampionSearch} from './app/ChampionSearch';
import {PreparationContext} from './app/PreparationContext';
import {FlameCompanion} from './companion/FlameCompanion';
import { useEffect, useReducer, useRef, useState } from 'react';
import { screenForPhase } from '@olc/shared';
import { Flame, Icon } from './ui/Icon';
import { copy } from './app/copy';
import { initialState, reduceApp, type Screen } from './app/state';
import { useSession } from './app/useSession';
import { Dashboard, GameScreen, EmptyPanel } from './app/Screens';
export function App({playersStore}:{playersStore?:PlayerStore}={}) {
    return <SettingsProvider><Application playersStore={playersStore}/></SettingsProvider>;
}
function Application({playersStore}:{playersStore?:PlayerStore}) {
    const settings=useSettings(),preferences=settings.state.values;
    const [settingsView,setSettingsView]=useState<SettingsView>({query:'',category:'all'});
    const players=usePlayers(playersStore);
    const homeEntry=players.state.home?players.state.entries[playerKey(players.state.home)]:null;
    const [state, dispatch] = useReducer(reduceApp, initialState);
    const [reduced, setReduced] = useState(() => window.matchMedia('(prefers-reduced-motion: reduce)').matches);
    const connection = useSession(dispatch), t = copy[preferences.locale];
    useEffect(()=>{players.store.syncAccount(state.session.connected,state.session.account)},[players.store,state.session.connected,state.session.account]);
    const menu = useRef<HTMLDialogElement>(null), heading = useRef<HTMLElement>(null), mounted = useRef(false);
    const pages = Object.keys(t.navigation) as Screen[];
    const navigate = (screen: Screen) => { dispatch({ type: 'navigate', screen }); menu.current?.close(); };
    const searchPlayer=(query:string,platform:string)=>{players.store.edit(query,platform);const identity=parsePlayerQuery(query,platform);if(identity)players.store.select(identity);navigate('players');};
    const openChampion = (id:number) => { dispatch({type:'champions',patch:{selected:id,query:'',category:'ALL',scrollTop:0}}); navigate('champions'); };
    useEffect(() => { document.documentElement.dataset.theme = preferences.theme; document.documentElement.lang = preferences.locale; document.documentElement.dataset.motion = preferences.motion && !reduced ? 'full' : 'reduced'; }, [preferences, reduced]);
    useEffect(() => { const media = window.matchMedia('(prefers-reduced-motion: reduce)'); const change = () => setReduced(media.matches); media.addEventListener('change', change); return () => media.removeEventListener('change', change); }, []);
    useEffect(() => { if (mounted.current)
        heading.current?.focus({ preventScroll: true }); mounted.current = true; }, [state.screen]);
    const status = connection.error ? 'error' : !connection.native ? 'browser' : state.session.revision < 0 ? 'checking' : state.session.connected ? 'connected' : 'waiting';
    const phase = state.session.connected && state.session.phase ? t.phases[state.session.phase] : null;
    return <PreparationContext.Provider value={{value:state.preparation,update:patch=>dispatch({type:'preparation',patch})}}><div className="desktop-app"><header className="app-header"><button className="icon-button" aria-label={t.shell.menu} onClick={() => menu.current?.showModal()}><Icon name="menu"/></button><button className="brand" onClick={() => navigate('dashboard')} aria-label={t.navigation.dashboard}><Flame size={27}/><span>{t.shell.brand}</span></button><nav aria-label={t.shell.navigation}>{(['dashboard', 'champions', 'champ-select'] as const).map(page => <button key={page} aria-current={state.screen === page ? 'page' : undefined} onClick={() => navigate(page)}>{t.navigation[page]}</button>)}</nav><ChampionSearch locale={preferences.locale} t={t} screen={state.screen} onPage={navigate} onChampion={openChampion} onPlayer={searchPlayer}/><button className="icon-button" aria-label={t.shell.theme} onClick={() => settings.store.change('theme',preferences.theme==='dark'?'light':'dark')}><Icon name={preferences.theme === 'dark' ? 'sun' : 'moon'}/></button><button className="icon-button" aria-label={t.shell.settings} onClick={() => navigate('settings')}><Icon name="settings"/></button></header>
 <div className="connection-bar" role="status"><span className={`status-light ${status === 'connected' ? 'online' : ''}`}/><strong>{t.connection[status]}</strong><span className="connection-description">{phase ?? t.connection[`${status}Hint`]}</span>{connection.error && <button onClick={connection.retry}>{t.connection.retry}</button>}{state.session.connected && state.session.phase && <button onClick={() => navigate(screenForPhase(state.session.phase!))}>{t.shell.current}<Icon name="arrow" size={14}/></button>}</div>
 <main className="app-main" ref={heading} tabIndex={-1} aria-label={t.navigation[state.screen]}>{state.history.length > 0 && <button className="back-button" onClick={() => dispatch({ type: 'back' })}><Icon name="back" size={14}/>{t.shell.back}</button>}<div key={state.screen} className={`screen-enter screen-${state.screen}`}>{state.screen === 'dashboard' ? <Dashboard profile={<HomePlayer locale={preferences.locale} state={players.state} store={players.store} onBrowse={()=>navigate('players')}/>} history={homeEntry?<PlayerHistory entry={homeEntry} locale={preferences.locale} store={players.store} onChampion={openChampion}/>:<EmptyPanel title={t.home.history} description={t.home.historyHint}/>} t={t} onDraft={() => navigate('champ-select')} companion={<FlameCompanion motion={preferences.motion && !reduced} theme={preferences.theme} locale={preferences.locale}/>}/> : state.screen === 'players' ? <PlayersScreen locale={preferences.locale} state={players.state} store={players.store} onChampion={openChampion}/> : state.screen === 'champions' ? <ChampionsScreen locale={preferences.locale} state={state.champions} update={patch=>dispatch({type:'champions',patch})}/> : state.screen === 'settings' ? <SettingsScreen reduced={reduced} view={settingsView} update={patch=>setSettingsView(value=>({...value,...patch}))}/> : <GameScreen screen={state.screen} session={state.session} t={t} locale={preferences.locale}/>}</div></main>
 <footer className="app-footer"><Flame size={16}/><span>Open LoL Companion</span><small>{t.shell.legal}</small></footer>
 <dialog ref={menu} className="app-menu" aria-labelledby="menu-title"><header><h2 id="menu-title">{t.shell.menu}</h2><button className="icon-button" aria-label={t.shell.close} onClick={() => menu.current?.close()}><Icon name="close"/></button></header>{pages.map(page => <button className="menu-link" key={page} aria-current={state.screen === page ? 'page' : undefined} onClick={() => navigate(page)}>{t.navigation[page]}<Icon name="arrow" size={16}/></button>)}</dialog>
 </div></PreparationContext.Provider>;
}
