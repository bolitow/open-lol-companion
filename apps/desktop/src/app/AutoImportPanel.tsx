import {syncCustomRole} from './customRoleSync';
import {useEffect, useMemo, useRef, useState, useSyncExternalStore} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {importErrorMessage, type AutoImportReceipt, type BuildReport, type Role, type LcuSession} from '@olc/shared';
import {autoImportTarget, chooseAutoImports, createAutoImportController, parseAutoImportPreferences, type AutoImportProgress} from './autoImport';
import {loadCatalog, type PreparationCatalog} from './catalog';
import {runeEditorCopy} from './runeEditorCopy';
import {buildCopy} from './buildCopy';
import type {Locale} from './state';

const storageKey = 'olc.auto-import.preferences.v1';
const messages = {
    fr: {active:'Activé',role:'Poste en personnalisée',chooseRole:'Choisir un poste',source:'En personnalisée : statistiques Solo/Duo au poste choisi.',singlePage:'Une seule page Open LoL Companion est réutilisée pour tous vos champions. Vos autres pages sont conservées.',title:'Imports au prépick',hint:'Dès votre prépick dans LoL, importer la variante la plus jouée pour votre champion et votre poste. Vous pouvez ensuite la modifier dans LoL. Les deux sorts d’invocateur restent inchangés.',runes:'Importer les runes',items:'Importer les objets',minimum:'Minimum de parties par variante',sample:'Petit échantillon : taux observé, à interpréter avec prudence.',observed:'Victoires observées',games:'parties',wins:'victoires',itemsHint:'Objets : inventaire final observé, sans ordre d’achat. Runes et objets ont des échantillons distincts.',waiting:'En attente de votre prépick dans LoL et du poste. En personnalisée, choisissez le poste ci-dessus.',disabled:'Désactivé',loading:'Lecture des statistiques…',importing:'Import en cours…',empty:'Aucune variante valide au seuil choisi pour ce contexte.',confirmed:'Import confirmé par le client',accepted:'Import accepté, relecture non confirmée. Vérifiez dans LoL.',error:'Import indisponible',retry:'Réessayer les imports manquants',catalog:'Catalogue indisponible. Réessayez le chargement.',reload:'Recharger',storage:'Les réglages ne seront pas conservés après fermeture.',development:'Développement : seuil initial de 1 partie. Valeur initiale en production : 100.',scope:'Patch',all:'Tous les rangs',desktop:'Ouvrez l’application desktop pour activer ces imports.'},
    en: {active:'Enabled',role:'Role in custom games',chooseRole:'Choose a role',source:'Custom games use Solo/Duo statistics for the selected role.',singlePage:'One Open LoL Companion rune page is reused for every champion. Your other pages are preserved.',title:'Imports on pre-pick',hint:'On pre-pick in LoL, import the most played variant for your champion and role. You can then adjust it in LoL. Both summoner spells remain unchanged.',runes:'Import runes',items:'Import items',minimum:'Minimum games per variant',sample:'Small sample: observed rate, interpret with care.',observed:'Observed win rate',games:'games',wins:'wins',itemsHint:'Items: observed final inventory, without purchase order. Runes and items have separate samples.',waiting:'Waiting for your pre-pick in LoL and role. In custom games, choose the role above.',disabled:'Disabled',loading:'Loading statistics…',importing:'Importing…',empty:'No valid variant meets the selected threshold for this context.',confirmed:'Import confirmed by the client',accepted:'Import accepted, readback not confirmed. Check in LoL.',error:'Import unavailable',retry:'Retry missing imports',catalog:'Catalog unavailable. Retry loading.',reload:'Reload',storage:'Settings will not be saved after closing.',development:'Development: initial threshold of 1 game. Production default: 100.',scope:'Patch',all:'All ranks',desktop:'Open the desktop app to enable these imports.'},
} as const;
function initialPreferences() {
    try {return parseAutoImportPreferences(localStorage.getItem(storageKey), import.meta.env.DEV);}
    catch {return parseAutoImportPreferences(null, import.meta.env.DEV);}
}
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
/** Le moteur reste monté pendant la navigation ; ses réglages apparaissent seulement dans les paramètres. */
export function AutoImportPanel({session,locale,native,visible=true}:{session:LcuSession;locale:Locale;native:boolean;visible?:boolean}) {
    const t = messages[locale], [preferences,setPreferences] = useState(initialPreferences);
    const [catalog,setCatalog] = useState<PreparationCatalog|null>(null), [catalogError,setCatalogError] = useState(false), [attempt,setAttempt] = useState(0), [storageFailed,setStorageFailed] = useState(false);
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
    useEffect(()=>{try {localStorage.setItem(storageKey,JSON.stringify(preferences));setStorageFailed(false);}catch {setStorageFailed(true);}},[preferences]);
    if (!visible) return null;
    return <details className="auto-import-panel"><summary>{t.title} · {snapshot.runes.status === 'confirmed' && snapshot.items.status === 'confirmed' ? t.confirmed : preferences.runes || preferences.items ? t.active : t.disabled}</summary>
        <p>{t.hint}</p><p>{t.singlePage}</p>{!native && <p>{t.desktop}</p>}
        <div className="auto-import-controls"><label><input type="checkbox" checked={preferences.runes} disabled={!native} onChange={e=>setPreferences(p=>({...p,runes:e.target.checked}))}/>{t.runes}</label>
        <label><input type="checkbox" checked={preferences.items} disabled={!native} onChange={e=>setPreferences(p=>({...p,items:e.target.checked}))}/>{t.items}</label>
        <label>{t.minimum}<input type="number" min={1} max={1000} step={1} value={preferences.minGames} disabled={!native} onChange={e=>{const minGames=e.target.valueAsNumber;if(Number.isInteger(minGames)&&minGames>=1&&minGames<=1000)setPreferences(p=>({...p,minGames}));}}/></label>
        <label>{t.role}<select value={preferences.customRole??''} disabled={!native} onChange={e=>setPreferences(p=>({...p,customRole:e.target.value?e.target.value as Role:undefined}))}><option value="">{t.chooseRole}</option>{(['TOP','JUNGLE','MIDDLE','BOTTOM','UTILITY'] as const).map(role=><option key={role} value={role}>{buildCopy[locale].roles[role]}</option>)}</select></label></div>
        {session.draft?.customGame && <p>{t.source}</p>}
        {import.meta.env.DEV && <small>{t.development}</small>}
        {target && <p>{target.championName} · {buildCopy[locale].roles[target.request.role]} · {target.request.platform} · {buildCopy[locale].queues[target.request.queue as 400|420|440]} · {t.scope} {target.request.patch} · {t.all}</p>}
        <div role="status" aria-live="polite"><Progress progress={snapshot.runes} locale={locale} label={t.runes}/><Progress progress={snapshot.items} locale={locale} label={t.items}/></div>
        <p>{t.itemsHint}</p>
        {native && (snapshot.runes.status==='error'||snapshot.items.status==='error'||snapshot.runes.status==='empty'||snapshot.items.status==='empty') && <button onClick={controller.retry}>{t.retry}</button>}
        {catalogError && <p>{t.catalog} <button onClick={()=>setAttempt(n=>n+1)}>{t.reload}</button></p>}
        {storageFailed && <p role="alert">{t.storage}</p>}
    </details>;
}
