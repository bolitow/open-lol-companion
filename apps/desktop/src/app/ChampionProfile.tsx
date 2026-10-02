import {SelectField} from '../ui/SelectField';
import {ChampionAbilities} from './ChampionAbilities';
import {ChampionVideoScope} from './ChampionVideoScope';
import {nextProfileTab,profileTabs} from './championKeyboard';
import {useEffect,useState} from 'react';
import type {CatalogRecord,Role} from '@olc/shared';
import {Icon} from '../ui/Icon';
import type {ChampionsState,Locale} from './state';
import type {ChampionSummary} from './championDirectory';
import {championsCopy} from './championsCopy';
import {buildCopy} from './buildCopy';
import {connectProfile,loadProfile,type ProfileLoad} from './championProfileConnection';
import {GameDetails} from './GameDetails';
import {CommunityBuildPanels} from './BuildPreparation';
import {RunePanel,ItemPanel} from './PreparationPanels';
import {useBuilds} from './useBuilds';
import type {ProfileData} from './championProfileConnection';
import './preparation.css';

export function ChampionProfile({champion,locale,state,update,onClose,closing=false}:{closing?:boolean;champion:ChampionSummary;locale:Locale;state:ChampionsState;update:(patch:Partial<ChampionsState>)=>void;onClose:()=>void}){
 const t=championsCopy[locale],[attempt,setAttempt]=useState(0),[load,setLoad]=useState<ProfileLoad>({status:'loading'});
 useEffect(()=>connectProfile(()=>loadProfile(locale,champion.id),setLoad),[locale,champion.id,attempt]);
 return <ChampionVideoScope championId={champion.id} closing={closing}><aside className={`surface champion-profile motion-panel ${state.tab==='abilities'?'is-abilities':''}`} data-state={closing?'closed':'open'} inert={closing} aria-hidden={closing} aria-label={`${t.open} · ${champion.names[locale]}`}>
  <header className="champion-profile-hero"><img src={`/game-data/champions/${champion.id}.jpg`} alt=""/><div><span className="eyebrow">{champion.categories.map(c=>t.classes[c as keyof typeof t.classes]).join(' · ')}</span><h2>{champion.names[locale]}</h2><p>{champion.titles[locale]}</p></div><button className="icon-button" aria-label={t.close} onClick={onClose}><Icon name="close"/></button></header>
  <div className="champion-tabs" role="tablist" aria-label={champion.names[locale]}>{profileTabs.map(tab=><button key={tab} id={`champion-tab-${tab}`} role="tab" tabIndex={state.tab===tab?0:-1} onKeyDown={event=>{const next=nextProfileTab(tab,event.key);if(next){event.preventDefault();update({tab:next});document.getElementById(`champion-tab-${next}`)?.focus()}}} aria-selected={state.tab===tab} aria-controls="champion-tab-content" onClick={()=>update({tab})}>{t[tab]}</button>)}</div>
  <div id="champion-tab-content" role="tabpanel" aria-labelledby={`champion-tab-${state.tab}`} className="champion-profile-body">
   {load.status==='ready'?<ProfileContent key={`${champion.id}:${locale}`} data={load.data} champion={champion} locale={locale} state={state} update={update}/>:<div className="champion-status" role="status"><Icon name="info"/><p>{load.status==='error'?t.error:t.loading}</p>{load.status==='error'&&<button className="button" onClick={()=>setAttempt(n=>n+1)}>{t.retry}</button>}</div>}
  </div>
 </aside></ChampionVideoScope>;
}
function ProfileContent({data,champion,locale,state,update}:{data:ProfileData;champion:ChampionSummary;locale:Locale;state:ChampionsState;update:(patch:Partial<ChampionsState>)=>void}){
 const t=championsCopy[locale],b=buildCopy[locale],[detail,setDetail]=useState<CatalogRecord|null>(null);
 const records=[...data.catalog.records,...data.abilities];
 return <>{state.tab==='abilities'?<ChampionAbilities championId={champion.id} records={data.abilities} locale={locale} version={data.catalog.version} onOpen={setDetail}/>:state.tab==='builds'?<ChampionBuilds championId={champion.id} data={data} locale={locale} state={state} update={update} onOpen={setDetail}/>:<div className="champion-catalog"><p className="champion-caption">{t.catalogHint}</p><RunePanel page={null} records={data.catalog.records} locale={locale} onOpen={setDetail}/><ItemPanel records={data.catalog.records} locale={locale} onOpen={setDetail}/></div>}
 <small className="champion-data-version">{b.patch} {data.catalog.version}</small>
 {detail&&<GameDetails record={detail} records={records} locale={locale} version={data.catalog.version} onClose={()=>setDetail(null)} onOpen={setDetail}/>}
 </>;
}
function ChampionBuilds({championId,data,locale,state,update,onOpen}:{championId:number;data:ProfileData;locale:Locale;state:ChampionsState;update:(patch:Partial<ChampionsState>)=>void;onOpen:(record:CatalogRecord)=>void}){
 const t=championsCopy[locale],b=buildCopy[locale];
 const request={champion_id:championId,patch:data.catalog.version.split('.').slice(0,2).join('.'),platform:state.platform,queue:state.queue,role:state.role,rank:state.rank};
 const {state:load,retry}=useBuilds(request);
 const report=load?.status==='ready'?load.report:null;
 return <><div className="champion-build-filters" aria-label={t.filters}>
 <label>{b.role}<SelectField label={b.role} value={state.role} onChange={value=>update({role:value as Role})} options={Object.entries(b.roles).filter(([role])=>role!=='UNKNOWN').map(([value,label])=>({value,label}))}/></label>
 <label>{b.region}<SelectField label={b.region} value={state.platform} onChange={value=>update({platform:value})} options={['EUW1','EUN1','NA1','KR','JP1','BR1','LA1','LA2','OC1','TR1','RU','ME1','SG2','TW2','VN2'].map(value=>({value,label:value}))}/></label>
 <label>{b.queue}<SelectField label={b.queue} value={String(state.queue)} onChange={value=>update({queue:Number(value)})} options={([420,440,400] as const).map(q=>({value:String(q),label:b.queues[q]}))}/></label>
 <label>{b.rank}<SelectField label={b.rank} value={state.rank} onChange={value=>update({rank:value})} options={['ALL','IRON','BRONZE','SILVER','GOLD','PLATINUM','EMERALD','DIAMOND','MASTER','GRANDMASTER','CHALLENGER'].map((value,i)=>({value,label:(locale==='fr'?[t.allRanks,'Fer','Bronze','Argent','Or','Platine','Émeraude','Diamant','Maître','Grand maître','Challenger']:[t.allRanks,'Iron','Bronze','Silver','Gold','Platinum','Emerald','Diamond','Master','Grandmaster','Challenger'])[i]??value}))}/></label>
 </div>{report?.builds.length?<><p className="champion-caption">{t.buildHint}</p><CommunityBuildPanels key={JSON.stringify([request,report.meta.published_at])} readOnly report={report} records={[...data.catalog.records,...data.abilities]} locale={locale} onOpen={onOpen}/><details className="champion-source"><summary>{b.source}</summary><p>{b.sourceHint}</p><p>{b.scope} : {request.patch} · {request.platform} · {b.roles[request.role]} · {b.queues[request.queue as 420]} · {request.rank}</p><p>{b.threshold} : {report.meta.min_games} {b.games}</p><p>{b.published} : {Number.isNaN(Date.parse(report.meta.published_at))?'—':new Intl.DateTimeFormat(locale,{dateStyle:'medium',timeStyle:'short'}).format(new Date(report.meta.published_at))}</p></details></>:<div className="champion-status" role="status"><Icon name="chart"/><p>{load?.status==='error'?b.errors[load.error]:load?.status==='ready'?b.empty:b.loading}</p>{load?.status!=='loading'&&<button className="button" onClick={retry}>{t.retry}</button>}</div>}</>;
}
