import {usePresence} from '../ui/usePresence';
import {useEffect,useId,useRef,useState} from 'react';
import {Icon} from '../ui/Icon';
import type {Copy} from './copy';
import {validProfileIcon,type RememberedPlayer} from './playerStore';
import './accountControl.css';
export function accountStatus({native,error,revision,connected}:{native:boolean;error:boolean;revision:number;connected:boolean}) {
 return error?'error':!native?'browser':revision<0?'checking':connected?'connected':'waiting';
}
type Status=ReturnType<typeof accountStatus>;
// Images publiques Data Dragon ; aucune requête au client local depuis React.
// https://developer.riotgames.com/docs/lol#data-dragon_other
const PROFILE_ICON_VERSION='16.19.1';
export const profileIconUrl=(id:unknown)=>validProfileIcon(id)?`https://ddragon.leagueoflegends.com/cdn/${PROFILE_ICON_VERSION}/img/profileicon/${id}.png`:null;
function Avatar({url}:{url:string|null}) {
 const [failed,setFailed]=useState(false);
 return url&&!failed?<img src={url} alt="" width="38" height="38" referrerPolicy="no-referrer" onError={()=>setFailed(true)}/>:<Icon name="user" size={22}/>;
}
export function AccountControl({t,account,accountIsActive=false,status,phase,onProfile,onSession,onRetry}:{t:Copy;account:RememberedPlayer|null;accountIsActive?:boolean;status:Status;phase:string|null;onProfile:()=>void;onSession:()=>void;onRetry:()=>void}) {
 const [open,setOpen]=useState(false),root=useRef<HTMLDivElement>(null),trigger=useRef<HTMLButtonElement>(null),id=useId();
 const presence=usePresence(open?true:null);
 const name=account?`${account.game_name}#${account.tag_line}`:t.account.empty,url=profileIconUrl(account?.profile_icon_id);
 const label=status==='waiting'&&account?t.connection.offline:t.connection[status];
 useEffect(()=>{
  if(!open)return;
  const outside=(event:PointerEvent)=>{if(event.target instanceof Node&&!root.current?.contains(event.target))setOpen(false)};
  const escape=(event:KeyboardEvent)=>{if(event.key==='Escape'){event.stopPropagation();setOpen(false);trigger.current?.focus()}};
  document.addEventListener('pointerdown',outside);document.addEventListener('keydown',escape);
  return()=>{document.removeEventListener('pointerdown',outside);document.removeEventListener('keydown',escape)};
 },[open]);
 const act=(action:()=>void)=>{setOpen(false);action()};
 return <div ref={root} className="account-control" onBlur={event=>{if(!event.currentTarget.contains(event.relatedTarget))setOpen(false)}}>
  <button ref={trigger} className="account-trigger" aria-label={`${name} · ${label}`} title={`${name} · ${label}`} aria-expanded={open} aria-controls={id} onClick={()=>setOpen(value=>!value)}><Avatar key={url} url={url}/><span className={`account-dot account-dot-${status}`} aria-hidden="true"/></button>
  <span className="account-sr-status" role="status">{label}</span>
  <section id={id} hidden={!presence.present} inert={!open} aria-hidden={!open} data-state={presence.closing?'closed':'open'} className="account-panel motion-surface" aria-label={t.account.title}>
   <div className="account-heading"><strong>{name}</strong>{account&&<small>{account.platform} · {accountIsActive?t.account.active:t.account.remembered}</small>}</div>
   <div className="account-service"><span className={`account-dot account-dot-${status}`} aria-hidden="true"/><strong>{label}</strong></div>
   <p>{phase??t.connection[`${status}Hint`]}</p>
   {status==='error'&&<button className="account-action" onClick={()=>act(()=>{onRetry();trigger.current?.focus()})}><Icon name="replay" size={16}/>{t.connection.retry}</button>}
   {account&&<button className="account-action" onClick={()=>act(onProfile)}><Icon name="user" size={16}/>{t.account.profile}<Icon name="arrow" size={16}/></button>}
   {status==='connected'&&phase&&<button className="account-action" onClick={()=>act(onSession)}><Icon name="sword" size={16}/>{t.shell.current}<Icon name="arrow" size={16}/></button>}
  </section>
 </div>;
}
