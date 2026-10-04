import {SelectField} from '../ui/SelectField';
import {buildRankOptions,rankedQueue,rankLabel,rankForQueue} from './buildRanks';
import {buildCopy} from './buildCopy';
import type {Locale} from './state';
export function RankSelect({queue,rank,locale,onChange,allLabel}:{queue:number;rank:string;locale:Locale;onChange:(rank:string)=>void;allLabel?:string}){
 const enabled=rankedQueue(queue);
 return <SelectField label={buildCopy[locale].rank} value={rankForQueue(queue,rank)} disabled={!enabled} onChange={onChange} options={(enabled?buildRankOptions:['ALL']).map(value=>({value,label:value==='ALL'&&allLabel?allLabel:rankLabel(value,locale)}))}/>;
}
