import type {DraftPlayer} from '@olc/shared';
import {catalogRuntime} from './catalogRuntime';
import {Icon} from '../ui/Icon';
import {SelectField} from '../ui/SelectField';
import {buildCopy} from './buildCopy';
import {championDetails} from './draft';
import {draftChampionContext,manualChampionSelection} from './draftChampionContext';
import type {Locale} from './state';

export function BuildChampionContext({manual,local,locale,onChange}:{manual:number|null;local?:DraftPlayer;locale:Locale;onChange:(manual:number|null)=>void}){
 const t=buildCopy[locale],context=draftChampionContext(manual,local);
 const champion=championDetails(context.championId||null,locale),own=championDetails(context.returnId,locale),returnName=own?.name??t.unknown;
 const label=context.state==='browsing'?t.manual:context.state==='locked'?t.lockedPick:context.state==='prepick'?t.prepick:t.choose;
 const champions=Object.entries(catalogRuntime.getSnapshot().index).map(([id,entry])=>({value:id,label:entry[locale]})).sort((a,b)=>a.label.localeCompare(b.label,locale));
 return <section className="build-context" data-context={context.state} aria-label={t.title}>
  <span className="build-context-art" aria-hidden="true">{champion?<img key={champion.image} src={champion.image} alt=""/>:<Icon name="user" size={22}/>}</span>
  <div className="build-context-selection"><small className="build-context-status" aria-live="polite">{context.state==='locked'&&<Icon name="check" size={11}/>} {label}</small>
   <SelectField label={t.champion} value={String(context.championId)} onChange={value=>onChange(manualChampionSelection(Number(value),local))} options={[{value:'0',label:t.choose},...(!champion&&context.championId?[{value:String(context.championId),label:t.unknown}]:[]),...champions]}/>
   {context.returnId!==null&&<button className="return-pick" title={`${t.follow} · ${returnName}`} aria-label={`${t.follow} · ${returnName}`} onClick={()=>onChange(null)}><Icon name="back" size={13}/><span>{context.returnLocked?t.lockedPick:t.prepick} · <b>{returnName}</b></span></button>}
  </div>
 </section>;
}
