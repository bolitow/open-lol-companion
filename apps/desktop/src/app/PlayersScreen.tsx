import {rankEmblemUrl} from './rankEmblems';
import {profileIconUrl} from './AccountControl';
import {SelectField} from '../ui/SelectField';
import {useId,useLayoutEffect,useRef,useState,type ReactNode} from 'react';
import type {PlayerRequest,PlayerProfile} from '@olc/shared';
import {Icon} from '../ui/Icon';
import {championDetails} from './draft';
import {parsePlayerQuery,playerKey,playerPlatforms,type PlayerEntry,type PlayersState,type PlayerStore,type RememberedPlayer} from './playerStore';
import {playersCopy} from './playersCopy';
import type {Locale} from './state';
import './players.css';

type Props={locale:Locale;state:PlayersState;store:PlayerStore;onChampion:(id:number)=>void};
const formatDate=(seconds:number,locale:Locale)=>new Intl.DateTimeFormat(locale,{dateStyle:'short',timeStyle:'short'}).format(seconds*1000);
const modeLabel=(queue:number,locale:Locale)=>{const t=playersCopy[locale];return queue===0?t.custom:queue===420?t.solo:queue===440?t.flex:queue===450?t.aram:`${t.mode} ${queue}`};

export function PlayerSearchForm({locale,state,store}:{locale:Locale;state:PlayersState;store:PlayerStore}){
 const t=playersCopy[locale],id=useId(),[invalid,setInvalid]=useState(false);
 return <form className="player-search-form surface" onSubmit={event=>{event.preventDefault();const request=parsePlayerQuery(state.query,state.platform);setInvalid(!request);if(request)store.select(request)}}>
  <label><span>{t.riotId}</span><input value={state.query} placeholder={t.placeholder} aria-invalid={invalid} aria-describedby={invalid?`${id}-error`:undefined} onChange={event=>{store.edit(event.target.value,state.platform);setInvalid(false)}} autoComplete="off" spellCheck={false}/></label>
  <label><span>{t.region}</span><SelectField label={t.region} value={state.platform} onChange={value=>store.edit(state.query,value)} options={playerPlatforms.map(value=>({value,label:value}))}/></label>
  <button className="button primary" type="submit"><Icon name="search" size={17}/>{t.search}</button>
  {invalid&&<p id={`${id}-error`} role="alert">{t.invalid}</p>}
 </form>;
}
export function PlayerIdentity({profile,identity,locale,actions,headerActions}:{profile:PlayerProfile|null;identity:RememberedPlayer;locale:Locale;actions?:ReactNode;headerActions?:ReactNode}){
 const t=playersCopy[locale],value=profile??identity,url=profileIconUrl(profile?.profile_icon_id??identity.profile_icon_id);
 return <><div className="player-identity"><PlayerAvatar key={url} url={url} initials={Array.from(value.game_name).slice(0,2).join('').toLocaleUpperCase(locale)}/><div><span className="eyebrow">{value.platform}{profile?.summoner_level!==null&&profile?.summoner_level!==undefined?` · ${t.level} ${new Intl.NumberFormat(locale).format(profile.summoner_level)}`:''}</span><h2 title={`${value.game_name}#${value.tag_line}`}>{value.game_name}<small>#{value.tag_line}</small></h2>{profile&&<small>{profile.source==='lcu'?t.sourceLocal:t.sourceApi}</small>}</div>{headerActions}</div>{actions}{profile&&<div className="player-ranks">{[420,440].map(queue=>{const rank=profile.ranks.find(row=>row.queue_id===queue),emblem=rank?.status==='ranked'?rankEmblemUrl(rank.tier):null;return <div className="player-rank" key={queue}><span className="rank-emblem-slot" aria-hidden="true">{emblem?<img className="rank-emblem" src={emblem} alt=""/>:<Icon name="shield" size={20}/>}</span><div><span>{modeLabel(queue,locale)}</span><strong>{!rank?t.rankUnavailable:rank.status==='unranked'?t.unranked:`${t.tiers[rank.tier as keyof typeof t.tiers]??rank.tier} ${['MASTER','GRANDMASTER','CHALLENGER'].includes(rank.tier??'')?'':rank.division??''}`}</strong>{rank?.status==='ranked'&&<small>{rank.league_points===null?'—':new Intl.NumberFormat(locale).format(rank.league_points)} {t.lp}</small>}</div></div>})}</div>}</>;
}
function PlayerAvatar({url,initials}:{url:string|null;initials:string}){
 const [failed,setFailed]=useState(false);
 return <span className="player-avatar" aria-hidden="true">{url&&!failed?<img src={url} alt="" referrerPolicy="no-referrer" onError={()=>setFailed(true)}/>:initials}</span>;
}
function EntryStatus({entry,locale,retry}:{entry:PlayerEntry;locale:Locale;retry:()=>void}){
 const t=playersCopy[locale];return <>{entry.loading&&<p className="player-message" role="status">{t.loading}</p>}{entry.error&&<div className="player-message" role="alert"><p>{t.errors[entry.error]}</p>{entry.error!=='desktop_required'&&<button className="button" onClick={retry}>{t.retry}</button>}</div>}</>;
}
export function PlayerHistory({entry,locale,store,onChampion}:{entry:PlayerEntry;locale:Locale;store:PlayerStore;onChampion:(id:number)=>void}){
 const t=playersCopy[locale],list=useRef<HTMLDivElement>(null),key=playerKey(entry.identity),wins=entry.matches.filter(match=>match.win).length;
 useLayoutEffect(()=>{if(list.current)list.current.scrollTop=entry.scrollTop},[key]);
 return <section className="surface player-history" aria-busy={entry.loading||entry.historyLoading}><header><h2>{t.history}</h2>{entry.matches.length>0&&<span className="history-record" title={t.recent}><b>{wins} {t.winsShort}</b><span> / </span><b>{entry.matches.length-wins} {t.lossesShort}</b><small>{t.recent}</small></span>}</header>
 {entry.historySource&&<p className="player-message">{entry.historySource==='lcu'?t.localHistory:t.sourceApi}</p>}
 {entry.matches.length>0&&<div className="history-results" aria-hidden="true">{wins>0&&<i className="won" style={{flex:wins}}/>}{entry.matches.length>wins&&<i className="lost" style={{flex:entry.matches.length-wins}}/>}</div>}
 <div className="player-history-scroll" ref={list} onScroll={event=>store.scroll(event.currentTarget.scrollTop,entry.identity)} tabIndex={0} role="region" aria-label={t.history}>
  {entry.matches.map(match=>{const champion=championDetails(match.champion_id,locale);return <article className={`player-match ${match.win?'won':'lost'}`} key={match.match_id}>
   <div className="match-outcome"><strong>{match.win?t.win:t.loss}</strong><time dateTime={new Date(match.game_start_ms).toISOString()}>{new Intl.DateTimeFormat(locale,{month:'short',day:'numeric'}).format(match.game_start_ms)}</time></div>
   <button className="match-champion" disabled={!champion} onClick={()=>onChampion(match.champion_id)} aria-label={champion?`${t.openChampion} : ${champion.name}`:t.unknownChampion}>{champion?<img src={champion.image} alt="" loading="lazy"/>:<Icon name="sword"/>}<span>{champion?.name??`${t.champion} ${match.champion_id}`}<small>{modeLabel(match.queue_id,locale)}</small></span><Icon name="arrow" size={14}/></button>
   <span className="match-kda" aria-label={t.kda}>{match.kills??'—'} <small>/</small> {match.deaths??'—'} <small>/</small> {match.assists??'—'}</span>
   <span className="match-duration" title={t.duration}>{Math.floor(match.duration_s/60)}:{String(match.duration_s%60).padStart(2,'0')}</span>
  </article>})}
  {!entry.matches.length&&!entry.profile&&!entry.historyLoading&&<div className="history-placeholder" role="status"><Icon name={entry.loading?'replay':'sword'} size={26}/><strong>{entry.loading?t.loadingMatches:t.historyUnavailable}</strong>{!entry.loading&&<p>{t.historyProfileHint}</p>}</div>}
  {!entry.matches.length&&!entry.loading&&!entry.historyLoading&&!entry.historyError&&entry.profile&&<p className="player-message">{entry.profile?.source==='lcu'?t.localEmpty:t.empty}</p>}
  {entry.omitted>0&&<p className="player-message">{entry.historySource==='lcu'?t.localOmitted:t.omitted}</p>}
  {(entry.historyLoading||entry.loading&&!entry.matches.length&&!!entry.profile)&&<p className="player-message" role="status">{t.loadingMatches}</p>}
  {entry.historyError&&<p className="player-message" role="alert">{t.errors[entry.historyError]}</p>}
  {entry.profile&&(entry.next!==null||entry.historyRefresh)&&!entry.historyLoading&&<button className="button player-more" disabled={entry.loading} onClick={()=>void store.more(entry.identity)}>{entry.historyError?t.retry:t.more}<Icon name="chevron" size={15}/></button>}
  {entry.profile&&entry.next===null&&!entry.historyRefresh&&!entry.historyLoading&&<p className="player-message">{entry.historySource==='lcu'?t.localEnd:t.end}</p>}
 </div>
 {entry.historyFetchedAt!==null&&<small className="player-fetched">{t.fetched} {formatDate(entry.historyFetchedAt,locale)}</small>}
 </section>;
}
export function PlayersScreen({locale,state,store,onChampion}:Props){
 const t=playersCopy[locale],entry=state.viewed?state.entries[playerKey(state.viewed)]:null,isHome=state.home&&state.viewed&&playerKey(state.home)===playerKey(state.viewed);
 return <div className="players-screen"><header className="players-heading"><h1>{t.title}</h1><PlayerSearchForm locale={locale} state={state} store={store}/></header>
 {entry?<div className="players-content"><section className="surface player-profile"><PlayerIdentity profile={entry.profile} identity={entry.identity} locale={locale} actions={entry.profile?<div className="player-actions"><button className="button primary" disabled={!!isHome||state.connected} title={state.connected?t.followsClient:undefined} onClick={store.pin}><Icon name={isHome?'check':'pin'} size={16}/>{isHome?t.pinned:t.pin}</button><button className="icon-button" aria-label={t.refresh} onClick={()=>store.refresh()}><Icon name="replay" size={16}/></button></div>:null}/><EntryStatus entry={entry} locale={locale} retry={()=>store.refresh()}/>{entry.profile&&<><p className="player-message">{state.connected?t.followsClient:t.local}</p><small className="player-fetched">{t.fetched} {formatDate(entry.profile.fetched_at,locale)}</small></>}{state.storageFailed&&<p role="status" className="player-message">{t.storage}</p>}</section><PlayerHistory entry={entry} locale={locale} store={store} onChampion={onChampion}/></div>:<section className="surface players-empty"><Icon name="users" size={38}/><h2>{t.findPlayer}</h2><p>{t.hint}</p></section>}
 </div>;
}
export function HomePlayer({locale,state,store,onBrowse}:{locale:Locale;state:PlayersState;store:PlayerStore;onBrowse:()=>void}){
 const t=playersCopy[locale],entry=state.home?state.entries[playerKey(state.home)]:null;
 return <section className="surface home-player">{state.home?<>
  {!state.active&&<span className="eyebrow home-account-status" role="status">{state.connected?t.unknownAccount:t.disconnected}</span>}
  <PlayerIdentity identity={state.home} profile={entry?.profile??null} locale={locale} headerActions={<div className="home-player-actions">
   <button className="icon-button" aria-label={t.refresh} title={t.refresh} disabled={entry?.loading} onClick={()=>store.refresh(state.home)}><Icon name="replay" size={16}/></button>
   <button className="icon-button" aria-label={t.view} title={t.view} onClick={()=>{store.select(state.home!);onBrowse()}}><Icon name="arrow" size={17}/></button>
   {!state.connected&&<button className="icon-button" title={t.forget} aria-label={t.forget} onClick={store.forget}><Icon name="close" size={15}/></button>}
  </div>}/>
  {entry&&<EntryStatus entry={entry} locale={locale} retry={()=>store.refresh(state.home)}/>}
 </>:<><Icon name="users" size={25}/><h2>{state.connected?t.waitingAccount:t.noHome}</h2><p>{t.noHomeHint}</p><button className="button primary" onClick={onBrowse}>{state.connected?t.findPlayer:t.choose}<Icon name="arrow" size={15}/></button></>}{state.storageFailed&&<p role="status">{t.storage}</p>}</section>;
}
