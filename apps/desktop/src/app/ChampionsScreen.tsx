import {useLayoutEffect,useRef} from 'react';
import {usePresence} from '../ui/usePresence';
import type {ChampionsState,Locale} from './state';
import {Icon} from '../ui/Icon';
import {championDirectory,findChampions,type ChampionSummary} from './championDirectory';
import {championsCopy} from './championsCopy';
import {ChampionProfile} from './ChampionProfile';
import './champions.css';
import {SelectField} from '../ui/SelectField';
export function ChampionTile({champion,locale,selected,onSelect}:{champion:ChampionSummary;locale:Locale;selected:boolean;onSelect:()=>void}){
 const t=championsCopy[locale];
 return <button className={`champion-tile ${selected?'selected':''}`} aria-pressed={selected} aria-label={`${champion.names[locale]} · ${t.open}`} onClick={onSelect}>
 <img src={`/game-data/champions/${champion.id}.jpg`} alt="" loading="lazy" width="308" height="560"/><span className="champion-tile-caption"><strong>{champion.names[locale]}</strong><small>{champion.categories.map(c=>t.classes[c as keyof typeof t.classes]).join(' · ')}</small></span>{selected&&<span className="champion-selected"><Icon name="check" size={15}/></span>}</button>;
}
export function ChampionsScreen({locale,state,update}:{locale:Locale;state:ChampionsState;update:(patch:Partial<ChampionsState>)=>void}){
 const t=championsCopy[locale],grid=useRef<HTMLDivElement>(null),cards=findChampions(state.query,state.category,locale,state.descending),selected=championDirectory.champions.find(c=>c.id===state.selected);
 const presence=usePresence(selected??null);
 // Restaurer au montage, avant la peinture ; le défilement reste propre à cette liste.
 useLayoutEffect(()=>{if(grid.current)grid.current.scrollTop=state.scrollTop},[]);
 useLayoutEffect(()=>{if(state.scrollTop===0&&grid.current)grid.current.scrollTop=0},[state.scrollTop]);
 const close=()=>{const id=state.selected;update({selected:null});requestAnimationFrame(()=>grid.current?.querySelector<HTMLButtonElement>(`[data-champion="${id}"] button`)?.focus({preventScroll:true}))};
 return <div className={`champions-screen ${presence.present?'has-selection':''}`}>
 <div className="champions-workspace"><section className="champions-library" aria-label={t.title}>
 <div className="champions-filters"><label className="champion-query"><Icon name="search" size={17}/><input aria-label={t.search} placeholder={t.search} value={state.query} onChange={e=>update({query:e.target.value,scrollTop:0})}/></label><div><SelectField label={t.category} value={state.category} onChange={value=>update({category:value,scrollTop:0})} options={[{value:'ALL',label:t.all},...Object.entries(t.classes).map(([value,label])=>({value,label}))]}/><SelectField label={t.sort} value={state.descending?'desc':'asc'} onChange={value=>update({descending:value==='desc',scrollTop:0})} options={[{value:'asc',label:t.ascending},{value:'desc',label:t.descending}]}/><small role="status">{new Intl.NumberFormat(locale).format(cards.length)} {t.count}</small></div></div>
 <div className="champion-grid-scroll" ref={grid} onScroll={e=>update({scrollTop:e.currentTarget.scrollTop})}><div className="champion-grid">{cards.map(champion=><div key={champion.id} data-champion={champion.id}><ChampionTile champion={champion} locale={locale} selected={state.selected===champion.id} onSelect={()=>update({selected:champion.id})}/></div>)}</div>{!cards.length&&<div className="champion-status"><Icon name="search"/><p>{t.empty}</p><button className="button" onClick={()=>update({query:'',category:'ALL',scrollTop:0})}>{t.reset}</button></div>}</div>
 </section>{presence.value&&<ChampionProfile key={`${presence.value.id}:${locale}`} closing={presence.closing} champion={presence.value} locale={locale} state={state} update={update} onClose={close}/>}</div>
 </div>;
}
