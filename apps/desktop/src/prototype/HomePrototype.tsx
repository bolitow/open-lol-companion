import {FlameCompanion} from "../companion/FlameCompanion";
import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import {PageFlame} from "./PageFlame";
import {DraftPage} from "./DraftPage";
import {LayoutsPage} from "./LayoutsPage";
import {draftCopy} from "./draftCopy";
import {initialDraft,draftReducer} from "./draft";
import { assets, accounts, friends, matches, type Match } from "./fixtures";
import { en, fr, type Copy } from "./i18n";
import { Flame, Icon } from "./Icon";
import { AmbientEmbers } from "./AmbientEmbers";
import { BurnScars } from "./BurnScars";
import { BurnReveal } from "./BurnReveal";
import { pageFromHash, filterMatches, initialSession, parsePreferences, searchCatalog, sessionReducer, type HistoryState, type Preferences, type QueueFilter } from "./model";

const STORAGE_KEY = "olc-home-prototype-preferences-v1";
type Modal = "settings" | "menu" | "scenarios" | "about" | "game" | "clips" | "collection" | "progress" | "player" | "champion" | "friend" | null;
type Entry = { id: string; label: string; keywords: string; kind: "champion" | "player" | "feature"; image?: string; destination: Modal };
function readPreferences() { try { return parsePreferences(localStorage.getItem(STORAGE_KEY)); } catch { return parsePreferences(null); } }
const duration = (m: Match) => `${m.minutes}:${String(m.seconds).padStart(2, "0")}`;

function Portrait({ name, className = "" }: { name: string; className?: string }) {
  return <img className={`portrait ${className}`} src={`${assets}${name}.png`} alt={name === "LeeSin" ? "Lee Sin" : name} width="48" height="48" />;
}
function Items({ items, label }: { items: number[]; label: string }) {
  return <div className="items" role="img" aria-label={label}>{Array.from({ length: 6 }, (_, i) => <span key={i}>{items[i] && <img src={`${assets}${items[i]}.png`} alt="" width="26" height="26" />}</span>)}</div>;
}

export function HomePrototype() {
  const [preferences, setPreferences] = useState<Preferences>(readPreferences);
  const [systemReduced, setSystemReduced] = useState(() => matchMedia("(prefers-reduced-motion: reduce)").matches);
  const motion = preferences.motion && !systemReduced;
  const [session, dispatch] = useReducer(sessionReducer, initialSession);
  const [page,setPage]=useState(()=>pageFromHash(location.hash));
  const [draft,draftDispatch]=useReducer(draftReducer,initialDraft);
  const [playing,setPlaying]=useState(false);
  const [entryPending,setEntryPending]=useState(false);
  const [arrival,setArrival]=useState(false);
  const [pageFlaming,setPageFlaming]=useState(false);
  const transitionCue=useRef("");
  const [flameCue,setFlameCue]=useState("");
  const pageHeading=useRef<HTMLDivElement>(null);
  const [modal, setModal] = useState<Modal>(null);
  const [returnToChampion, setReturnToChampion] = useState(false);
  const [previewName, setPreviewName] = useState("Ahri");
  const [previewChampion, setPreviewChampion] = useState("Ahri");
  const [queue, setQueue] = useState<QueueFilter>("all");
  const [historyState, setHistoryState] = useState<HistoryState>("ready");
  const [updated, setUpdated] = useState(0);
  const [intro, setIntro] = useState(()=>pageFromHash(location.hash)==="home");
  const [introKey, setIntroKey] = useState(0);
  const [query, setQuery] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchKind, setSearchKind] = useState<"all" | Entry["kind"]>("all");
  const [activeResult, setActiveResult] = useState(0);
  const [capture, setCapture] = useState(false);
  const [pins, setPins] = useState<string[]>([]);
  const [goal, setGoal] = useState(0);
  const [notice, setNotice] = useState("");
  const [storageUnavailable, setStorageUnavailable] = useState(false);
  const goalList = useRef<HTMLDivElement>(null);
  const prototypeRoot = useRef<HTMLDivElement>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const refreshingRows = useRef(false);
  const searchResults = useRef<HTMLDivElement>(null);
  const refreshTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const t: Copy = preferences.locale === "fr" ? fr : en;
  const dt=draftCopy[preferences.locale];
  const locale = preferences.locale === "fr" ? "fr-FR" : "en-US";
  const number = (value: number, digits = 0) => new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(value);
  const account = accounts[session.account];
  const accountMatches = filterMatches(matches, session.account, "all");
  const visibleMatches = filterMatches(matches, session.account, queue);
  const selectedMatch = accountMatches.find(m => m.id === session.selectedMatch);
  const wins = accountMatches.filter(m => m.win).length;
  const winRate = new Intl.NumberFormat(locale, { style: "percent", maximumFractionDigits: 0 }).format(wins / accountMatches.length);
  useEffect(() => {
    const list = goalList.current;
    const chosen = list?.querySelector<HTMLElement>(".chosen");
    if (!list || !chosen) return;
    const visible = list.getBoundingClientRect(), item = chosen.getBoundingClientRect();
    if (item.bottom > visible.bottom) list.scrollTop += item.bottom - visible.bottom;
    if (item.top < visible.top) list.scrollTop -= visible.top - item.top;
  }, [goal]);
  const finishIntro = useCallback(() => setIntro(false), []);

  useEffect(() => {
    const media = matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setSystemReduced(media.matches);
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme = preferences.theme;
    document.documentElement.lang = preferences.locale;
    document.title = `Open LoL · ${t.home}`;
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences)); setStorageUnavailable(false); }
    catch { setStorageUnavailable(true); }
  }, [preferences, t.home]);
  useEffect(() => { if (!motion) setIntro(false); }, [motion]);
  useEffect(() => {
    if (modal || selectedMatch) { if (!dialog.current?.open) dialog.current?.showModal(); }
    else dialog.current?.close();
  }, [modal, selectedMatch]);
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k" && !dialog.current?.open) {
        event.preventDefault(); searchInput.current?.focus(); setSearchOpen(true);
      }
      if (event.key === "Escape") { setSearchOpen(false); setIntro(false); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);
  useEffect(() => { if (!notice) return; const id = setTimeout(() => setNotice(""), 3500); return () => clearTimeout(id); }, [notice]);
  useEffect(() => () => { if (refreshTimer.current) clearTimeout(refreshTimer.current); }, []);

  useEffect(()=>{
    if(!entryPending)return;
    const timer=setTimeout(()=>{
      setModal(null);dispatch({type:"match",id:null});setSearchOpen(false);setIntro(false);
      draftDispatch({type:"reset"});goPage("draft");setPlaying(true);setEntryPending(false);setArrival(true);
    },1200);
    return()=>clearTimeout(timer);
  },[entryPending]);
  useEffect(()=>{
    if(!playing||draft.stage>=6||draft.stage===3&&!draft.prepick)return;
    const timer=setTimeout(()=>draftDispatch({type:"next"}),draft.stage===5?3000:4200);
    return()=>clearTimeout(timer);
  },[playing,draft.stage,draft.prepick]);
  useEffect(()=>{if(draft.stage===6)setPlaying(false);},[draft.stage]);
  useEffect(()=>{
    if(!arrival)return;
    const timer=setTimeout(()=>setArrival(false),900);return()=>clearTimeout(timer);
  },[arrival]);
  useEffect(()=>{document.title=`Open LoL · ${page==="home"?t.home:page==="draft"?t.game:dt.layout}`;},[page,t,dt]);
  useEffect(()=>{if(page!=="home")pageHeading.current?.focus({preventScroll:true});},[page]);
  useEffect(()=>{
    const cue=`${page}:${page==="draft"?Math.min(6,Math.max(4,draft.stage)):0}`;
    if(transitionCue.current&&transitionCue.current!==cue)setFlameCue(cue);
    transitionCue.current=cue;
  },[page,draft.stage]);
  const goPage=(next:typeof page)=>{if(page!==next)history.pushState(null,"",`#${next}`);setModal(null);dispatch({type:"match",id:null});setSearchOpen(false);setIntro(false);setPage(next);};
  useEffect(()=>{
    const restore=()=>{setModal(null);dispatch({type:"match",id:null});setIntro(false);setSearchOpen(false);setPage(pageFromHash(location.hash));};
    window.addEventListener("popstate",restore);return()=>window.removeEventListener("popstate",restore);
  },[]);
  const startDraftDemo=()=>{setEntryPending(true);setModal(null);};

  const catalog: Entry[] = useMemo(() => [
    ...["Ahri", "Jinx", "Lux", "Orianna"].map(name => ({ id: `champion-${name}`, label: name, keywords: "champion mid adc mage", kind: "champion" as const, image: name, destination: "champion" as const })),
    ...Object.values(accounts).map(a => ({ id: `player-${a.name}`, label: `${a.name}#${a.tag}`, keywords: "joueur player compte account", kind: "player" as const, image: a.champion, destination: "player" as const })),
    { id: "clips", label: t.clips, keywords: "enregistrement capture recording video", kind: "feature", destination: "clips" },
    { id: "settings", label: t.settings, keywords: "options paramètres theme langue language", kind: "feature", destination: "settings" },
    { id: "progress", label: t.progress, keywords: "farm améliorer conseils improvement vision", kind: "feature", destination: "progress" },
  ], [t]);
  const results = searchCatalog(catalog, query).filter(e => searchKind === "all" || e.kind === searchKind);
  useEffect(() => {
    if (!searchOpen) return;
    const list = searchResults.current;
    const option = list?.querySelector<HTMLElement>('[aria-selected="true"]');
    if (!list || !option) return;
    const itemRect = option.getBoundingClientRect();
    const listRect = list.getBoundingClientRect();
    if (itemRect.bottom > listRect.bottom) list.scrollTop += itemRect.bottom - listRect.bottom;
    else if (itemRect.top < listRect.top) list.scrollTop -= listRect.top - itemRect.top;
  }, [activeResult, searchOpen, query, searchKind]);
  const chooseResult = (entry: Entry) => {
    setReturnToChampion(false); setSearchOpen(false); setPreviewName(entry.label); setPreviewChampion(entry.image ?? "Ahri"); setModal(entry.destination);
  };
  const closeModal = () => { setReturnToChampion(false); setModal(null); dispatch({ type: "match", id: null }); };
  const showModal = (next: Modal) => { if(next==="game"){goPage("draft");return;} setReturnToChampion(false); setSearchOpen(false); setModal(next); };
  const refresh = () => {
    if (historyState === "loading") return;
    refreshingRows.current = historyState === "ready";
    if (refreshTimer.current) clearTimeout(refreshTimer.current);
    setHistoryState("loading");
    refreshTimer.current = setTimeout(() => { refreshingRows.current = false; setHistoryState("ready"); setUpdated(x => x + 1); setNotice(t.refreshed); }, 700);
  };
  const setScenario = (state: HistoryState) => {
    refreshingRows.current = false;
    if (refreshTimer.current) clearTimeout(refreshTimer.current);
    setHistoryState(state);
  };
  const toggleCapture = () => { setCapture(!capture); setNotice(`${capture ? t.captureOff : t.captureOn}. ${t.noCapture}`); };
  const startIntro = () => { closeModal(); setIntroKey(x => x + 1); setIntro(true); };
  const goals = [
    { title: t.farm, value: number(6.8, 1), unit: "CS/min", target: "7", samples: [5.9, 6.2, 6.1, 6.5, 6.8], chart: "10,68 65,53 120,58 175,38 230,23", targetY: 13, lastY: 23, icon: "sword" as const, tip: t.farmTip },
    { title: t.vision, value: number(1.1, 1), unit: "/min", target: number(1.3, 1), samples: [.7, .8, .8, 1, 1.1], chart: "10,67 65,58 120,58 175,40 230,31", targetY: 13, lastY: 31, icon: "eye" as const, tip: t.visionTip },
    { title: t.deaths, value: "4", unit: "/game", target: "3", samples: [6, 5, 7, 5, 4], chart: "10,26 65,39 120,13 175,39 230,52", targetY: 65, lastY: 52, icon: "shield" as const, tip: t.deathsTip },
  ];
  const modalTitle = selectedMatch ? t.matchDetails : modal === "settings" ? t.preferences : modal === "scenarios" ? t.simulation : modal === "about" ? t.aboutTitle : modal === "menu" ? t.menu : modal === "champion" || modal === "player" || modal === "friend" ? previewName : modal ? t[modal] : "";

  return <div ref={prototypeRoot} className="prototype" data-motion={motion ? "full" : "reduced"}>
    <header className="topbar">
      <button className="icon-button" aria-label={t.menu} title={t.menu} onClick={() => showModal("menu")}><Icon name="menu" /></button>
      <button className="brand" aria-label="Open LoL" onClick={()=>goPage("home")}><Flame /><span>OPEN LOL</span></button>
      <nav aria-label={t.menu}><button className={page==="home"?"nav-active":""} onClick={()=>goPage("home")}>{t.home}</button><button className={page==="draft"?"nav-active":""} onClick={() => showModal("game")}>{t.game}</button><button onClick={() => { setSearchKind("champion"); setQuery(""); setActiveResult(0); searchInput.current?.focus(); setSearchOpen(true); }}>{t.champions}</button></nav>
      <div className="global-search" onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) setSearchOpen(false); }}>
        <div className={`search-field ${searchOpen ? "focused" : ""}`}><Icon name="search" />
          <input ref={searchInput} type="search" aria-label={t.searchLabel} role="combobox" aria-expanded={searchOpen} aria-controls="search-results" aria-autocomplete="list" aria-activedescendant={searchOpen && results[activeResult] ? `result-${results[activeResult]?.id}` : undefined} placeholder={t.search} value={query}
            onFocus={() => setSearchOpen(true)} onChange={event => { setQuery(event.target.value); setActiveResult(0); setSearchOpen(true); }}
            onKeyDown={event => {
              if (event.key === "ArrowDown" || event.key === "ArrowUp") { event.preventDefault(); setSearchOpen(true); setActiveResult(i => Math.max(0, Math.min(results.length - 1, i + (event.key === "ArrowDown" ? 1 : -1)))); }
              if (event.key === "Enter" && searchOpen && results[activeResult]) { event.preventDefault(); chooseResult(results[activeResult]); }
              if (event.key === "Escape") { event.preventDefault(); setSearchOpen(false); }
            }} />
          {query ? <button className="icon-button small" aria-label={t.clear} onClick={() => { setQuery(""); searchInput.current?.focus(); }}><Icon name="close" size={16} /></button> : <kbd>Ctrl / ⌘ K</kbd>}
        </div>
        {searchOpen && <section className="search-popover">
          <div className="search-filters">{(["all", "champion", "player", "feature"] as const).map(kind => <button key={kind} className={searchKind === kind ? "selected" : ""} aria-pressed={searchKind === kind} onClick={() => { setSearchKind(kind); setActiveResult(0); }}>{t[kind]}</button>)}</div>
          <p className="small-muted">{t.searchHint}</p>
          <div ref={searchResults} role="listbox" id="search-results" aria-label={t.searchLabel} className="search-results">
            {results.length === 0 && <p className="empty-search">{t.noResults}</p>}
            {results.map((entry, i) => <button role="option" aria-selected={i === activeResult} id={`result-${entry.id}`} className={`search-result ${i === activeResult ? "highlighted" : ""}`} key={entry.id} onMouseEnter={() => setActiveResult(i)} onClick={() => chooseResult(entry)}>{entry.image ? <Portrait name={entry.image} /> : <span className="result-symbol"><Icon name={entry.id === "clips" ? "clip" : entry.id === "settings" ? "settings" : "chart"} /></span>}<span><strong>{entry.label}</strong><small>{t[entry.kind]}</small></span><Icon name="arrow" size={16} /></button>)}
          </div><div className="search-help"><span>↑ ↓ {t.keyboard}</span><span>↵ {t.open}</span><span>Esc</span></div>
        </section>}
      </div>
      <div className="header-actions"><button className={`icon-button ${capture ? "accent" : ""}`} aria-label={capture ? t.captureOn : t.captureOff} title={capture ? t.captureOn : t.captureOff} aria-pressed={capture} onClick={toggleCapture}><Icon name="clip" /></button><button className="icon-button theme-toggle" aria-label={t.switchTheme} title={t.switchTheme} onClick={() => setPreferences(p => ({ ...p, theme: p.theme === "dark" ? "light" : "dark" }))}><Icon name={preferences.theme === "dark" ? "sun" : "moon"} /></button><button className="icon-button" aria-label={t.settings} title={t.settings} onClick={() => showModal("settings")}><Icon name="settings" /></button></div>
    </header>

    <div className={`page-stage ${arrival?"auto-arrival":""}`} ref={pageHeading} tabIndex={-1}>
    <div className={page==="home"?"page-visible":"page-hidden"}>
    <main className="home-content">
      <div className="page-heading"><div className="page-context"><h1>{t.home}</h1><span className="context-divider" /><span className="demo-tag">{t.demo}</span></div><div className="prototype-companion"><FlameCompanion motion={motion} theme={preferences.theme} locale={preferences.locale} visible={page==="home" && !modal && !selectedMatch}/></div><div className="home-demo-actions"><button className="text-button" onClick={()=>goPage("layouts")}>{dt.layout}</button><button className="outline-button" disabled={entryPending} onClick={startDraftDemo}>{dt.simulate}</button><button className="lab-button" onClick={() => showModal("scenarios")}><Icon name="settings" size={16} />{t.scenarios}</button></div></div>
      <div className="dashboard-grid">
        <div className="main-column">
          <div className="summary-grid">
            <section className="panel profile-panel shoulder" aria-label={t.yourProfile}>
              <div className="profile-top"><Portrait name={account.champion} className="profile-portrait" /><div><span className="caption">{t.yourProfile}</span><h2>{account.name}<span className="tag">#{account.tag}</span></h2><p className="small-muted">{t.mainRole}</p></div><button className="icon-button" aria-label={t.viewProfile} onClick={() => { setPreviewName(account.name); setPreviewChampion(account.champion); showModal("player"); }}><Icon name="arrow" size={18} /></button></div>
              <div className="rank-line"><span className="rank-emblem"><Icon name="shield" size={28} /><i /></span><div><strong>{t[account.rank]}</strong><small>{t.ranked}</small></div><div className="lp"><strong>{account.lp}</strong><span>LP</span></div></div>
              <div className="profile-bottom"><div><strong>{winRate}</strong><small>{t.winRate} · {accountMatches.length} {t.gamesPlayed}</small></div><div className="recent-form"><span className="caption">{t.recentForm}</span><div>{accountMatches.slice(0, 5).map(m => <span key={m.id} className={m.win ? "won" : "lost"} title={m.win ? t.victory : t.defeat}><Icon name={m.win ? "check" : "close"} size={11} /></span>)}</div></div></div>
            </section>
            <section className={`panel session-panel shoulder ${session.connected ? "" : "disconnected"}`}>
              <div className="session-art" aria-hidden="true" /><div className="session-seam" aria-hidden="true" /><BurnScars />
              <div className="session-content"><div className="session-label"><span className={`status-dot ${session.connected ? "" : "off"}`} />{session.connected ? t.current : t.offline}</div><h2>{session.connected ? t.ready : t.offlineTitle}</h2><p>{session.connected ? t.sessionCopy : t.offlineCopy}</p><button className="primary-button" disabled={!session.connected} onClick={() => showModal("game")}>{t.enterGame}<Icon name="arrow" size={18} /></button>{!session.connected && <small>{t.retained}</small>}</div>
              <span className="champion-signature"><span>{t.lastPlayed}</span><strong>Ahri</strong></span>
            </section>
          </div>

          <section className="panel history-panel" aria-labelledby="history-title">
            {!!updated && motion && <span key={updated} className="update-edge just-updated" aria-hidden="true" />}
            <div className="section-heading"><div><h2 id="history-title">{t.history}</h2><p>{t.historyHint}</p></div><button className="icon-button" aria-label={t.refresh} title={t.refresh} onClick={refresh} aria-disabled={historyState === "loading"}><Icon name="replay" size={18} /></button></div>
            <div className="history-toolbar"><div className="segmented">{(["all", "ranked", "aram"] as const).map(q => <button key={q} aria-pressed={queue === q} className={q === queue ? "selected" : ""} onClick={() => setQueue(q)}>{t[q]}</button>)}</div><span className="small-muted">{visibleMatches.length} {t.gamesPlayed}</span></div>
            <div className="history-scroll" aria-label={t.history} tabIndex={0} aria-busy={historyState === "loading"}>
              {historyState === "loading" && !refreshingRows.current ? <div className="loading-state" role="status"><span className="sr-only">{t.loading}</span>{[0, 1, 2, 3, 4].map(i => <div className="skeleton-row" key={i}><i /><span /><span /></div>)}</div> : historyState === "empty" || visibleMatches.length === 0 ? <div className="state-message"><Icon name="chart" size={36} /><h3>{t.noGames}</h3><p>{t.noGamesHint}</p><button className="text-button" onClick={() => { setScenario("ready"); setQueue("all"); }}>{t.reset}<Icon name="arrow" size={16} /></button></div> : historyState === "error" ? <div className="state-message"><Icon name="info" size={32} /><h3>{t.error}</h3><p>{t.errorHint}</p><button className="primary-button" onClick={refresh}>{t.retry}<Icon name="replay" size={16} /></button></div> : visibleMatches.map((m, index) => <button className={`match-row ${m.win ? "win" : "loss"}`} key={m.id} onClick={() => dispatch({ type: "match", id: m.id })} aria-label={`${t.showDetails} · ${m.champion} · ${m.win ? t.victory : t.defeat} · ${m.kda.join(" / ")}`}>
                <span className="result-marker" /><Portrait name={m.champion} /><span className="match-result"><strong>{m.win ? t.victory : t.defeat}</strong><small>{m.champion} · {t[m.queue]}</small></span><span className="match-kda"><strong>{m.kda.join(" / ")}</strong><small>{m.cs} CS · {number((m.kda[0] + m.kda[2]) / Math.max(1, m.kda[1]), 1)} KDA</small></span><Items items={m.items} label={t.items} /><span className="match-time"><strong>{duration(m)}</strong><small>{index < 2 ? t.today : index < 5 ? t.yesterday : t.daysAgo}</small></span><span className={`match-lp ${m.lp > 0 ? "positive" : ""}`}>{m.lp ? `${m.lp > 0 ? "+" : ""}${m.lp}` : "—"}<small>{m.lp ? "LP" : ""}</small></span><Icon name="chevron" size={15} />
              </button>)}
            </div><div className="history-footer"><span className="status-dot off" />{t.lastSync}</div>
          </section>
        </div>

        <aside className="side-column">
          <section className="panel friends-panel" aria-labelledby="friends-title"><div className="section-heading"><div><h2 id="friends-title">{t.friends}<span className="live-badge">5</span></h2><p>{t.friendsHint}</p></div><Icon name="users" size={20} /></div>
            <div className="friends-scroll" tabIndex={0} aria-label={t.friends}>{[...friends].sort((a, b) => Number(pins.includes(b.name)) - Number(pins.includes(a.name))).map(friend => <div className="friend-row" key={friend.name}><button className="friend-main" onClick={() => { setPreviewName(friend.name); setPreviewChampion(friend.champion); showModal("friend"); }}><span className="friend-avatar"><Portrait name={friend.champion} /><i className={friend.time ? "online-dot" : "draft-dot"} /></span><span><strong>{friend.name}{friend.followed && <small className="followed-label">{t.following}</small>}</strong><small>{friend.time ? `${friend.champion === "LeeSin" ? "Lee Sin" : friend.champion} · ${friend.time} min` : t.inDraft}</small></span><span className="live-mini">{friend.time ? t.inGame : ""}</span></button><button className={`pin-button ${pins.includes(friend.name) ? "pinned" : ""}`} aria-label={`${pins.includes(friend.name) ? t.unfollow : t.follow} ${friend.name}`} aria-pressed={pins.includes(friend.name)} onClick={() => setPins(old => old.includes(friend.name) ? old.filter(name => name !== friend.name) : [...old, friend.name])}><Icon name="pin" size={15} /></button></div>)}</div>
          </section>
          <section className="panel goals-panel" aria-labelledby="goals-title"><div className="section-heading"><div><h2 id="goals-title">{t.working}</h2><p>{t.workingHint}</p></div><span className="goal-symbol"><Flame size={24} /></span></div>
            <div ref={goalList} className="goals-scroll" tabIndex={0} aria-label={t.working}>{goals.map((g, i) => <button className={`goal-row ${goal === i ? "chosen" : ""}`} key={g.title} aria-pressed={goal === i} aria-label={`${t.focus} : ${g.title}`} aria-describedby={`goal-description-${i}`} onClick={() => { setGoal(i); setNotice(`${t.focused} : ${g.title}`); }}><span id={`goal-description-${i}`} className="sr-only">{t.lastFive} : {g.samples.map(value => number(value, 1)).join(", ")} {g.unit === "/game" ? `/ ${t.game.toLowerCase()}` : g.unit}. {t.goal} : {g.target}. {t.demo}.</span><span className="goal-icon"><Icon name={g.icon} size={19} /></span><span className="goal-body">{goal === i && <span className="focus-caption">{t.activeFocus}</span>}<span className="goal-title">{g.title}{goal === i && <Icon name="check" size={13} />}</span><span className="goal-values"><strong>{g.value}<small>{g.unit === "/game" ? ` / ${t.game.toLowerCase()}` : ` ${g.unit}`}</small></strong><small>{t.goal} {g.target}</small></span>{goal === i && <span className="goal-trend"><svg viewBox="0 0 240 84" preserveAspectRatio="none" aria-hidden="true"><line className="trend-target" x1="0" x2="240" y1={g.targetY} y2={g.targetY} /><path className="trend-area" d={`M${g.chart.replaceAll(" ", " L")} L230,84 L10,84 Z`} /><polyline className="trend-line" points={g.chart} />{g.chart.split(" ").map((point,index) => { const [x,y] = point.split(","); return <circle key={point+index} cx={x} cy={y} r={index===4 ? 4 : 2.5} />; })}<circle className="trend-halo" cx="230" cy={g.lastY} r="8" /></svg><span className="trend-caption"><span>{t.lastFive}</span><span><i />{t.goal}</span></span></span>}</span></button>)}</div>
            <button className="section-link" onClick={() => showModal("progress")}>{t.learn}<Icon name="arrow" size={16} /></button>
          </section>
        </aside>
      </div>
      <footer className="statusbar"><span className="connection-state"><span className={`status-dot ${session.connected ? "" : "off"}`} />{session.connected ? t.online : `${t.offline} · ${t.retained}`}</span><span className="status-account">{account.name}#{account.tag} <span>EUW</span></span><div><button onClick={startIntro} disabled={!motion}><Icon name="replay" size={14} />{t.replay}</button><button onClick={() => showModal("about")}>{t.about}<Icon name="info" size={14} /></button></div></footer>
    </main>
    </div>
    {page==="draft"&&<div className="page-visible"><DraftPage state={draft} locale={preferences.locale} dispatch={draftDispatch} onHome={()=>goPage("home")} playing={playing} onPlaying={setPlaying}/></div>}
    {page==="layouts"&&<div className="page-visible"><LayoutsPage locale={preferences.locale} onBack={()=>goPage("home")}/></div>}
    </div>
    {entryPending&&<div className="entry-notice" role="status"><Flame size={22}/><span>{dt.autoEntry}</span><button onClick={()=>setEntryPending(false)}>{dt.cancel}</button></div>}
    {arrival&&<div className="arrival-label" role="status">{dt.entering}</div>}
    <dialog ref={dialog} className={`app-dialog ${modal === "menu" || modal === "settings" || modal === "scenarios" ? "drawer" : ""}`} aria-labelledby="dialog-title" onCancel={closeModal} onClick={event => { if (event.target === event.currentTarget) { const rect = event.currentTarget.getBoundingClientRect(); if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) closeModal(); } }}>
      <div className="dialog-header">{returnToChampion && modal === "progress" && <button className="outline-button" onClick={() => { setModal("champion"); setReturnToChampion(false); }}>{t.back} · {previewName}</button>}<div><span className="caption">{t.demo}</span><h2 id="dialog-title">{modalTitle}</h2></div><button className="icon-button" aria-label={t.close} onClick={closeModal}><Icon name="close" /></button></div>
      <div className="dialog-content">
        {selectedMatch ? <><div className="detail-hero"><Portrait name={selectedMatch.champion} /><div><h3>{selectedMatch.champion}</h3><span className={selectedMatch.win ? "positive" : "negative"}>{selectedMatch.win ? t.victory : t.defeat}</span><p>{t[selectedMatch.queue]}</p></div><strong className="detail-kda">{selectedMatch.kda.join(" / ")}</strong></div><div className="detail-stats"><div><small>{t.duration}</small><strong>{duration(selectedMatch)}</strong></div><div><small>CS/min</small><strong>{number(selectedMatch.cs / (selectedMatch.minutes + selectedMatch.seconds / 60), 1)}</strong></div><div><small>LP</small><strong>{selectedMatch.lp > 0 ? "+" : ""}{selectedMatch.lp || "—"}</strong></div></div><h3>{t.items}</h3><Items items={selectedMatch.items} label={t.items} /><p className="demo-explanation">{t.previewHint}</p><button className="primary-button" onClick={closeModal}>{t.back}<Icon name="arrow" size={16} /></button></> : modal === "settings" ? <><p>{t.preferencesHint}</p><label className="setting-label">{t.appearance}</label><div className="theme-choices">{(["dark", "light"] as const).map(theme => <button className={`theme-choice ${theme} ${preferences.theme === theme ? "selected" : ""}`} key={theme} aria-pressed={preferences.theme === theme} onClick={() => setPreferences(p => ({ ...p, theme }))}><span className="theme-swatch"><i /><i /><i /></span><span>{t[theme]}{preferences.theme === theme && <Icon name="check" size={16} />}</span></button>)}</div><label className="setting-label" htmlFor="language">{t.language}</label><select id="language" value={preferences.locale} onChange={event => setPreferences(p => ({ ...p, locale: event.target.value === "en" ? "en" : "fr" }))}><option value="fr">Français</option><option value="en">English</option></select><label className="switch-row"><span><strong>{t.motion}</strong><small>{t.motionHint}</small></span><input type="checkbox" checked={preferences.motion} onChange={event => setPreferences(p => ({ ...p, motion: event.target.checked }))} /></label>{systemReduced && <p className="small-muted">{t.reduced}</p>}<button className="outline-button" disabled={!motion} onClick={startIntro}><Icon name="replay" size={16} />{t.replay}</button><p className="small-muted">{storageUnavailable ? t.storageUnavailable : t.remembered}</p></> : modal === "scenarios" ? <><p>{t.simulationHint}</p><div className="setting-label">{t.connection}</div><button className="outline-button full" onClick={() => dispatch({ type: "connection", connected: !session.connected })}><span className={`status-dot ${session.connected ? "" : "off"}`} />{session.connected ? t.disconnect : t.reconnect}</button><label className="setting-label" htmlFor="account">{t.account}</label><select id="account" value={session.account} aria-label={t.switchAccount} onChange={event => { dispatch({ type: "account", account: event.target.value === "orbite" ? "orbite" : "nebuleuse" }); setQueue("all"); setScenario("ready"); }}><option value="nebuleuse">Nébuleuse#EUW</option><option value="orbite">Orbite#EUW</option></select><label className="setting-label" htmlFor="history-state">{t.historyState}</label><select id="history-state" value={historyState} onChange={event => setScenario(event.target.value as HistoryState)}><option value="ready">{t.normal}</option><option value="loading">{t.loading}</option><option value="empty">{t.empty}</option><option value="error">{t.failure}</option></select><p className="small-muted">{t.profileHint}</p><button className="primary-button" onClick={closeModal}>{t.back}<Icon name="arrow" size={16} /></button></> : modal === "menu" ? <div className="feature-menu">{(["game", "champion", "progress", "clips", "collection", "settings"] as const).map((destination, i) => <button key={destination} onClick={() => { setPreviewName("Ahri"); setPreviewChampion("Ahri"); showModal(destination); }}><Icon name={(["sword", "shield", "chart", "clip", "eye", "settings"] as const)[i]!} /><span>{destination === "champion" ? t.champions : t[destination]}</span><Icon name="chevron" size={16} /></button>)}</div> : modal === "progress" ? <><span className="preview-pill">{t.preview}</span><div className="progress-detail"><Icon name={goals[goal]!.icon} size={38} /><h3>{goals[goal]!.title}</h3><p>{goals[goal]!.tip}</p><svg className="progress-chart" viewBox="0 0 400 110" role="img" aria-label={t.evolution}><path d="M10 90 100 64 195 74 280 35 390 18" fill="none" stroke="currentColor" strokeWidth="3" />{[[10, 90], [100, 64], [195, 74], [280, 35], [390, 18]].map(([x, y], i) => <circle key={i} cx={x} cy={y} r="5" fill="currentColor" />)}</svg><p className="small-muted">{t.evolution}</p></div><p className="demo-explanation">{t.focusNote}</p></> : modal === "about" ? <><button className="outline-button" onClick={()=>goPage("layouts")}>{dt.layout}</button><p>{t.aboutCopy}</p><p className="legal">{t.legal}</p></> : modal === "clips" ? <><label className="switch-row"><span><strong>{t.capture}</strong><small>{t.demo}</small></span><input type="checkbox" checked={capture} onChange={toggleCapture} /></label><div className="clip-preview"><Icon name="clip" size={42} /><h3>{capture ? t.captureOn : t.captureOff}</h3><p>{t.noCapture}</p></div><p className="demo-explanation">{t.previewHint}</p></> : <><span className="preview-pill">{t.preview}</span>{(modal === "champion" || modal === "player" || modal === "friend") && <div className="detail-hero"><Portrait name={previewChampion} /><div><h3>{previewName}</h3><p>{modal === "champion" ? t.champion : t.player}</p></div></div>}<p>{modal === "game" ? session.connected ? t.draftHint : t.offlineCopy : modal === "champion" ? t.championHint : modal === "player" ? t.profileHint : t.previewHint}</p>{modal === "champion" && <button className="primary-button" onClick={() => { setReturnToChampion(true); setModal("progress"); }}>{t.progress}<Icon name="arrow" size={16} /></button>}<p className="demo-explanation">{t.aboutCopy}</p></>}
      </div>
    </dialog>
    <div className={`toast ${notice ? "visible" : ""}`} role="status" aria-live="polite">{notice && <><Icon name="check" size={17} /><span>{notice}</span></>}</div>
    <AmbientEmbers root={prototypeRoot} theme={preferences.theme} scene={`${page}:${draft.stage>=4?draft.stage:0}`} enabled={motion && !intro && !pageFlaming && page!=="layouts"} />
    {flameCue&&<PageFlame root={prototypeRoot} theme={preferences.theme} cue={flameCue} phase={page==="draft"} enabled={motion&&!intro} onActive={setPageFlaming}/> }{intro && motion && <BurnReveal key={introKey} theme={preferences.theme} root={prototypeRoot} onDone={finishIntro} />}
  </div>;
}
