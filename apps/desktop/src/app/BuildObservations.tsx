import {SelectField} from '../ui/SelectField';
import {useState} from 'react';
import type {BuildReport,CatalogRecord} from '@olc/shared';
import type {Locale} from './state';
import {buildCopy} from './buildCopy';
export const hasBuildObservations=(report:BuildReport)=>!!(report.builds.length||report.summary||report.skill_levels?.length||report.item_events?.length);
const clock=(milliseconds:number)=>`${Math.floor(milliseconds/60000)}:${String(Math.floor(milliseconds/1000)%60).padStart(2,'0')}`;
export function BuildSummaryView({report,locale}:{report:BuildReport;locale:Locale}){
 const summary=report.summary;if(!summary)return null;
 const t=buildCopy[locale],format=new Intl.NumberFormat(locale,{maximumFractionDigits:1});
 return <section className="build-observed-summary"><h3>{t.championSummary}</h3><div className="build-metrics"><span>{format.format(summary.games)} <small>{t.games}</small></span><span><small>{t.winrate}</small> {summary.win_rate===null?'—':`${format.format(summary.win_rate)} %`}</span><span><small>{t.championPickRate}</small> {summary.pick_rate===null?'—':`${format.format(summary.pick_rate)} %`}</span></div></section>;
}
export function SkillTimings({report,locale}:{report:BuildReport;locale:Locale}){
 const rows=report.skill_levels??[];if(!rows.length)return null;
 const t=buildCopy[locale],keys=locale==='fr'?['A','Z','E','R']:['Q','W','E','R'];
 return <details className="build-observations"><summary>{t.skillTimings}</summary><p>{t.skillTimingsHint}</p><div className="observation-scroll"><table><thead><tr><th>{t.skillPoint}</th><th>{t.skill_order}</th><th>{t.meanTime}</th><th>{t.games}</th></tr></thead><tbody>{[...rows].sort((a,b)=>a.point-b.point||a.slot-b.slot).map(row=><tr key={`${row.point}:${row.slot}`}><td>{row.point}</td><td>{keys[row.slot-1]}</td><td>{clock(row.mean_timestamp_ms)}</td><td>{new Intl.NumberFormat(locale).format(row.games)}</td></tr>)}</tbody></table></div></details>;
}
export function ItemTimings({report,records,locale}:{report:BuildReport;records:CatalogRecord[];locale:Locale}){
 const t=buildCopy[locale],events=report.item_events??[];
 const [selected,setSelected]=useState<number|null>(null),[kind,setKind]=useState('ITEM_PURCHASED'),[page,setPage]=useState(0);
 const items=[...new Set(events.map(event=>event.item_id))];
 const item=selected!==null&&items.includes(selected)?selected:items[0];
 const kinds=[...new Set(events.filter(event=>event.item_id===item).map(event=>event.event))];
 const eventKind=kinds.includes(kind)?kind:kinds[0];
 const rows=events.filter(event=>event.item_id===item&&event.event===eventKind).sort((a,b)=>a.minute-b.minute);
 const size=24,last=Math.max(0,Math.ceil(rows.length/size)-1),current=Math.min(page,last);
 if(!events.length)return null;
 const record=records.find(record=>record.kind==='item'&&record.id===String(item));
 const eventName=(value:string)=>t.itemEventNames[value as keyof typeof t.itemEventNames]??t.otherEvent;
 return <details className="build-observations"><summary>{t.itemTimings}</summary><p>{t.itemTimingsHint}</p>
 <div className="observation-controls">{record?.icon&&<img src={record.icon} width={32} height={32} alt=""/>}<SelectField label={t.observedItem} value={String(item)} onChange={value=>{setSelected(Number(value));setPage(0)}} options={items.map(id=>({value:String(id),label:records.find(record=>record.kind==='item'&&record.id===String(id))?.name??`${t.unknown} · ${id}`}))}/><SelectField label={t.eventType} value={eventKind??''} onChange={value=>{setKind(value);setPage(0)}} options={kinds.map(value=>({value,label:eventName(value)}))}/></div>
 <div className="observation-scroll"><table><thead><tr><th>{t.minuteBucket}</th><th>{t.events}</th></tr></thead><tbody>{rows.slice(current*size,(current+1)*size).map(row=><tr key={row.minute}><td>{row.minute}–{row.minute+1} min</td><td>{new Intl.NumberFormat(locale).format(row.events)}</td></tr>)}</tbody></table></div>
 {last>0&&<div className="observation-controls"><button disabled={current===0} onClick={()=>setPage(current-1)}>{t.previous}</button><span>{current+1} / {last+1}</span><button disabled={current===last} onClick={()=>setPage(current+1)}>{t.next}</button></div>}
 {report.max_item_events!=null&&<p>{t.eventCap} : {report.max_item_events}{report.omitted_item_events!=null?` · ${t.eventsOmitted} : ${report.omitted_item_events}`:''}</p>}<p>{t.eventCapBias}</p></details>;
}
export function BuildOmissions({report,locale}:{report:BuildReport;locale:Locale}){
 const categories=report.omitted_build_variants_by_category??[];if(!categories.length&&report.max_build_variants_per_category==null)return null;
 const t=buildCopy[locale];
 return <section className="build-observations"><h3>{t.omittedVariants}</h3><p>{t.omittedVariantsHint}</p>{report.max_build_variants_per_category!=null&&<p>{t.variantCap} : {report.max_build_variants_per_category}</p>}<ul>{categories.map(row=><li key={row.category}>{t.observationCategories[row.category as keyof typeof t.observationCategories]??(typeof t[row.category as keyof typeof t]==='string'?t[row.category as keyof typeof t] as string:t.otherCategory)} : {new Intl.NumberFormat(locale).format(row.omitted)}</li>)}</ul></section>;
}
