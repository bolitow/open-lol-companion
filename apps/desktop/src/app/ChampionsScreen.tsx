import {useLayoutEffect,useRef} from 'react';
import type {ChampionsState,Locale} from './state';
import {Icon} from '../ui/Icon';
import {championDirectory,findChampions,type ChampionSummary} from './championDirectory';
import {championsCopy} from './championsCopy';
import {ChampionProfile} from './ChampionProfile';
import './champions.css';
export function ChampionTile({champion,locale,selected,onSelect}:{champion:ChampionSummary;locale:Locale;selected:boolean;onSelect:()=>void}){
 const t=championsCopy[locale];
 return <button className={`champion-tile ${selected?'selected':''}`} aria-pressed={selected} aria-label={`${champion.names[locale]} · ${t.open}`} onClick={onSelect}>
 <img src={`/game-data/champions/${champion.id}.jpg`} alt="" loading="lazy" width="308" height="560"/><span className="champion-tile-caption"><strong>{champion.names[locale]}</strong><small>{champion.categories.map(c=>t.classes[c as keyof typeof t.classes]).join(' · ')}</small></span>{selected&&<span className="champion-selected"><Icon name="check" size={15}/></span>}</button>;
}
export function ChampionsScreen({locale,state,update}:{locale:Locale;state:ChampionsState;update:(patch:Partial<ChampionsState>)=>void}){
 const t=championsCopy[locale],grid=useRef<HTMLDivElement>(null),cards=findChampions(state.query,state.category,locale,state.descending),selected=championDirectory.champions.find(c=>c.id===state.selected);
 // Restaurer au montage, avant la peinture ; le défilement reste propre à cette liste.
 useLayoutEffect(()=>{if(grid.current)grid.current.scrollTop=state.scrollTop},[]);
 useLayoutEffect(()=>{if(state.scrollTop===0&&grid.current)grid.current.scrollTop=0},[state.scrollTop]);
 const close=()=>{const id=state.selected;update({selected:null});requestAnimationFrame(()=>grid.current?.querySelector<HTMLButtonElement>(`[data-champion="${id}"] button`)?.focus({preventScroll:true}))};
 return <div className={`champions-screen ${selected?'has-selection':''}`}>
 <header className="champions-heading"><div><span className="eyebrow">{t.eyebrow}</span><h1>{t.title}</h1></div><p>{t.hint}</p></header>
 <div className="champions-workspace"><section className="champions-library" aria-label={t.title}>
 <div className="champions-filters"><label className="champion-query"><Icon name="search" size={17}/><input aria-label={t.search} placeholder={t.search} value={state.query} onChange={e=>update({query:e.target.value,scrollTop:0})}/></label><div><select aria-label={t.category} value={state.category} onChange={e=>update({category:e.target.value,scrollTop:0})}><option value="ALL">{t.all}</option>{Object.entries(t.classes).map(([id,name])=><option key={id} value={id}>{name}</option>)}</select><select aria-label={t.sort} value={state.descending?'desc':'asc'} onChange={e=>update({descending:e.target.value==='desc',scrollTop:0})}><option value="asc">{t.ascending}</option><option value="desc">{t.descending}</option></select><small role="status">{new Intl.NumberFormat(locale).format(cards.length)} {t.count}</small></div></div>
 <div className="champion-grid-scroll" ref={grid} onScroll={e=>update({scrollTop:e.currentTarget.scrollTop})}><div className="champion-grid">{cards.map(champion=><div key={champion.id} data-champion={champion.id}><ChampionTile champion={champion} locale={locale} selected={state.selected===champion.id} onSelect={()=>update({selected:champion.id})}/></div>)}</div>{!cards.length&&<div className="champion-status"><Icon name="search"/><p>{t.empty}</p><button className="button" onClick={()=>update({query:'',category:'ALL',scrollTop:0})}>{t.reset}</button></div>}</div>
 </section>{selected&&<ChampionProfile key={`${selected.id}:${locale}`} champion={selected} locale={locale} state={state} update={update} onClose={close}/>}</div>
 </div>;
}
