import {useId,useState,type ReactNode} from 'react';
import {Icon} from './Icon';
import './surfaceMotion.css';
export function Disclosure({label,children,className=''}:{label:ReactNode;children:ReactNode;className?:string}){
 const [open,setOpen]=useState(false),id=useId();
 return <section className={`motion-disclosure ${className}`}><button type="button" aria-expanded={open} aria-controls={id} onClick={()=>setOpen(value=>!value)}><Icon name="chevron" size={12}/>{label}</button><div id={id} className="motion-disclosure-content" data-open={open} aria-hidden={!open} inert={!open}><div>{children}</div></div></section>;
}
