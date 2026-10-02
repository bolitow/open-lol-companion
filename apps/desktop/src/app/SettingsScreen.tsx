import {DiagnosticsExport} from './DiagnosticsExport';
import {useEffect,useRef} from 'react';
import {Icon} from '../ui/Icon';
import {useSettings} from './SettingsContext';
import {settingsCopy} from './settingsCopy';
import {searchSettings,type SettingCategory,type SettingKey} from './settingsStore';
import './settings.css';
export interface SettingsView {query:string;category:SettingCategory}
export function SettingsScreen({reduced,view,update}:{reduced:boolean;view:SettingsView;update:(patch:Partial<SettingsView>)=>void}){
 const {state,store,desktopState,desktopStore,localeError,retryLocale}=useSettings(),v=state.values,t=settingsCopy[v.locale],search=useRef<HTMLInputElement>(null);
 useEffect(()=>{
  const reload=()=>{void desktopStore.load()};
  reload();window.addEventListener('focus',reload);
  return()=>window.removeEventListener('focus',reload);
 },[desktopStore]);
 const results=searchSettings(view.query,v.locale,view.category);
 const valueLabel=(key:SettingKey)=>key==='theme'?t[v.theme]:key==='locale'?(v.locale==='fr'?'Français':'English'):key==='motion'?(v.motion?t.enabled:t.disabled):v.flashSlot??t.unset;
 const icons={theme:'sun',locale:'language',motion:'sparkles',flashSlot:'flash',closeToTray:'pin',autostartEnabled:'play'} as const;
 const native=desktopState.value;
 const nativeIssue=desktopState.error?t.nativeFailed:native?.storageError?t.nativeStorage:desktopState.native&&native?.autostartEnabled===null?t.nativeUnknown:null;
 const latest=desktopState.lastChange;
 const localBusy=desktopState.pending;
 const undo=()=>{if(latest)void desktopStore.undo();else store.undo()};
 return <div className="settings-screen">
  <header className="settings-heading"><div><h1>{t.title}</h1><p>{t.intro}</p></div><Icon name="settings" size={26}/></header>
  <div className="settings-search surface" role="search"><Icon name="search"/><label htmlFor="settings-search">{t.search}</label><input ref={search} id="settings-search" type="search" value={view.query} placeholder={t.placeholder} onChange={e=>update({query:e.target.value})} autoComplete="off" spellCheck={false}/>{view.query&&<button className="icon-button" aria-label={t.clear} onClick={()=>{update({query:''});search.current?.focus()}}><Icon name="close" size={16}/></button>}</div>
  <div className="settings-filters"><div role="group" aria-label={t.categories}>{(['all','app','league'] as const).map(category=><button key={category} aria-pressed={view.category===category} onClick={()=>update({category})}>{t[category]}</button>)}</div><span role="status">{results.length} {results.length===1?t.result:t.results}</span></div>
  <div className="settings-results" role="region" aria-label={t.search} tabIndex={0}>
   {results.map(key=><section className="surface setting-card" key={key} aria-labelledby={`setting-${key}`}>
    <header><span className="setting-symbol"><Icon name={icons[key]} size={21}/></span><div><span className="setting-path">{t.paths[key]}</span><h2 id={`setting-${key}`}>{t.names[key]}</h2></div>{state.recent.slice(0,2).some(recent=>recent===key)&&<span className="setting-recent" title={t.recent}><Icon name="check" size={13}/><span>{t.recent}</span></span>}</header>
    <p>{t.descriptions[key]}</p>
    <div className="setting-control">
     {key==='theme'&&<div className="setting-segmented" role="group" aria-label={t.names.theme}>{(['dark','light'] as const).map(theme=><button key={theme} aria-pressed={v.theme===theme} disabled={localBusy} onClick={()=>store.change('theme',theme)}><Icon name={theme==='dark'?'moon':'sun'} size={17}/>{t[theme]}</button>)}</div>}
     {key==='locale'&&<select id="settings-language" disabled={localBusy} aria-label={t.names.locale} value={v.locale} onChange={e=>store.change('locale',e.target.value==='en'?'en':'fr')}><option value="fr">Français</option><option value="en">English</option></select>}
     {key==='motion'&&<label className="setting-switch"><input type="checkbox" role="switch" aria-label={t.names.motion} disabled={localBusy} checked={v.motion} onChange={e=>store.change('motion',e.target.checked)}/><span>{v.motion?t.enabled:t.disabled}</span></label>}
     {key==='flashSlot'&&<div className="setting-segmented" role="group" aria-label={t.names.flashSlot}>{([null,'D','F'] as const).map(slot=><button key={slot??'unset'} aria-label={slot?`${t.names.flashSlot} ${slot}`:t.unset} aria-pressed={v.flashSlot===slot} disabled={localBusy} onClick={()=>store.change('flashSlot',slot)}>{slot??t.unset}</button>)}</div>}
     {(key==='closeToTray'||key==='autostartEnabled')&&<NativeControl name={t.names[key]} value={native?.[key]??null} disabled={!desktopState.native||desktopState.pending||!native||(key==='closeToTray'&&!native.trayAvailable)} unknown={t.unknown} enabled={t.nativeEnabled} off={t.nativeDisabled} change={value=>{void desktopStore.change(key,value)}}/>}
    </div>
    {(key==='closeToTray'||key==='autostartEnabled')&&!desktopState.native&&<small className="setting-system-note">{t.desktopOnly}</small>}
    {key==='closeToTray'&&desktopState.native&&native&&!native.trayAvailable&&<small className="setting-system-note">{t.trayUnavailable}</small>}
    {key==='motion'&&reduced&&<small className="setting-system-note"><Icon name="info" size={14}/>{t.reduced}</small>}
   </section>)}
   {!results.length&&<div className="settings-empty surface"><Icon name="search" size={28}/><h2>{t.empty}</h2><p>{t.emptyHint}</p><button className="button" onClick={()=>{update({query:'',category:'all'});search.current?.focus()}}>{t.showAll}</button></div>}
  </div>
  <footer className="settings-save surface"><div><p role="status">{desktopState.pending?t.nativePending:nativeIssue??(state.storageFailed?t.storage:localeError?t.localeFailed:t.saved)}</p>{(latest||state.lastChange)&&<small>{t.changed} : {latest?`${t.names[latest.key]} · ${latest.applied?t.nativeEnabled:t.nativeDisabled}`:state.lastChange?`${t.names[state.lastChange.key]} · ${valueLabel(state.lastChange.key)}`:''}</small>}</div>{state.storageFailed&&<button className="button" onClick={store.retrySave}>{t.retry}</button>}{(nativeIssue||(desktopState.native&&native?.autostartEnabled===null))&&<button className="button" disabled={desktopState.pending} onClick={()=>{void desktopStore.load()}}>{t.nativeRetry}</button>}{localeError&&<button className="button" disabled={desktopState.pending} onClick={retryLocale}>{t.localeRetry}</button>}<DiagnosticsExport locale={v.locale}/><button className="button" aria-label={t.undoLabel} disabled={desktopState.pending||(!state.lastChange&&!latest)} onClick={undo}><Icon name="replay" size={15}/>{t.undo}</button></footer>
 </div>;
}

export function NativeControl({name,value,disabled,unknown,enabled,off,change}:{name:string;value:boolean|null;disabled:boolean;unknown:string;enabled:string;off:string;change:(value:boolean)=>void}){
 if(value===null)return <span className="setting-unknown" role="status">{unknown}</span>;
 return <label className="setting-switch"><input type="checkbox" role="switch" aria-label={name} disabled={disabled} checked={value} onChange={event=>change(event.target.checked)}/><span>{value?enabled:off}</span></label>;
}
