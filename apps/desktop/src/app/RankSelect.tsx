import {buildRankOptions,rankedQueue,rankLabel,rankForQueue} from './buildRanks';
import {buildCopy} from './buildCopy';
import type {Locale} from './state';
export function RankSelect({queue,rank,locale,onChange,allLabel}:{queue:number;rank:string;locale:Locale;onChange:(rank:string)=>void;allLabel?:string}){
 const enabled=rankedQueue(queue);
 return <select aria-label={buildCopy[locale].rank} value={rankForQueue(queue,rank)} disabled={!enabled} onChange={event=>onChange(event.target.value)}>{(enabled?buildRankOptions:['ALL']).map(value=><option key={value} value={value}>{value==='ALL'&&allLabel?allLabel:rankLabel(value,locale)}</option>)}</select>;
}
