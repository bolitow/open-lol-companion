import {playersCopy} from './playersCopy';
import {playerPlatforms} from './playerStore';
import {useEffect,useId,useRef,useState} from 'react';
import {Icon} from '../ui/Icon';
import {findChampions} from './championDirectory';
import {championsCopy} from './championsCopy';
import type {Copy} from './copy';
import type {Locale,Screen} from './state';
export function ChampionSearch({locale,t,screen,onPage,onChampion,onPlayer}:{locale:Locale;t:Copy;screen:Screen;onPage:(page:Screen)=>void;onChampion:(id:number)=>void;onPlayer:(query:string,platform:string)=>void}){
 const p=playersCopy[locale],[platform,setPlatform]=useState('EUW1');
 const c=championsCopy[locale],[query,setQuery]=useState(''),[open,setOpen]=useState(false),[active,setActive]=useState(0),input=useRef<HTMLInputElement>(null),id=useId();
 const champions=query.trim()?findChampions(query,'ALL',locale).slice(0,8):[];
 const pages=(Object.keys(t.navigation) as Screen[]).filter(p=>t.navigation[p].toLocaleLowerCase(locale).includes(query.trim().toLocaleLowerCase(locale)));
 const results=[...(query.includes('#')?[{key:'player-query',name:`${p.search} ${query.trim()}`,image:null,choose:()=>onPlayer(query,platform)}]:[]),...champions.map(champion=>({key:`c${champion.id}`,name:champion.names[locale],image:`/game-data/champions/${champion.id}.jpg`,choose:()=>onChampion(champion.id)})),...pages.map(page=>({key:page,name:t.navigation[page],image:null,choose:()=>onPage(page)}))];
 const choose=(index:number)=>{results[index]?.choose();setOpen(false);setQuery('');setActive(0)};
 useEffect(()=>{const handle=(event:KeyboardEvent)=>{if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='k'){event.preventDefault();input.current?.focus();setOpen(true)}};window.addEventListener('keydown',handle);return()=>window.removeEventListener('keydown',handle)},[]);
 useEffect(()=>{if(open&&results[active])document.getElementById(`${id}-${results[active].key}`)?.scrollIntoView({block:'nearest'})},[active,open,query,locale]);
 useEffect(()=>{setOpen(false);setQuery('');setActive(0)},[screen]);
 return <div className="app-search champion-global-search" onBlur={event=>{if(!event.currentTarget.contains(event.relatedTarget))setOpen(false)}}>
 <label className="search-input"><Icon name="search"/><input ref={input} value={query} aria-label={p.global} placeholder={p.global} role="combobox" aria-expanded={open} aria-autocomplete="list" aria-controls={`${id}-list`} aria-activedescendant={open&&results[active]?`${id}-${results[active].key}`:undefined} onFocus={()=>setOpen(true)} onChange={event=>{setQuery(event.target.value);setOpen(true);setActive(0)}} onKeyDown={event=>{
 if(event.key==='Escape'){setOpen(false);event.stopPropagation()}
 if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();setOpen(true);setActive(i=>results.length?(i+(event.key==='ArrowDown'?1:-1)+results.length)%results.length:0)}
 if(event.key==='Enter'){event.preventDefault();if(open)choose(active);else setOpen(true)}
 }}/><kbd>⌘ / Ctrl K</kbd></label>
 {open&&<div className="search-results champion-global-results"><div role="listbox" id={`${id}-list`} aria-label={c.results}>{results.length?results.map((result,index)=><div key={result.key}>{result.key===`c${champions[0]?.id}`&&champions.length>0&&<span className="eyebrow">{c.title}</span>}{result.key===pages[0]&&pages.length>0&&<span className="eyebrow">{c.pages}</span>}<button id={`${id}-${result.key}`} role="option" aria-selected={active===index} tabIndex={-1} onMouseDown={event=>event.preventDefault()} onClick={()=>choose(index)} onMouseEnter={()=>setActive(index)}>{result.image&&<img src={result.image} alt=""/>}<span>{result.name}</span><Icon name="arrow" size={15}/></button></div>):<p role="status">{c.noResults}</p>}</div><div className="global-player-region"><label>{p.region}<select value={platform} onChange={event=>setPlatform(event.target.value)}>{playerPlatforms.map(value=><option key={value}>{value}</option>)}</select></label><button onClick={()=>{onPlayer(query,platform);setOpen(false);setQuery('')}}>{p.findPlayer}<Icon name="arrow" size={14}/></button></div></div>}
 </div>;
}
