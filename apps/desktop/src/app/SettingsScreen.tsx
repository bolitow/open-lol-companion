import {useRef} from 'react';
import {Icon} from '../ui/Icon';
import {useSettings} from './SettingsContext';
import {settingsCopy} from './settingsCopy';
import {searchSettings,type SettingCategory,type SettingKey} from './settingsStore';
import './settings.css';
export interface SettingsView {query:string;category:SettingCategory}
export function SettingsScreen({reduced,view,update}:{reduced:boolean;view:SettingsView;update:(patch:Partial<SettingsView>)=>void}){
 const {state,store}=useSettings(),v=state.values,t=settingsCopy[v.locale],search=useRef<HTMLInputElement>(null);
 const results=searchSettings(view.query,v.locale,view.category);
 const valueLabel=(key:SettingKey)=>key==='theme'?t[v.theme]:key==='locale'?(v.locale==='fr'?'Français':'English'):key==='motion'?(v.motion?t.enabled:t.disabled):v.flashSlot??t.unset;
 const icons={theme:'sun',locale:'language',motion:'sparkles',flashSlot:'flash'} as const;
 return <div className="settings-screen">
  <header className="settings-heading"><div><h1>{t.title}</h1><p>{t.intro}</p></div><Icon name="settings" size={26}/></header>
  <div className="settings-search surface" role="search"><Icon name="search"/><label htmlFor="settings-search">{t.search}</label><input ref={search} id="settings-search" type="search" value={view.query} placeholder={t.placeholder} onChange={e=>update({query:e.target.value})} autoComplete="off" spellCheck={false}/>{view.query&&<button className="icon-button" aria-label={t.clear} onClick={()=>{update({query:''});search.current?.focus()}}><Icon name="close" size={16}/></button>}</div>
  <div className="settings-filters"><div role="group" aria-label={t.categories}>{(['all','app','league'] as const).map(category=><button key={category} aria-pressed={view.category===category} onClick={()=>update({category})}>{t[category]}</button>)}</div><span role="status">{results.length} {results.length===1?t.result:t.results}</span></div>
  <div className="settings-results" role="region" aria-label={t.search} tabIndex={0}>
   {results.map(key=><section className="surface setting-card" key={key} aria-labelledby={`setting-${key}`}>
    <header><span className="setting-symbol"><Icon name={icons[key]} size={21}/></span><div><span className="setting-path">{t.paths[key]}</span><h2 id={`setting-${key}`}>{t.names[key]}</h2></div>{state.recent.slice(0,2).includes(key)&&<span className="setting-recent" title={t.recent}><Icon name="check" size={13}/><span>{t.recent}</span></span>}</header>
    <p>{t.descriptions[key]}</p>
    <div className="setting-control">
     {key==='theme'&&<div className="setting-segmented" role="group" aria-label={t.names.theme}>{(['dark','light'] as const).map(theme=><button key={theme} aria-pressed={v.theme===theme} onClick={()=>store.change('theme',theme)}><Icon name={theme==='dark'?'moon':'sun'} size={17}/>{t[theme]}</button>)}</div>}
     {key==='locale'&&<select id="settings-language" aria-label={t.names.locale} value={v.locale} onChange={e=>store.change('locale',e.target.value==='en'?'en':'fr')}><option value="fr">Français</option><option value="en">English</option></select>}
     {key==='motion'&&<label className="setting-switch"><input type="checkbox" role="switch" aria-label={t.names.motion} checked={v.motion} onChange={e=>store.change('motion',e.target.checked)}/><span>{v.motion?t.enabled:t.disabled}</span></label>}
     {key==='flashSlot'&&<div className="setting-segmented" role="group" aria-label={t.names.flashSlot}>{([null,'D','F'] as const).map(slot=><button key={slot??'unset'} aria-label={slot?`${t.names.flashSlot} ${slot}`:t.unset} aria-pressed={v.flashSlot===slot} onClick={()=>store.change('flashSlot',slot)}>{slot??t.unset}</button>)}</div>}
    </div>
    {key==='motion'&&reduced&&<small className="setting-system-note"><Icon name="info" size={14}/>{t.reduced}</small>}
   </section>)}
   {!results.length&&<div className="settings-empty surface"><Icon name="search" size={28}/><h2>{t.empty}</h2><p>{t.emptyHint}</p><button className="button" onClick={()=>{update({query:'',category:'all'});search.current?.focus()}}>{t.showAll}</button></div>}
  </div>
  <footer className="settings-save surface"><div><p role="status">{state.storageFailed?t.storage:t.saved}</p>{state.lastChange&&<small>{t.changed} : {t.names[state.lastChange.key]} · {valueLabel(state.lastChange.key)}</small>}</div>{state.storageFailed&&<button className="button" onClick={store.retrySave}>{t.retry}</button>}<button className="button" aria-label={t.undoLabel} disabled={!state.lastChange} onClick={store.undo}><Icon name="replay" size={15}/>{t.undo}</button></footer>
 </div>;
}
