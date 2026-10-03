import {SelectField} from '../ui/SelectField';
import {createContext, useContext, useEffect, useMemo, useRef, useState, useSyncExternalStore, type ReactNode} from 'react';
import {syncCustomRole} from './customRoleSync';
import {invoke} from '@tauri-apps/api/core';
import {importErrorMessage, type AutoImportReceipt, type BuildReport, type Role, type LcuSession} from '@olc/shared';
import {autoImportTarget, chooseAutoImports, createAutoImportController, type AutoImportProgress, type AutoImportSnapshot, type AutoImportTarget} from './autoImport';
import {loadCatalog, type PreparationCatalog} from './catalog';
import {runeEditorCopy} from './runeEditorCopy';
import {buildCopy} from './buildCopy';
import type {Locale} from './state';

import {useSettings} from './SettingsContext';
import {autoImportPreferences,type AutoImportSettingKey} from './settingsStore';
const messages = {
    fr: {details:'Fonctionnement et source',invalid:'Choisissez un entier entre 1 et 1 000.',active:'Activé',role:'Poste en personnalisée',chooseRole:'Choisir un poste',source:'En personnalisée : statistiques Solo/Duo au poste choisi.',singlePage:'Une seule page Open LoL Companion est réutilisée pour tous vos champions. Vos autres pages sont conservées.',title:'Imports au prépick',hint:'Dès votre prépick dans LoL, importer la variante la plus jouée pour votre champion et votre poste. Vous pouvez ensuite la modifier dans LoL. Les deux sorts d’invocateur restent inchangés.',runes:'Importer les runes',items:'Importer les objets',minimum:'Minimum de parties par variante',sample:'Petit échantillon : taux observé, à interpréter avec prudence.',observed:'Victoires observées',games:'parties',wins:'victoires',itemsHint:'Objets : inventaire final observé, sans ordre d’achat. Runes et objets ont des échantillons distincts.',waiting:'En attente de votre prépick dans LoL et du poste. En personnalisée, choisissez le poste ci-dessus.',disabled:'Désactivé',loading:'Lecture des statistiques…',importing:'Import en cours…',empty:'Aucune variante valide au seuil choisi pour ce contexte.',confirmed:'Import confirmé par le client',accepted:'Import accepté, relecture non confirmée. Vérifiez dans LoL.',error:'Import indisponible',retry:'Réessayer les imports manquants',catalog:'Catalogue indisponible. Réessayez le chargement.',reload:'Recharger',storage:'Les réglages ne seront pas conservés après fermeture.',development:'Développement : seuil initial de 1 partie. Valeur initiale en production : 100.',scope:'Patch',all:'Tous les rangs',desktop:'Ouvrez l’application desktop pour activer ces imports.'},
    en: {details:'How it works and source',invalid:'Choose a whole number from 1 to 1,000.',active:'Enabled',role:'Role in custom games',chooseRole:'Choose a role',source:'Custom games use Solo/Duo statistics for the selected role.',singlePage:'One Open LoL Companion rune page is reused for every champion. Your other pages are preserved.',title:'Imports on pre-pick',hint:'On pre-pick in LoL, import the most played variant for your champion and role. You can then adjust it in LoL. Both summoner spells remain unchanged.',runes:'Import runes',items:'Import items',minimum:'Minimum games per variant',sample:'Small sample: observed rate, interpret with care.',observed:'Observed win rate',games:'games',wins:'wins',itemsHint:'Items: observed final inventory, without purchase order. Runes and items have separate samples.',waiting:'Waiting for your pre-pick in LoL and role. In custom games, choose the role above.',disabled:'Disabled',loading:'Loading statistics…',importing:'Importing…',empty:'No valid variant meets the selected threshold for this context.',confirmed:'Import confirmed by the client',accepted:'Import accepted, readback not confirmed. Check in LoL.',error:'Import unavailable',retry:'Retry missing imports',catalog:'Catalog unavailable. Retry loading.',reload:'Reload',storage:'Settings will not be saved after closing.',development:'Development: initial threshold of 1 game. Production default: 100.',scope:'Patch',all:'All ranks',desktop:'Open the desktop app to enable these imports.'},
} as const;
function errorMessage(error: unknown, locale: Locale): string {
    const errors = buildCopy[locale].errors;
    if (typeof error === 'string' && Object.hasOwn(errors,error)) return errors[error as keyof typeof errors];
    if(error === 'draftContextChanged' || error === 'unsupportedMode' || error === 'importBusy') return runeEditorCopy[locale][error];
    return importErrorMessage(error,locale);
}
function Progress({progress,locale,label}:{progress:AutoImportProgress;locale:Locale;label:string}) {
    const t = messages[locale], metrics = progress.metrics;
    return <div className="auto-import-progress"><strong>{label}</strong><span>{progress.status === 'error' ? errorMessage(progress.error,locale) : t[progress.status]}</span>
        {metrics && <small>{metrics.games} {t.games}{metrics.wins !== null && <> · {metrics.wins} {t.wins} · {t.observed} : {metrics.observedWinRate?.toLocaleString(locale,{maximumFractionDigits:1})}%</>}{metrics.lowSample && <> · {t.sample}</>}</small>}</div>;
}
interface AutoImportView {
 native:boolean;
 snapshot:AutoImportSnapshot;
 target:AutoImportTarget|null;
 retry:()=>void;
 catalogError:boolean;
 reload:()=>void;
}
const Context=createContext<AutoImportView|null>(null);
/** Le moteur reste monté au-dessus des pages : ouvrir les réglages ne relance aucun import. */
export function AutoImportProvider({session,locale,native,children}:{session:LcuSession;locale:Locale;native:boolean;children:ReactNode}) {
    const {state}=useSettings(),preferences=autoImportPreferences(state.values);
    const [catalog,setCatalog] = useState<PreparationCatalog|null>(null), [catalogError,setCatalogError] = useState(false), [attempt,setAttempt] = useState(0);
    const [syncedRole,setSyncedRole]=useState<Role|null|undefined>(undefined);
    const catalogRef = useRef(catalog); catalogRef.current = catalog;
    const controller = useMemo(()=>createAutoImportController({
        prepare:async(target,minGames)=>{
            const current = catalogRef.current;
            if (!current) throw 'unavailable';
            const report = await invoke<BuildReport>('community_builds',{request:target.request});
            return chooseAutoImports(report,target,minGames,current.records);
        },
        send:request=>invoke<AutoImportReceipt>('import_selected_build',{request}),
    }),[]);
    const snapshot = useSyncExternalStore(controller.subscribe,controller.getSnapshot,controller.getSnapshot);
    const target = catalog && native && syncedRole === (preferences.customRole??null) ? autoImportTarget(session,catalog,locale,preferences.customRole) : null;
    useEffect(()=>{
        let active = true; setCatalogError(false);setCatalog(null);
        void loadCatalog(locale).then(value=>{if(active)setCatalog(value);},()=>{if(active)setCatalogError(true);});
        return ()=>{active=false;};
    },[locale,attempt]);
    useEffect(()=>{controller.update(target,preferences);});
    useEffect(()=>{if(!native)return;return syncCustomRole(role=>invoke<void>('live_custom_role',{role}),preferences.customRole??null,setSyncedRole);},[native,preferences.customRole]);
    useEffect(()=>()=>controller.suspend(),[controller]);
    return <Context.Provider value={{native,snapshot,target,retry:controller.retry,catalogError,reload:()=>setAttempt(n=>n+1)}}>{children}</Context.Provider>;
}
export function AutoImportSetting({setting,locale}:{setting:AutoImportSettingKey;locale:Locale}) {
 const engine=useContext(Context),{state,store,desktopState}=useSettings(),v=state.values,t=messages[locale];
 const disabled=!engine?.native||desktopState.pending;
 const kind=setting==='autoRunes'?'runes':setting==='autoItems'?'items':null;
 const progress=kind?engine?.snapshot[kind]:null;
 return <>
  <div className="setting-control">
   {kind&&<label className="setting-switch"><input type="checkbox" role="switch" aria-label={t[kind]} disabled={disabled} checked={kind==='runes'?v.autoRunes:v.autoItems} onChange={e=>store.change(kind==='runes'?'autoRunes':'autoItems',e.target.checked)}/><span>{(kind==='runes'?v.autoRunes:v.autoItems)?t.active:t.disabled}</span></label>}
   {setting==='autoMinGames'&&<MinimumGames value={v.autoMinGames} disabled={disabled} label={t.minimum} invalid={t.invalid} change={value=>store.change('autoMinGames',value)}/>}
   {setting==='autoCustomRole'&&<SelectField label={t.role} value={v.autoCustomRole??''} disabled={disabled} onChange={value=>store.change('autoCustomRole',value?value as Role:undefined)} options={[{value:"",label:t.chooseRole},...(['TOP','JUNGLE','MIDDLE','BOTTOM','UTILITY'] as const).map(value=>({value,label:buildCopy[locale].roles[value]}))]}/>}
  </div>
  {!engine?.native&&<small className="setting-system-note">{t.desktop}</small>}
  {progress&&kind&&progress.status!=='disabled'&&<div className="setting-import-status" role="status"><Progress progress={progress} locale={locale} label={t[kind]}/>{['error','empty'].includes(progress.status)&&<button className="button" disabled={disabled} onClick={engine?.retry}>{t.retry}</button>}</div>}
  {kind&&<details className="setting-import-details"><summary>{t.details}</summary><p>{t.hint}</p><p>{kind==='runes'?t.singlePage:t.itemsHint}</p>{engine?.target&&<p>{engine.target.championName} · {buildCopy[locale].roles[engine.target.request.role]} · {engine.target.request.platform} · {buildCopy[locale].queues[engine.target.request.queue as 400|420|440]} · {t.scope} {engine.target.request.patch} · {t.all}</p>}</details>}
  {setting==='autoMinGames'&&import.meta.env.DEV&&<small className="setting-system-note">{t.development}</small>}
  {setting==='autoCustomRole'&&<small className="setting-system-note">{t.source}</small>}
  {kind&&engine?.catalogError&&<p className="setting-system-note" role="alert">{t.catalog}<button className="button" onClick={engine.reload}>{t.reload}</button></p>}
 </>;
}
function MinimumGames({value,disabled,label,invalid,change}:{value:number;disabled:boolean;label:string;invalid:string;change:(value:number)=>void}) {
 const [draft,setDraft]=useState(String(value)),[error,setError]=useState(false);
 useEffect(()=>{setDraft(String(value));setError(false)},[value]);
 const commit=()=>{const number=Number(draft);if(!draft.trim()||!Number.isInteger(number)||number<1||number>1000){setError(true);return}setError(false);change(number)};
 return <div><input aria-label={label} aria-invalid={error} type="number" min={1} max={1000} step={1} value={draft} disabled={disabled} onChange={e=>{setDraft(e.target.value);setError(false)}} onBlur={commit} onKeyDown={e=>{if(e.key==='Enter')e.currentTarget.blur();if(e.key==='Escape'){setDraft(String(value));setError(false)}}}/>{error&&<small role="alert">{invalid}</small>}</div>;
}
