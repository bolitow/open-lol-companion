import {useEffect,useMemo,useState,useRef} from 'react';
import {createPortal} from 'react-dom';
import type {CatalogRecord,RunePage} from '@olc/shared';
import {Icon} from '../ui/Icon';
import {loadCatalog,type PreparationCatalog} from './catalog';
import {CatalogButton,CatalogIcon,GameDetails} from './GameDetails';
import {activeRuneExploration,displayedRuneStyles,runeStyles,runeRows,shardRows,searchableItems} from './preparation';
import {preparationCopy} from './preparationCopy';
import type {Locale} from './state';
import './preparation.css';
import {BuildPreparation} from './BuildPreparation';
import type {DraftSession} from '@olc/shared';

export interface RunePanelEditor {
 disabled:boolean;chooseLabel:string;incompleteLabel:string;
 onStyle:(branch:'primary'|'secondary',id:number)=>void;
 onChoose:(branch:'primary'|'secondary'|'shard',slot:number,id:number)=>void;
}
export function RunePanel({records,page,locale,onOpen,onExpand,sourceLabel,returnLabel,incompleteLabel,editor}:{editor?:RunePanelEditor;incompleteLabel?:string;sourceLabel?:string;returnLabel?:string;onExpand?:()=>void;records:CatalogRecord[];page:RunePage|null;locale:Locale;onOpen:(record:CatalogRecord)=>void}){
 const t=preparationCopy[locale],styles=useMemo(()=>runeStyles(records),[records]);
 const [exploring,setExploring]=useState<[string,string]|null>(null);
 const equipped=[String(page?.primaryStyleId??''),String(page?.subStyleId??'')];
 const browsing=activeRuneExploration(exploring,!!page&&!!sourceLabel,!!editor);
 const trees=browsing??displayedRuneStyles(styles,page);
 const unknown=page&&(displayedRuneStyles(styles,page).includes('')||page.selectedPerkIds.some(id=>id!==0&&!records.some(record=>record.id===String(id)&&['rune','rune_shard'].includes(record.kind))));
 return <section className="surface preparation-panel rune-panel"><header><div><h2>{t.runes}</h2><small title={!page?t.missingPage:undefined}>{page&&!browsing?(sourceLabel??t.equipped):t.browse}</small></div>{page&&browsing&&<button className="icon-button" onClick={()=>setExploring(null)} aria-label={returnLabel??t.returnPage}><Icon name="back" size={17}/></button>}{onExpand&&<button className="icon-button" aria-label={t.expand} onClick={event=>{event.currentTarget.focus({preventScroll:true});onExpand()}}><Icon name="expand" size={16}/></button>}</header>
 <div className="preparation-scroll">
  {page&&(!page.isValid||unknown)&&<p className="catalog-note">{!page.isValid?(incompleteLabel??editor?.incompleteLabel??t.incomplete):t.unknownPage}</p>}
  {page&&(page.isTemporary||page.autoModifiedSelections.length>0)&&<p className="catalog-note">{page.isTemporary?t.temporary:t.modified}</p>}
  <div className="live-rune-trees">{trees.map((styleId,index)=><section className="live-rune-tree" key={index}>
   <label className="rune-style-picker">{styles.find(s=>s.id===styleId)&&<CatalogIcon record={styles.find(s=>s.id===styleId)!}/>}<span><small>{index===0?t.primary:t.secondary}</small><select disabled={editor?editor.disabled:!!sourceLabel&&!!page} aria-label={index===0?t.primary:t.secondary} value={styleId} onChange={event=>{if(editor){editor.onStyle(index===0?'primary':'secondary',Number(event.target.value));return}const next:[string,string]=[trees[0],trees[1]];next[index]=event.target.value;setExploring(next)}}>{!styleId&&<option value="">{t.unavailableTree}</option>}{styles.map(style=><option key={style.id} value={style.id} disabled={!!editor&&index===1&&Number(style.id)===page?.primaryStyleId}>{style.name}</option>)}</select></span></label>
   <div className="live-rune-rows">{runeRows(records,styleId,index===1).map((row,slot)=><div className={`live-rune-row ${index===0&&slot===0?'keystones':''}`} key={slot}>{row.map(rune=><CatalogButton key={rune.id} record={rune} locale={locale} selectedLabel={sourceLabel} selected={equipped[index]===styleId&&!!page?.selectedPerkIds.slice(index===0?0:4,index===0?4:6).includes(Number(rune.id))} disabled={editor?.disabled} actionLabel={editor?.chooseLabel} onOpen={editor?record=>editor.onChoose(index===0?'primary':'secondary',index===0?slot:slot+1,Number(record.id)):onOpen}/>)}</div>)}</div>
   {index===1&&<div className="live-shards" aria-label={t.shards}>{shardRows(records,trees[0]).map((row,slot)=><div className="live-rune-row" key={slot}>{row.map(shard=><CatalogButton key={shard.id} record={shard} locale={locale} selectedLabel={sourceLabel} selected={page?.selectedPerkIds[6+slot]===Number(shard.id)} disabled={editor?.disabled} actionLabel={editor?.chooseLabel} onOpen={editor?record=>editor.onChoose('shard',slot,Number(record.id)):onOpen}/>)}</div>)}</div>}
  </section>)}</div>
 </div></section>;
}
export function ExpandedRunes({records,page,locale,onOpen,onClose,sourceLabel,returnLabel}:{sourceLabel?:string;returnLabel?:string;records:CatalogRecord[];page:RunePage|null;locale:Locale;onOpen:(record:CatalogRecord)=>void;onClose:()=>void}){
 const dialog=useRef<HTMLDialogElement>(null),[trigger]=useState(()=>document.activeElement instanceof HTMLElement?document.activeElement:null);
 useEffect(()=>{dialog.current?.showModal();return()=>{dialog.current?.close();if(trigger?.isConnected)trigger.focus({preventScroll:true})}},[trigger]);
 return createPortal(<dialog ref={dialog} className="expanded-runes" aria-label={preparationCopy[locale].runes} onCancel={event=>{event.preventDefault();onClose()}}><button className="icon-button expanded-close" onClick={onClose} aria-label={preparationCopy[locale].close}><Icon name="close"/></button><RunePanel records={records} page={page} locale={locale} onOpen={onOpen} sourceLabel={sourceLabel} returnLabel={returnLabel}/></dialog>,document.body);
}
export function ItemPanel({records,locale,onOpen}:{records:CatalogRecord[];locale:Locale;onOpen:(record:CatalogRecord)=>void}){
 const t=preparationCopy[locale],[query,setQuery]=useState(''),[limit,setLimit]=useState(40);
 const items=useMemo(()=>searchableItems(records,query),[records,query]);
 return <section className="surface preparation-panel item-panel"><header><div><h2>{t.items}</h2><small>{t.map}</small></div><Icon name="sword" size={18}/></header>
  <label className="item-search"><Icon name="search" size={16}/><input value={query} onChange={event=>{setQuery(event.target.value);setLimit(40)}} placeholder={t.search} aria-label={t.search}/>{query&&<button className="icon-button" aria-label={t.clearSearch} onClick={()=>{setQuery('');setLimit(40)}}><Icon name="close" size={14}/></button>}</label>
  <div className="preparation-scroll"><div className="item-catalog-grid">{items.slice(0,limit).map(item=><CatalogButton key={item.id} className="item-catalog-card" record={item} locale={locale} onOpen={onOpen}><span>{item.name}</span></CatalogButton>)}</div>{!items.length&&<p role="status" className="catalog-note">{t.empty}</p>}{limit<items.length&&<button className="catalog-more" onClick={()=>setLimit(n=>n+40)}>{t.more}<Icon name="chevron" size={14}/></button>}</div>
  <small className="catalog-footer">{t.catalogHint}</small>
 </section>;
}
export function PreparationPanels({page,locale,draft=null,connected=false}:{connected?:boolean;page:RunePage|null;locale:Locale;draft?:DraftSession|null}){
 const t=preparationCopy[locale],[catalog,setCatalog]=useState<PreparationCatalog|null>(null),[error,setError]=useState(false),[attempt,setAttempt]=useState(0),[detail,setDetail]=useState<CatalogRecord|null>(null);
 useEffect(()=>{
  let active=true;setCatalog(null);setError(false);setDetail(null);
  loadCatalog(locale).then(data=>{if(active)setCatalog(data)},()=>{if(active)setError(true)});
  return()=>{active=false};
 },[locale,attempt]);
 if(!catalog)return <div className="preparation-state surface" role="status"><p>{error?t.error:t.loading}</p>{error&&<button className="button" onClick={()=>setAttempt(n=>n+1)}>{t.retry}</button>}</div>;
 return <><BuildPreparation connected={connected} draft={draft} catalog={catalog} equipped={page} locale={locale} onOpen={setDetail}/>{detail&&<GameDetails record={detail} records={catalog.records} locale={locale} version={catalog.version} onClose={()=>setDetail(null)} onOpen={setDetail}/>}</>;
}
