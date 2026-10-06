import {publicClientPatch} from './clientPatch';
import {useBuildPatch} from './useBuildPatch';
import {BuildPatchNotice} from './BuildPatchNotice';
import {rankLabel} from './buildRanks';
import {usePreparation} from './PreparationContext';
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
    fr: {details:'Fonctionnement et source',invalid:'Choisissez un entier entre 1 et 1 000.',active:'Activé',role:'Poste en personnalisée',chooseRole:'Choisir un poste',source:'En personnalisée : statistiques Solo/Duo au poste choisi.',singlePage:'Une seule page Open LoL Companion est réutilisée pour tous vos champions. Vos autres pages sont conservées.',title:'Imports au prépick',hint:'Dès votre prépick dans LoL, importer la variante la plus jouée pour votre champion et votre poste. Vous pouvez ensuite la modifier dans LoL.',runes:'Importer les runes',items:'Importer les objets',spells:'Importer les sorts',flash:'Flash sur',needsFlash:'Choisissez D ou F pour Flash avant cet import.',flashHint:'Position commune aux imports manuels et automatiques, appliquée aux prochains envois. Un import en cours ou déjà effectué reste inchangé.',minimum:'Minimum de parties par variante',sample:'Petit échantillon : taux observé, à interpréter avec prudence.',observed:'Victoires observées',games:'parties',wins:'victoires',itemsHint:'Objets : inventaire final observé, sans ordre d’achat. Runes, objets et sorts ont des échantillons distincts.',waiting:'En attente de votre prépick dans LoL et du poste. En personnalisée, choisissez le poste ci-dessus.',disabled:'Désactivé',loading:'Lecture des statistiques…',importing:'Import en cours…',empty:'Aucune variante valide au seuil choisi pour ce contexte.',confirmed:'Import confirmé par le client',accepted:'Import accepté, relecture non confirmée. Vérifiez dans LoL.',error:'Import indisponible',retry:'Réessayer les imports manquants',catalog:'Catalogue indisponible. Réessayez le chargement.',reload:'Recharger',storage:'Les réglages ne seront pas conservés après fermeture.',development:'Développement : seuil initial de 1 partie. Valeur initiale en production : 100.',scope:'Patch',all:'Échantillon collecté',desktop:'Ouvrez l’application desktop pour activer ces imports.',converted:(n:number)=>`${n} objet${n>1?'s':''} non achetable${n>1?'s':''} remplacé${n>1?'s':''} par l’objet achetable dont ${n>1?'ils découlent':'il découle'}`,dropped:(n:number)=>`${n} objet${n>1?'s':''} sans équivalent achetable retiré${n>1?'s':''} du set envoyé à League`},
    en: {details:'How it works and source',invalid:'Choose a whole number from 1 to 1,000.',active:'Enabled',role:'Role in custom games',chooseRole:'Choose a role',source:'Custom games use Solo/Duo statistics for the selected role.',singlePage:'One Open LoL Companion rune page is reused for every champion. Your other pages are preserved.',title:'Imports on pre-pick',hint:'On pre-pick in LoL, import the most played variant for your champion and role. You can then adjust it in LoL.',runes:'Import runes',items:'Import items',spells:'Import summoner spells',flash:'Flash on',needsFlash:'Choose D or F for Flash before importing.',flashHint:'Shared by manual and automatic imports, applied to future sends. In-flight and completed imports remain unchanged.',minimum:'Minimum games per variant',sample:'Small sample: observed rate, interpret with care.',observed:'Observed win rate',games:'games',wins:'wins',itemsHint:'Items: observed final inventory, without purchase order. Runes, items and spells have separate samples.',waiting:'Waiting for your pre-pick in LoL and role. In custom games, choose the role above.',disabled:'Disabled',loading:'Loading statistics…',importing:'Importing…',empty:'No valid variant meets the selected threshold for this context.',confirmed:'Import confirmed by the client',accepted:'Import accepted, readback not confirmed. Check in LoL.',error:'Import unavailable',retry:'Retry missing imports',catalog:'Catalog unavailable. Retry loading.',reload:'Reload',storage:'Settings will not be saved after closing.',development:'Development: initial threshold of 1 game. Production default: 100.',scope:'Patch',all:'Collected sample',desktop:'Open the desktop app to enable these imports.',converted:(n:number)=>`${n} unpurchasable item${n>1?'s':''} replaced by the purchasable item ${n>1?'they come':'it comes'} from`,dropped:(n:number)=>`${n} item${n>1?'s':''} with no purchasable equivalent removed from the set sent to League`},
} as const;
function errorMessage(error: unknown, locale: Locale): string {
    const errors = buildCopy[locale].errors;
    if (typeof error === 'string' && Object.hasOwn(errors,error)) return errors[error as keyof typeof errors];
    if(error === 'draftContextChanged' || error === 'unsupportedMode' || error === 'importBusy') return runeEditorCopy[locale][error];
    return importErrorMessage(error,locale);
}
export function Progress({progress,locale,label}:{progress:AutoImportProgress;locale:Locale;label:string}) {
    const t = messages[locale], metrics = progress.metrics, adjustments = progress.adjustments;
    return <div className="auto-import-progress"><strong>{label}</strong><span>{progress.status === 'error' ? errorMessage(progress.error,locale) : t[progress.status]}</span>
        {progress.scope&&<small>{progress.scope.platform} · {buildCopy[locale].queues[progress.scope.queue as 420]??progress.scope.queue} · {buildCopy[locale].roles[progress.scope.role]} · {t.scope} {publicClientPatch(progress.scope.patch)??'—'} · {rankLabel(progress.scope.rank,locale)}</small>}
        {metrics && <small>{metrics.games} {t.games}{metrics.wins !== null && <> · {metrics.wins} {t.wins} · {t.observed} : {metrics.observedWinRate?.toLocaleString(locale,{maximumFractionDigits:1})}%</>}{metrics.lowSample && <> · {t.sample}</>}</small>}
        {adjustments?.converted ? <small>{t.converted(adjustments.converted)}</small> : null}{adjustments?.dropped ? <small>{t.dropped(adjustments.dropped)}</small> : null}</div>;
}
interface AutoImportView {
 patchChoice:ReturnType<typeof useBuildPatch>;
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
    const {value:preparation,update:updatePreparation,rankReady=true}=usePreparation();
    const {state}=useSettings(),preferences=autoImportPreferences(state.values),flashSlot=state.values.flashSlot;
    const [catalog,setCatalog] = useState<PreparationCatalog|null>(null), [catalogError,setCatalogError] = useState(false), [attempt,setAttempt] = useState(0);
    const [syncedRole,setSyncedRole]=useState<Role|null|undefined>(undefined);
    const catalogRef = useRef(catalog); catalogRef.current = catalog;
    const controller = useMemo(()=>createAutoImportController({
        prepare:async(target,minGames,slot)=>{
            const current = catalogRef.current;
            if (!current) throw 'unavailable';
            const report = await invoke<BuildReport>('community_builds',{request:target.request});
            return chooseAutoImports(report,target,minGames,current.records,slot);
        },
        send:request=>invoke<AutoImportReceipt>('import_selected_build',{request}),
    }),[]);
    const snapshot = useSyncExternalStore(controller.subscribe,controller.getSnapshot,controller.getSnapshot);
    const patchChoice=useBuildPatch(catalog?.version??'',JSON.stringify([session.connected,session.draftId,session.draft?.allies.find(p=>p.local)?.championId,catalog?.version,preparation.platform,preparation.queue,preparation.rank,preparation.roleOverride,preferences.customRole]));
    const target = catalog && native && rankReady && patchChoice.canImport && syncedRole === (preferences.customRole??null) ? autoImportTarget(session,catalog,locale,preferences.customRole,{...preparation,customRole:preferences.customRole??null}) : null;
    useEffect(()=>{
        let active = true; setCatalogError(false);setCatalog(null);
        void loadCatalog(locale).then(value=>{if(active)setCatalog(value);},()=>{if(active)setCatalogError(true);});
        return ()=>{active=false;};
    },[locale,attempt]);
    useEffect(()=>{if(preparation.customRole!==(preferences.customRole??null))updatePreparation({customRole:preferences.customRole??null});},[preparation.customRole,preferences.customRole,updatePreparation]);
    useEffect(()=>{controller.update(target,preferences,flashSlot);});
    useEffect(()=>{if(!native)return;return syncCustomRole(role=>invoke<void>('live_custom_role',{role}),preferences.customRole??null,setSyncedRole);},[native,preferences.customRole]);
    useEffect(()=>()=>controller.suspend(),[controller]);
    return <Context.Provider value={{patchChoice,native,snapshot,target,retry:controller.retry,catalogError,reload:()=>setAttempt(n=>n+1)}}>{children}</Context.Provider>;
}
export function AutoImportSetting({setting,locale}:{setting:AutoImportSettingKey;locale:Locale}) {
 const engine=useContext(Context),{state,store,desktopState}=useSettings(),v=state.values,t=messages[locale];
 const disabled=!engine?.native||desktopState.pending;
 const kind=setting==='autoRunes'?'runes':setting==='autoItems'?'items':setting==='autoSpells'?'spells':null;
 const progress=kind?engine?.snapshot[kind]:null;
 const toggleKey=setting==='autoRunes'||setting==='autoItems'||setting==='autoSpells'?setting:null;
 return <>
  <div className="setting-control">
   {kind&&toggleKey&&<label className="setting-switch"><input type="checkbox" role="switch" aria-label={t[kind]} disabled={disabled} checked={v[toggleKey]} onChange={e=>store.change(toggleKey,e.target.checked)}/><span>{v[toggleKey]?t.active:t.disabled}</span></label>}
   {setting==='autoMinGames'&&<MinimumGames value={v.autoMinGames} disabled={disabled} label={t.minimum} invalid={t.invalid} change={value=>store.change('autoMinGames',value)}/>}
   {setting==='autoCustomRole'&&<SelectField label={t.role} value={v.autoCustomRole??''} disabled={disabled} onChange={value=>store.change('autoCustomRole',value?value as Role:undefined)} options={[{value:"",label:t.chooseRole},...(['TOP','JUNGLE','MIDDLE','BOTTOM','UTILITY'] as const).map(value=>({value,label:buildCopy[locale].roles[value]}))]}/>}
  </div>
  {setting==='autoSpells'&&v.autoSpells&&<fieldset className="auto-import-flash"><legend>{t.flash}</legend>{(['D','F'] as const).map(slot=><button key={slot} aria-label={`${t.flash} ${slot}`} aria-pressed={slot===v.flashSlot} disabled={disabled} onClick={()=>store.change('flashSlot',slot)}>{slot}</button>)}<small>{t.flashHint}</small>{state.flashStorageFailed&&<small role="alert">{t.storage}</small>}</fieldset>}
  {!engine?.native&&<small className="setting-system-note">{t.desktop}</small>}
  {progress&&kind&&progress.status!=='disabled'&&<div className="setting-import-status" role="status"><Progress progress={progress} locale={locale} label={t[kind]}/>{['error','empty'].includes(progress.status)&&<button className="button" disabled={disabled} onClick={engine?.retry}>{t.retry}</button>}</div>}
  {kind&&engine&&<BuildPatchNotice choice={engine.patchChoice} locale={locale}/>}
  {kind&&<details className="setting-import-details"><summary>{t.details}</summary><p>{t.hint}</p><p>{kind==='runes'?t.singlePage:t.itemsHint}</p>{engine?.target&&<p>{engine.target.championName} · {buildCopy[locale].roles[engine.target.request.role]} · {engine.target.request.platform} · {buildCopy[locale].queues[engine.target.request.queue as 400|420|440]} · {t.scope} {publicClientPatch(engine.target.request.patch)??'—'} · {rankLabel(engine.target.request.rank,locale)}</p>}</details>}
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
