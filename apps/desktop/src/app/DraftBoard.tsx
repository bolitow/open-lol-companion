import {ChampionCard} from './ChampionCard';
import {usePreparation} from './PreparationContext';
import {useEffect,useState} from 'react';
import type {DraftSession,DraftTimer,RunePage} from '@olc/shared';
import {PreparationPanels} from './PreparationPanels';
import {Icon} from '../ui/Icon';
import type {Copy} from './copy';
import type {Locale} from './state';
import {championDetails,draftTeams,secondsRemaining} from './draft';
function Countdown({timer,label}:{timer:DraftTimer|null;label:string}){
 const [now,setNow]=useState(Date.now);
 useEffect(()=>{
  if(!timer)return;
  let interval:ReturnType<typeof setInterval>|undefined;
  const resume=()=>{clearInterval(interval);setNow(Date.now());if(!document.hidden)interval=setInterval(()=>setNow(Date.now()),1000)};
  resume();document.addEventListener('visibilitychange',resume);
  return()=>{clearInterval(interval);document.removeEventListener('visibilitychange',resume)};
 },[timer]);
 const seconds=secondsRemaining(timer,now);
 return seconds===null?null:<div className="draft-clock" aria-label={`${label} : ${seconds} s`}><small>{label}</small><strong>{seconds}<span>s</span></strong></div>;
}
export function DraftBoard({draft,runePage=null,locale,t,connected=false}:{connected?:boolean;draft:DraftSession|null;runePage?:RunePage|null;locale:Locale;t:Copy}){
 const visible=draft?.supported?draft:null;
 const {value,update}=usePreparation(),local=visible?.allies.find(p=>p.local),inspected=value.manual??local?.championId;
 return <div className="draft-content">
  <div className="screen-heading draft-heading"><div><span className="eyebrow">{t.navigation['champ-select']}</span><h1>{t.phases.ChampSelect}</h1><p>{draft?(draft.supported?t.draft.readOnly:t.draft.unsupported):t.game.draftHint}</p></div><Countdown timer={visible?.timer??null} label={t.draft.timer}/></div>
  <div className="team-grid">{draftTeams(visible).map(team=><section key={team.ally?'allies':'enemies'} className={`surface team ${team.side??''}`}><header><h2>{team.ally?t.draft.allies:t.draft.enemies}</h2><span>{team.side?(team.side==='blue'?t.game.blue:t.game.red):t.draft.sideUnknown}</span></header><div className="draft-slots">{Array.from({length:5},(_,i)=>{const player=team.players[i];return <ChampionCard key={`${player?.cellId??i}:${player?.championId??0}:${player?.locked??false}`} player={player} locale={locale} t={t} selected={!!player?.championId&&player.championId===inspected} onSelect={()=>{if(player?.championId)update({manual:player.local?null:player.championId,roleOverride:null})}}/>})}</div><div className="draft-bans"><span>{t.draft.bans}</span>{team.bans.length?team.bans.map((id,i)=>{const c=championDetails(id,locale);return <span className="ban-portrait" key={`${id}:${i}`} title={c?.name??t.draft.unknown} aria-label={c?.name??t.draft.unknown}>{c?<img src={c.image} alt={c.name}/>:<Icon name="close" size={14}/>}</span>}):<span>—</span>}</div></section>)}</div>
  <PreparationPanels connected={connected} page={runePage} locale={locale} draft={visible}/>
 </div>;
}
