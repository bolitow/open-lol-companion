import {publicClientPatch} from './clientPatch';
import type {BuildReport} from '@olc/shared';
import {rankLabel,simpleRanks} from './buildRanks';
import {buildCopy} from './buildCopy';
import type {Locale} from './state';
export function populationName(label:string|undefined,rank:string,locale:Locale):string {
 const t=buildCopy[locale];
 if(label===undefined||label==='observed_tier')return rankLabel(rank,locale);
 if(label==='collected_sample')return rankLabel('ALL',locale);
 if(label==='unranked_mode')return t.unrankedMode;
 if(label==='unranked')return t.unranked;
 return t.populationNeutral;
}
export function BuildPopulation({report,locale}:{report:BuildReport;locale:Locale}){
 const t=buildCopy[locale],format=new Intl.NumberFormat(locale,{maximumFractionDigits:1});
 const scopes=report.meta.coverage??[];
 return <details className="build-population"><summary>{populationName(report.meta.population_label,report.request.rank,locale)}{scopes.some(scope=>scope.high_elo_biased===true)&&<span className="population-bias">{t.populationBias}</span>}</summary>
 <p>{t.populationScope}</p>{scopes.map(scope=><div key={`${scope.patch}:${scope.platform_id}:${scope.queue_id}`}><small>{publicClientPatch(scope.patch)??'—'} · {scope.platform_id} · {t.queues[scope.queue_id as 420]??scope.queue_id}</small>
 {scope.high_elo_biased===true&&scope.apex_share!==null&&<p>{t.apexShare} : {format.format(scope.apex_share*100)} %</p>}
 {Object.keys(scope.tier_participations).length?<table><thead><tr><th>{t.rank}</th><th>{t.participations}</th><th>{t.share}</th></tr></thead><tbody>{simpleRanks.filter(rank=>scope.tier_participations[rank]!==undefined).map(rank=><tr key={rank}><th>{rankLabel(rank,locale)}</th><td>{format.format(scope.tier_participations[rank]!)}</td><td>{scope.ranked_participations>0?`${format.format(100*scope.tier_participations[rank]!/scope.ranked_participations)} %`:'—'}</td></tr>)}</tbody><tfoot><tr><th>{t.total}</th><td>{format.format(scope.ranked_participations)}</td><td/></tr></tfoot></table>:<p>{t.populationMissing}</p>}</div>)}{!scopes.length&&<p>{t.populationMissing}</p>}
 </details>;
}
