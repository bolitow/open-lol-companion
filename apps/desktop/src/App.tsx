import {ChampionsScreen} from './app/ChampionsScreen';
import {ChampionSearch} from './app/ChampionSearch';
import {PreparationContext} from './app/PreparationContext';
import {FlameCompanion} from './companion/FlameCompanion';
import { useEffect, useReducer, useRef, useState } from 'react';
import { screenForPhase } from '@olc/shared';
import { Flame, Icon } from './ui/Icon';
import { copy } from './app/copy';
import { initialState, parsePreferences, reduceApp, type Screen } from './app/state';
import { useSession } from './app/useSession';
import { Dashboard, GameScreen } from './app/Screens';
const STORAGE_KEY = 'olc.app.preferences';
function readPreferences() { try {
    return parsePreferences(localStorage.getItem(STORAGE_KEY));
}
catch {
    return parsePreferences(null);
} }
export function App() {
    const [state, dispatch] = useReducer(reduceApp, initialState), [preferences, setPreferences] = useState(readPreferences);
    const [storageFailed, setStorageFailed] = useState(false);
    const [reduced, setReduced] = useState(() => window.matchMedia('(prefers-reduced-motion: reduce)').matches);
    const connection = useSession(dispatch), t = copy[preferences.locale];
    const menu = useRef<HTMLDialogElement>(null), heading = useRef<HTMLElement>(null), mounted = useRef(false);
    const pages = Object.keys(t.navigation) as Screen[];
    const navigate = (screen: Screen) => { dispatch({ type: 'navigate', screen }); menu.current?.close(); };
    const openChampion = (id:number) => { dispatch({type:'champions',patch:{selected:id,query:'',category:'ALL',scrollTop:0}}); navigate('champions'); };
    useEffect(() => { document.documentElement.dataset.theme = preferences.theme; document.documentElement.lang = preferences.locale; document.documentElement.dataset.motion = preferences.motion && !reduced ? 'full' : 'reduced'; try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences));
        setStorageFailed(false);
    }
    catch {
        setStorageFailed(true);
    } }, [preferences, reduced]);
    useEffect(() => { const media = window.matchMedia('(prefers-reduced-motion: reduce)'); const change = () => setReduced(media.matches); media.addEventListener('change', change); return () => media.removeEventListener('change', change); }, []);
    useEffect(() => { if (mounted.current)
        heading.current?.focus({ preventScroll: true }); mounted.current = true; }, [state.screen]);
    const status = connection.error ? 'error' : !connection.native ? 'browser' : state.session.revision < 0 ? 'checking' : state.session.connected ? 'connected' : 'waiting';
    const phase = state.session.connected && state.session.phase ? t.phases[state.session.phase] : null;
    return <PreparationContext.Provider value={{value:state.preparation,update:patch=>dispatch({type:'preparation',patch})}}><div className="desktop-app"><header className="app-header"><button className="icon-button" aria-label={t.shell.menu} onClick={() => menu.current?.showModal()}><Icon name="menu"/></button><button className="brand" onClick={() => navigate('dashboard')} aria-label={t.navigation.dashboard}><Flame size={27}/><span>{t.shell.brand}</span></button><nav aria-label={t.shell.navigation}>{(['dashboard', 'champions', 'champ-select'] as const).map(page => <button key={page} aria-current={state.screen === page ? 'page' : undefined} onClick={() => navigate(page)}>{t.navigation[page]}</button>)}</nav><ChampionSearch locale={preferences.locale} t={t} screen={state.screen} onPage={navigate} onChampion={openChampion}/><button className="icon-button" aria-label={t.shell.theme} onClick={() => setPreferences(p => ({ ...p, theme: p.theme === 'dark' ? 'light' : 'dark' }))}><Icon name={preferences.theme === 'dark' ? 'sun' : 'moon'}/></button><button className="icon-button" aria-label={t.shell.settings} onClick={() => navigate('settings')}><Icon name="settings"/></button></header>
 <div className="connection-bar" role="status"><span className={`status-light ${status === 'connected' ? 'online' : ''}`}/><strong>{t.connection[status]}</strong><span className="connection-description">{phase ?? t.connection[`${status}Hint`]}</span>{connection.error && <button onClick={connection.retry}>{t.connection.retry}</button>}{state.session.connected && state.session.phase && <button onClick={() => navigate(screenForPhase(state.session.phase!))}>{t.shell.current}<Icon name="arrow" size={14}/></button>}</div>
 <main className="app-main" ref={heading} tabIndex={-1} aria-label={t.navigation[state.screen]}>{state.history.length > 0 && <button className="back-button" onClick={() => dispatch({ type: 'back' })}><Icon name="back" size={14}/>{t.shell.back}</button>}<div key={state.screen} className={`screen-enter screen-${state.screen}`}>{state.screen === 'dashboard' ? <Dashboard t={t} onDraft={() => navigate('champ-select')} companion={<FlameCompanion motion={preferences.motion && !reduced} theme={preferences.theme} locale={preferences.locale}/>}/> : state.screen === 'champions' ? <ChampionsScreen locale={preferences.locale} state={state.champions} update={patch=>dispatch({type:'champions',patch})}/> : state.screen === 'settings' ? <><div className="screen-heading"><span className="eyebrow">{t.settings.general}</span><h1>{t.settings.title}</h1><p>{t.settings.hint}</p></div><section className="surface settings-panel"><h2>{t.settings.appearance}</h2><div className="theme-options">{(['dark', 'light'] as const).map(theme => <button key={theme} className={`theme-option ${theme}`} aria-pressed={preferences.theme === theme} onClick={() => setPreferences(p => ({ ...p, theme }))}><span className="theme-sample"><i /><i /><i /></span>{t.settings[theme]}{preferences.theme === theme && <Icon name="check" size={18}/>}</button>)}</div><label className="setting-row" htmlFor="app-language"><span>{t.settings.language}</span><select id="app-language" value={preferences.locale} onChange={e => setPreferences(p => ({ ...p, locale: e.target.value === 'en' ? 'en' : 'fr' }))}><option value="fr">Français</option><option value="en">English</option></select></label><label className="setting-row"><span>{t.settings.motion}<small>{t.settings.motionHint}</small></span><input type="checkbox" checked={preferences.motion} onChange={e => setPreferences(p => ({ ...p, motion: e.target.checked }))}/></label>{reduced && <p>{t.settings.reduced}</p>}<p role="status">{storageFailed ? t.settings.storage : t.settings.saved}</p></section></> : <GameScreen screen={state.screen} session={state.session} t={t} locale={preferences.locale}/>}</div></main>
 <footer className="app-footer"><Flame size={16}/><span>Open LoL Companion</span><small>{t.shell.legal}</small></footer>
 <dialog ref={menu} className="app-menu" aria-labelledby="menu-title"><header><h2 id="menu-title">{t.shell.menu}</h2><button className="icon-button" aria-label={t.shell.close} onClick={() => menu.current?.close()}><Icon name="close"/></button></header>{pages.map(page => <button className="menu-link" key={page} aria-current={state.screen === page ? 'page' : undefined} onClick={() => navigate(page)}>{t.navigation[page]}<Icon name="arrow" size={16}/></button>)}</dialog>
 </div></PreparationContext.Provider>;
}
