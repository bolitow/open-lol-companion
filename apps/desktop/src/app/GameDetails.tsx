import {useEffect,useId,useLayoutEffect,useRef,useState,type ReactNode} from 'react';
import {createPortal} from 'react-dom';
import type {CatalogRecord} from '@olc/shared';
import {formatStat} from './catalogFormat';
import {Icon} from '../ui/Icon';
import {catalogDescription,itemComponents,itemStats,readableValue} from './preparation';
import {preparationCopy,statLabels} from './preparationCopy';
import type {Locale} from './state';

let dismissTooltip: (()=>void)|null=null;

export function CatalogIcon({record}:{record:CatalogRecord}){
 const [failed,setFailed]=useState(false);
 return record.icon&&!failed?<img src={record.icon} alt="" loading="lazy" onError={()=>setFailed(true)}/>:<Icon name="shield" size={24}/>;
}
export function CatalogButton({record,locale,selected=false,selectedLabel,onOpen,children,className='',actionLabel,accessibleLabel,disabled=false}:{accessibleLabel?:string;actionLabel?:string;disabled?:boolean;record:CatalogRecord;locale:Locale;selected?:boolean;selectedLabel?:string;onOpen:(record:CatalogRecord)=>void;children?:ReactNode;className?:string}){
 const ref=useRef<HTMLButtonElement>(null),tip=useRef<HTMLDivElement>(null),timer=useRef<ReturnType<typeof setTimeout>>(undefined),id=useId();
 const [open,setOpen]=useState(false),[position,setPosition]=useState({left:0,top:0});
 const t=preparationCopy[locale];
 const tooltipStats=itemStats(record).filter(stat=>statLabels[stat.key]&&formatStat(stat.value,stat.unit,locale)!==null).slice(0,3);
 const hide=()=>{clearTimeout(timer.current);setOpen(false)};
 const show=()=>{dismissTooltip?.();dismissTooltip=hide;clearTimeout(timer.current);setOpen(true)};
 const scheduleHide=()=>{clearTimeout(timer.current);timer.current=setTimeout(()=>setOpen(false),100)};
 useEffect(()=>()=>{clearTimeout(timer.current);dismissTooltip?.();dismissTooltip=null},[]);
 useLayoutEffect(()=>{
  if(!open||!ref.current||!tip.current)return;
  const anchor=ref.current.getBoundingClientRect(),box=tip.current.getBoundingClientRect();
  setPosition({left:Math.max(12,Math.min(anchor.left+anchor.width/2-box.width/2,window.innerWidth-box.width-12)),top:Math.max(12,anchor.top-box.height-10>=12?anchor.top-box.height-10:Math.min(anchor.bottom+10,window.innerHeight-box.height-12))});
  const escape=(event:KeyboardEvent)=>{if(event.key==='Escape')hide()};
  window.addEventListener('keydown',escape);window.addEventListener('resize',hide);document.addEventListener('scroll',hide,true);
  return()=>{window.removeEventListener('keydown',escape);window.removeEventListener('resize',hide);document.removeEventListener('scroll',hide,true)};
 },[open,record.description]);
 return <><button ref={ref} type="button" className={`catalog-button ${selected?'is-equipped':''} ${className}`} disabled={disabled} aria-pressed={actionLabel?selected:undefined} aria-label={accessibleLabel??`${record.name}${selected?` · ${selectedLabel??t.selected}`:''} · ${actionLabel??t.details}`} aria-describedby={open?id:undefined} onMouseEnter={show} onMouseLeave={scheduleHide} onFocus={show} onBlur={hide} onClick={()=>{ref.current?.focus({preventScroll:true});hide();onOpen(record)}}><CatalogIcon key={record.id} record={record}/>{children}{selected&&<span className="equipped-mark"><Icon name="check" size={10}/></span>}</button>{open&&createPortal(<div ref={tip} id={id} role="tooltip" className="game-tooltip" style={position} onMouseEnter={()=>clearTimeout(timer.current)} onMouseLeave={scheduleHide}><strong>{record.name}</strong><p>{(catalogDescription(record)??t.noDescription).slice(0,230)}{(catalogDescription(record)?.length??0)>230?'…':''}</p><div className="tooltip-stats">{record.kind==='champion'&&tooltipStats.length>0&&<span>{locale==='fr'?'Statistiques de base':'Base stats'}</span>}{tooltipStats.map(stat=><span key={stat.key}><b>{formatStat(stat.value,stat.unit,locale)}</b> {statLabels[stat.key]?.[locale]}</span>)}</div><small>{actionLabel??t.details}</small></div>,ref.current?.closest('dialog')??document.body)}</>;
}
export function GameDetails({record,records,locale,version,onClose,onOpen}:{record:CatalogRecord;records:CatalogRecord[];locale:Locale;version:string;onClose:()=>void;onOpen:(record:CatalogRecord)=>void}){
 const dialog=useRef<HTMLDialogElement>(null),title=useId(),t=preparationCopy[locale];
 const [trigger]=useState(()=>document.activeElement instanceof HTMLElement?document.activeElement:null);
 useEffect(()=>{const node=dialog.current;node?.showModal();return()=>{node?.close();if(trigger?.isConnected)trigger.focus({preventScroll:true})}},[trigger]);
 useEffect(()=>{dialog.current?.querySelector('.game-detail-body')?.scrollTo(0,0);dialog.current?.querySelector<HTMLElement>('h2')?.focus({preventScroll:true})},[record.id]);
 const price=readableValue(record,'price_total'),components=itemComponents(records,record),stats=itemStats(record).filter(stat=>statLabels[stat.key]&&formatStat(stat.value,stat.unit,locale)!==null);
 return createPortal(<dialog ref={dialog} className="game-detail" aria-labelledby={title} onCancel={event=>{event.preventDefault();onClose()}} onClick={event=>{if(event.target===event.currentTarget)onClose()}}>
  <div className="game-detail-shell"><header><CatalogIcon key={record.id} record={record}/><div><small>{t.patch} {version} · {t.readOnly}</small><h2 id={title} tabIndex={-1}>{record.name}</h2>{typeof price==='number'&&<span className="item-price">{new Intl.NumberFormat(locale).format(price)} {t.gold} · {t.price}</span>}</div><button className="icon-button" onClick={onClose} aria-label={t.close}><Icon name="close"/></button></header>
  <div className="game-detail-body"><div className="item-stat-list">{stats.map(stat=><span key={stat.key}><strong>{formatStat(stat.value,stat.unit,locale)}</strong> {statLabels[stat.key]?.[locale]}</span>)}</div><p className="game-description">{catalogDescription(record)??t.noDescription}</p>
   {record.kind==='item'&&<section className="item-components"><h3>{t.components}</h3>{components.length?<div>{components.map(({id,record:component},i)=>component?<button key={`${id}:${i}`} onClick={()=>onOpen(component)}><CatalogIcon record={component}/><span>{component.name}</span><Icon name="chevron" size={14}/></button>:<span key={`${id}:${i}`}>{t.unknownItem} · {id}</span>)}</div>:<p>{t.noComponents}</p>}</section>}
   {record.coverage.issues?.length>0&&<p className="catalog-note"><Icon name="info" size={14}/>{t.partial}</p>}
  </div></div>
 </dialog>,document.body);
}
