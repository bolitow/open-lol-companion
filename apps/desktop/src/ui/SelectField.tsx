import {useEffect,useId,useLayoutEffect,useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import {Icon} from './Icon';
import {selectIndex} from './selectNavigation';
import {usePresence} from './usePresence';
import './selectField.css';
export type SelectOption={value:string;label:string;disabled?:boolean};
export type SelectFieldProps={id?:string;label:string;value:string;options:readonly SelectOption[];onChange:(value:string)=>void;disabled?:boolean;className?:string;placeholder?:string};
export function SelectField({id:controlId,label,value,options,onChange,disabled=false,className='',placeholder='—'}:SelectFieldProps){
 const id=useId(),trigger=useRef<HTMLButtonElement>(null),popup=useRef<HTMLDivElement>(null);
 const [open,setOpen]=useState(false),[active,setActive]=useState(0),[position,setPosition]=useState({left:0,top:0,width:180,maxHeight:300});
 const unavailable=disabled||!options.some(option=>!option.disabled),presence=usePresence(open&&!unavailable?true:null);
 const selected=options.findIndex(option=>option.value===value),excluded=options.flatMap((option,index)=>option.disabled?[index]:[]);
 const search=useRef({text:'',time:0});
 const show=()=>{if(unavailable)return;trigger.current?.focus({preventScroll:true});setActive(selected>=0&&!options[selected]?.disabled?selected:selectIndex(0,'Home',options.length,excluded)??0);search.current={text:'',time:0};setOpen(true)};
 const choose=(index:number)=>{if(unavailable||!options[index]||options[index].disabled)return;onChange(options[index].value);setOpen(false);trigger.current?.focus({preventScroll:true})};
 useLayoutEffect(()=>{
  if(!open||!trigger.current)return;
  const rect=trigger.current.getBoundingClientRect(),width=Math.min(Math.max(rect.width,180),window.innerWidth-24);
  const below=window.innerHeight-rect.bottom-18,above=rect.top-18;
  const height=Math.max(38,Math.min(300,Math.max(below,above))),actual=Math.min(height,options.length*38+12);
  setPosition({left:Math.max(12,Math.min(rect.left,window.innerWidth-width-12)),top:below>=actual?rect.bottom+6:Math.max(12,rect.top-actual-6),width,maxHeight:height});
 },[open,options.length]);
 useLayoutEffect(()=>{if(open)popup.current?.querySelector<HTMLElement>(`[data-index="${active}"]`)?.scrollIntoView({block:'nearest'})},[open,active]);
 useEffect(()=>{if(unavailable)setOpen(false)},[unavailable]);
 useEffect(()=>{
  if(!open)return;
  const outside=(event:Event)=>{if(event.target instanceof Node&&!popup.current?.contains(event.target)&&!trigger.current?.contains(event.target))setOpen(false)};
  const close=()=>setOpen(false);
  document.addEventListener('pointerdown',outside);document.addEventListener('scroll',outside,true);window.addEventListener('resize',close);
  return()=>{document.removeEventListener('pointerdown',outside);document.removeEventListener('scroll',outside,true);window.removeEventListener('resize',close)};
 },[open]);
 return <><button id={controlId} ref={trigger} type="button" disabled={unavailable} className={`select-field ${className}`} role="combobox" aria-label={label} aria-expanded={open&&!unavailable} aria-haspopup="listbox" aria-controls={open?id:undefined} aria-activedescendant={open&&options[active]?`${id}-${active}`:undefined} onBlur={event=>{if(!popup.current?.contains(event.relatedTarget))setOpen(false)}} onClick={()=>open?setOpen(false):show()} onKeyDown={event=>{
  if(event.key==='Escape'){if(open){event.preventDefault();event.stopPropagation();setOpen(false)}return}
  if(event.key==='Tab'){setOpen(false);return}
  if(event.key==='Enter'||event.key===' '){event.preventDefault();if(open)choose(active);else show();return}
  const next=selectIndex(open?active:selected,event.key,options.length,excluded);
  if(next!==null){event.preventDefault();setActive(next);setOpen(true);return}
  if(event.key.length===1&&!event.ctrlKey&&!event.metaKey&&!event.altKey){
   const now=Date.now();search.current={text:(now-search.current.time<600?search.current.text:'')+event.key.toLocaleLowerCase(),time:now};
   const found=options.findIndex(option=>!option.disabled&&option.label.toLocaleLowerCase().startsWith(search.current.text));
   if(found>=0){event.preventDefault();setActive(found);setOpen(true)}
  }
 }}><span>{options[selected]?.label??placeholder}</span><span className="select-chevron"><Icon name="chevron" size={14}/></span></button>
 {presence.present&&createPortal(<div ref={popup} id={id} role="listbox" aria-label={label} aria-hidden={!open} inert={!open} className="select-popup motion-surface" data-state={presence.closing?'closed':'open'} style={position}>{options.map((option,index)=><div key={option.value} id={`${id}-${index}`} role="option" aria-disabled={option.disabled||undefined} aria-selected={option.value===value} data-index={index} className={active===index&&!option.disabled?'is-active':''} onPointerMove={()=>{if(!option.disabled)setActive(index)}} onPointerDown={event=>event.preventDefault()} onClick={()=>choose(index)}><span>{option.label}</span>{option.value===value&&<Icon name="check" size={14}/>}</div>)}</div>,trigger.current?.closest('dialog')??document.body)}</>;
}
