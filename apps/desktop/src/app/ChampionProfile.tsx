import {championImage} from './catalogRuntime';
import {BuildPatchNotice} from './BuildPatchNotice';
import {publicClientPatch} from './clientPatch';
import {BuildSummaryView,BuildOmissions,hasBuildObservations} from './BuildObservations';
import {BuildPopulation,populationName} from './BuildPopulation';
import {RankSelect} from './RankSelect';
import {rankForQueue} from './buildRanks';
import {nextProfileTab,profileTabs} from './championKeyboard';
import {useEffect,useState} from 'react';
import type {CatalogRecord,Role} from '@olc/shared';
import {Icon} from '../ui/Icon';
import type {ChampionsState,Locale} from './state';
import type {ChampionSummary} from './championDirectory';
import {championsCopy} from './championsCopy';
import {buildCopy} from './buildCopy';
import {connectProfile,loadProfile,type ProfileLoad} from './championProfileConnection';
import {CatalogButton,GameDetails} from './GameDetails';
import {catalogDescription,itemStats} from './preparation';
import {formatStat} from './catalogFormat';
import {statLabels} from './preparationCopy';
import {CommunityBuildPanels} from './BuildPreparation';
import {RunePanel,ItemPanel} from './PreparationPanels';
import {useBuilds} from './useBuilds';
import type {ProfileData} from './championProfileConnection';
import './preparation.css';

export function ChampionProfile({champion,locale,state,update,onClose}:{champion:ChampionSummary;locale:Locale;state:ChampionsState;update:(patch:Partial<ChampionsState>)=>void;onClose:()=>void}){
 const t=championsCopy[locale],[attempt,setAttempt]=useState(0),[load,setLoad]=useState<ProfileLoad>({status:'loading'});
 useEffect(()=>connectProfile(()=>loadProfile(locale,champion.id),setLoad),[locale,champion.id,attempt]);
 return <aside className="surface champion-profile" aria-label={`${t.open} · ${champion.names[locale]}`}>
  <header className="champion-profile-hero"><img src={championImage(champion.id)} alt=""/><div><span className="eyebrow">{champion.categories.map(c=>t.classes[c as keyof typeof t.classes]).join(' · ')}</span><h2>{champion.names[locale]}</h2><p>{champion.titles[locale]}</p></div><button className="icon-button" aria-label={t.close} onClick={onClose}><Icon name="close"/></button></header>
  <div className="champion-tabs" role="tablist" aria-label={champion.names[locale]}>{profileTabs.map(tab=><button key={tab} id={`champion-tab-${tab}`} role="tab" tabIndex={state.tab===tab?0:-1} onKeyDown={event=>{const next=nextProfileTab(tab,event.key);if(next){event.preventDefault();update({tab:next});document.getElementById(`champion-tab-${next}`)?.focus()}}} aria-selected={state.tab===tab} aria-controls="champion-tab-content" onClick={()=>update({tab})}>{t[tab]}</button>)}</div>
  <div id="champion-tab-content" role="tabpanel" aria-labelledby={`champion-tab-${state.tab}`} className="champion-profile-body">
   {load.status==='ready'?<ProfileContent key={`${champion.id}:${locale}`} data={load.data} champion={champion} locale={locale} state={state} update={update}/>:<div className="champion-status" role="status"><Icon name="info"/><p>{load.status==='error'?t.error:t.loading}</p>{load.status==='error'&&<button className="button" onClick={()=>setAttempt(n=>n+1)}>{t.retry}</button>}</div>}
  </div>
 </aside>;
}
function ProfileContent({data,champion,locale,state,update}:{data:ProfileData;champion:ChampionSummary;locale:Locale;state:ChampionsState;update:(patch:Partial<ChampionsState>)=>void}){
 const t=championsCopy[locale],b=buildCopy[locale],[detail,setDetail]=useState<CatalogRecord|null>(null);
 const records=[...data.catalog.records,...data.abilities],self=data.abilities.find(r=>r.kind==='champion');
 const stats=self?itemStats(self).filter(s=>statLabels[s.key]&&formatStat(s.value,s.unit,locale)!==null).slice(0,6):[];
 const slots=['passive','Q','W','E','R'];
 return <>{state.tab==='abilities'?<><div className="champion-stat-strip" aria-label={t.baseStats}>{stats.map(s=><span key={s.key}><b>{formatStat(s.value,s.unit,locale)}</b><small>{statLabels[s.key]?.[locale]}</small></span>)}</div><div className="champion-abilities">{slots.map((slot,index)=>{const ability=data.abilities.find(r=>r.id===`${champion.id}:${slot}`);return ability?<article key={slot}><CatalogButton record={ability} locale={locale} onOpen={setDetail}/><div><small>{index===0?t.passive:locale==='fr'?['','A','Z','E','R'][index]:slot}</small><h3>{ability.name}</h3><p>{catalogDescription(ability)}</p></div></article>:null})}</div>{self&&<button className="champion-detail-link" onClick={()=>setDetail(self)}>{t.details}<Icon name="arrow" size={14}/></button>}</>:state.tab==='builds'?<ChampionBuilds championId={champion.id} data={data} locale={locale} state={state} update={update} onOpen={setDetail}/>:<div className="champion-catalog"><p className="champion-caption">{t.catalogHint}</p><RunePanel page={null} records={data.catalog.records} locale={locale} onOpen={setDetail}/><ItemPanel records={data.catalog.records} locale={locale} onOpen={setDetail}/></div>}
 <small className="champion-data-version">{b.patch} {data.catalog.version}</small>
 {detail&&<GameDetails record={detail} records={records} locale={locale} version={data.catalog.version} onClose={()=>setDetail(null)} onOpen={setDetail}/>}
 </>;
}
function ChampionBuilds({championId,data,locale,state,update,onOpen}:{championId:number;data:ProfileData;locale:Locale;state:ChampionsState;update:(patch:Partial<ChampionsState>)=>void;onOpen:(record:CatalogRecord)=>void}){
 const t=championsCopy[locale],b=buildCopy[locale];
 const request={champion_id:championId,patch:data.catalog.version.split('.').slice(0,2).join('.'),platform:state.platform,queue:state.queue,role:state.role,rank:rankForQueue(state.queue,state.rank)};
 const {state:load,retry,choice}=useBuilds(state.role==='UNKNOWN'?null:request);
 const report=load?.status==='ready'?load.report:null;
 return <><div className="champion-build-filters" aria-label={t.filters}>
 <label>{b.role}<select value={state.role} onChange={e=>update({role:e.target.value as Role})}>{Object.entries(b.roles).map(([role,label])=><option key={role} value={role}>{label}</option>)}</select></label>
 <label>{b.region}<select value={state.platform} onChange={e=>update({platform:e.target.value})}>{['EUW1','EUN1','NA1','KR','JP1','BR1','LA1','LA2','OC1','TR1','RU','ME1','SG2','TW2','VN2'].map(r=><option key={r}>{r}</option>)}</select></label>
 <label>{b.queue}<select value={state.queue} onChange={e=>update({queue:Number(e.target.value)})}>{![420,440,400].includes(state.queue)&&<option value={state.queue}>{state.queue}</option>}{([420,440,400] as const).map(q=><option key={q} value={q}>{b.queues[q]}</option>)}</select></label>
 <label>{b.rank}<RankSelect queue={state.queue} rank={state.rank} locale={locale} onChange={rank=>update({rank})} allLabel={report?.request.rank==='ALL'?populationName(report.meta.population_label,'ALL',locale):undefined}/></label>
 </div>{state.role!=='UNKNOWN'&&<BuildPatchNotice choice={choice} locale={locale} actual={report?.request.patch}/ >}{report&&<BuildPopulation report={report} locale={locale}/ >}{report&&hasBuildObservations(report)?<><p className="champion-caption">{t.buildHint}</p><CommunityBuildPanels key={JSON.stringify([request,report.meta.published_at])} readOnly report={report} records={[...data.catalog.records,...data.abilities]} locale={locale} onOpen={onOpen}/><details className="champion-source"><summary>{b.source}</summary><BuildSummaryView report={report} locale={locale}/><BuildOmissions report={report} locale={locale}/><p>{b.sourceHint}</p><p>{b.scope} : {publicClientPatch(report.request.patch)} · {request.platform} · {b.roles[request.role]} · {b.queues[request.queue as 420]} · {request.rank}</p><p>{b.threshold} : {report.meta.min_games} {b.games}</p><p>{b.published} : {Number.isNaN(Date.parse(report.meta.published_at))?'—':new Intl.DateTimeFormat(locale,{dateStyle:'medium',timeStyle:'short'}).format(new Date(report.meta.published_at))}</p></details></>:<div className="champion-status" role="status"><Icon name="chart"/><p>{state.role==='UNKNOWN'?b.chooseRole:load?.status==='error'?b.errors[load.error]:choice.kind==='unavailable'||load?.status==='ready'?b.empty:b.loading}</p>{load?.status!=='loading'&&<button className="button" onClick={retry}>{t.retry}</button>}</div>}</>;
}
