import {useCatalogRuntime,CatalogStatus} from './app/useCatalogRuntime';
import {usePatchRefresh} from './app/useBuildPatch';
import {defaultPlayerRank,watchPlayerRank} from './app/buildRanks';
import {FriendsPanel} from './app/friends/FriendsPanel';
import {useFriends} from './app/friends/useFriends';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {useDialogMotion} from './ui/useDialogMotion';
import {listen} from '@tauri-apps/api/event';
import {SettingsProvider,useSettings} from './app/SettingsContext';
import {SettingsScreen,type SettingsView} from './app/SettingsScreen';
import {useCollection,useCollectionView} from './app/collection/useCollection';
import {CollectionScreen} from './app/collection/CollectionScreen';
import {usePlayers} from './app/usePlayers';
import {AutoImportProvider} from './app/AutoImportPanel';
import {PlayersScreen,HomePlayer,PlayerHistory} from './app/PlayersScreen';
import {parsePlayerQuery,playerKey,type PlayerStore} from './app/playerStore';
import {ChampionsScreen} from './app/ChampionsScreen';
import {ChampionSearch} from './app/ChampionSearch';
import {PreparationContext} from './app/PreparationContext';
import {AccountControl,accountStatus} from './app/AccountControl';
import {ApplicationEffects} from './app/ApplicationEffects';
import './app/identity.css';
import {FlameCompanion} from './companion/FlameCompanion';
import { useEffect, useReducer, useRef, useState } from 'react';
import { screenForPhase } from '@olc/shared';
import { Icon } from './ui/Icon';
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
    const friends=useFriends();
    const collection=useCollection();
    const [collectionView,updateCollectionView]=useCollectionView(collection.state);
    const homeEntry=players.state.home?players.state.entries[playerKey(players.state.home)]:null;
    const [state, dispatch] = useReducer(reduceApp, initialState);
    const [reduced, setReduced] = useState(() => window.matchMedia('(prefers-reduced-motion: reduce)').matches);
    const connection = useSession(dispatch), t = copy[preferences.locale];
    const catalog=useCatalogRuntime(JSON.stringify([state.session.connected,state.session.draftId,state.session.account?.platform]));
    usePatchRefresh(JSON.stringify([state.session.connected,state.session.draftId,state.session.account?.platform]));
    useEffect(()=>{players.store.syncAccount(state.session.connected,state.session.account)},[players.store,state.session.connected,state.session.account]);
    const visualRoot=useRef<HTMLDivElement>(null);
    useEffect(()=>{players.store.syncPhase(state.session.phase)},[players.store,state.session.phase]);
    useEffect(()=>{if(connection.native)void invoke('overlay_locale',{locale:preferences.locale}).catch(()=>{});},[connection.native,preferences.locale]);
    useEffect(()=>{
      const profile=homeEntry&&!homeEntry.loading&&!homeEntry.error?homeEntry.profile:null;
      return watchPlayerRank(state.session.connected?state.session.account:null,profile,rank=>dispatch({type:'default-rank',rank}));
    },[state.session.connected,state.session.account,homeEntry?.profile,homeEntry?.loading,homeEntry?.error]);
    const menu = useRef<HTMLDialogElement>(null), heading = useRef<HTMLElement>(null), mounted = useRef(false);
    const menuMotion=useDialogMotion(()=>menu.current?.close());
    const openMenu=()=>{menuMotion.reopen();if(menu.current){menu.current.inert=false;menu.current.showModal()}};
    const pages = Object.keys(t.navigation) as Screen[];
    const navigate = (screen: Screen) => { dispatch({ type: 'navigate', screen }); if(menu.current?.open)menuMotion.close(); };
    useEffect(()=>{
        if(!isTauri())return;
        let disposed=false,unlisten:(()=>void)|undefined;
        void listen('open-settings',()=>{dispatch({type:'navigate',screen:'settings'});if(menu.current?.open)menuMotion.close()}).then(stop=>{if(disposed)stop();else unlisten=stop}).catch(()=>{});
        return()=>{disposed=true;unlisten?.()};
    },[menuMotion.close]);
    const searchPlayer=(query:string,platform:string)=>{players.store.edit(query,platform);const identity=parsePlayerQuery(query,platform);if(identity)players.store.select(identity);navigate('players');};
    const openImportSettings=()=>{setSettingsView({query:'imports',category:'league'});navigate('settings')};
    const openChampion = (id:number) => { dispatch({type:'champions',patch:{selected:id,query:'',category:'ALL',scrollTop:0}}); navigate('champions'); };
    useEffect(() => { document.documentElement.dataset.theme = preferences.theme; document.documentElement.lang = preferences.locale; document.documentElement.dataset.motion = preferences.motion && !reduced ? 'full' : 'reduced'; }, [preferences, reduced]);
    useEffect(() => { const media = window.matchMedia('(prefers-reduced-motion: reduce)'); const change = () => setReduced(media.matches); media.addEventListener('change', change); return () => media.removeEventListener('change', change); }, []);
    useEffect(() => { if (mounted.current)
        heading.current?.focus({ preventScroll: true }); mounted.current = true; }, [state.screen]);
    const status=accountStatus({...connection,revision:state.session.revision,connected:state.session.connected});
    const account=state.session.connected&&state.session.account?state.session.account:players.state.home;
    const accountIcon=account?.profile_icon_id??(account&&homeEntry?.profile&&playerKey(account)===playerKey(homeEntry.profile)?homeEntry.profile.profile_icon_id:null);
    const phase = state.session.connected && state.session.phase ? t.phases[state.session.phase] : null;
    return <PreparationContext.Provider value={{rankReady:!state.session.account||!!(players.state.active&&playerKey(players.state.active)===playerKey(state.session.account)&&homeEntry&&!homeEntry.loading&&state.defaultRank===defaultPlayerRank(state.session.account,homeEntry.error?null:homeEntry.profile)),defaultRank:state.defaultRank,session:state.session,value:state.preparation,update:patch=>dispatch({type:'preparation',patch})}}><AutoImportProvider key={catalog.generation} session={state.session} locale={preferences.locale} native={connection.native}><div ref={visualRoot} className="desktop-app"><header className="app-header"><button className="icon-button shell-back" aria-label={t.shell.back} title={t.shell.back} disabled={state.history.length===0} onClick={()=>dispatch({type:'back'})}><Icon name="back" size={18}/></button><button className="icon-button" aria-label={t.shell.menu} onClick={openMenu}><Icon name="menu"/></button><div className="brand-cluster"><div className="shell-companion"><FlameCompanion motion={preferences.motion && !reduced} theme={preferences.theme} locale={preferences.locale}/></div><button className="brand" onClick={() => navigate('dashboard')} aria-label={t.navigation.dashboard}><span>{t.shell.brand}</span></button></div><nav aria-label={t.shell.navigation}>{(['dashboard', 'champions', 'champ-select'] as const).map(page => <button key={page} aria-current={state.screen === page ? 'page' : undefined} onClick={() => navigate(page)}>{t.navigation[page]}</button>)}</nav><ChampionSearch key={catalog.generation} locale={preferences.locale} t={t} screen={state.screen} onPage={navigate} onChampion={openChampion} onPlayer={searchPlayer}/><button className="icon-button" aria-label={t.shell.theme} onClick={() => settings.store.change('theme',preferences.theme==='dark'?'light':'dark')}><Icon name={preferences.theme === 'dark' ? 'sun' : 'moon'}/></button><button className="icon-button" aria-label={t.shell.settings} onClick={() => navigate('settings')}><Icon name="settings"/></button><AccountControl t={t} accountIsActive={state.session.connected&&!!state.session.account} account={account?{...account,profile_icon_id:accountIcon}:null} status={status} phase={phase} onRetry={connection.retry} onProfile={()=>{if(account){players.store.select(account);navigate('players')}}} onSession={()=>{if(state.session.phase)navigate(screenForPhase(state.session.phase))}}/></header>
 <main key={catalog.generation} className="app-main" ref={heading} tabIndex={-1} aria-label={t.navigation[state.screen]}><div key={state.screen} className={`screen-enter screen-${state.screen}`}>{state.screen === 'dashboard' ? <Dashboard friends={<FriendsPanel locale={preferences.locale} state={friends} onPlayer={request=>{players.store.select(request);navigate('players');}}/>} profile={<HomePlayer locale={preferences.locale} state={players.state} store={players.store} onBrowse={()=>navigate('players')}/>} history={homeEntry?<PlayerHistory entry={homeEntry} locale={preferences.locale} store={players.store} onChampion={openChampion}/>:<EmptyPanel title={t.home.history} description={t.home.historyHint}/>} t={t} onDraft={() => navigate('champ-select')}/> : state.screen === 'players' ? <PlayersScreen locale={preferences.locale} state={players.state} store={players.store} onChampion={openChampion}/> : state.screen === 'champions' ? <ChampionsScreen locale={preferences.locale} state={state.champions} update={patch=>dispatch({type:'champions',patch})}/> : state.screen === 'collection' ? <CollectionScreen locale={preferences.locale} collection={collection} view={collectionView} update={updateCollectionView}/> : state.screen === 'settings' ? <SettingsScreen reduced={reduced} view={settingsView} update={patch=>setSettingsView(value=>({...value,...patch}))}/> : <GameScreen showDraftWinEstimate={preferences.showDraftWinEstimate} screen={state.screen} session={state.session} t={t} locale={preferences.locale} onImportSettings={openImportSettings}/>}</div></main>
 <dialog ref={menu} className="app-menu motion-panel" data-state={menuMotion.state} inert={menuMotion.closing} aria-labelledby="menu-title" onCancel={event=>{event.preventDefault();menuMotion.close()}}><header><h2 id="menu-title">{t.shell.menu}</h2><button className="icon-button" aria-label={t.shell.close} onClick={menuMotion.close}><Icon name="close"/></button></header>{pages.map(page => <button className="menu-link" key={page} aria-current={state.screen === page ? 'page' : undefined} onClick={() => navigate(page)}>{t.navigation[page]}<Icon name="arrow" size={16}/></button>)}<CatalogStatus locale={preferences.locale}/><p className="menu-legal">{t.shell.legal}</p></dialog>
 <ApplicationEffects root={visualRoot} scene={{screen:state.screen,phase:state.session.phase,connected:state.session.connected}} theme={preferences.theme} enabled={preferences.motion&&!reduced}/>
 </div></AutoImportProvider></PreparationContext.Provider>;
}
