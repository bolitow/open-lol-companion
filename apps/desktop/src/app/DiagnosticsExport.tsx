import {useDialogMotion} from '../ui/useDialogMotion';
import {useEffect,useId,useRef,useState,useSyncExternalStore} from 'react';
import {createPortal} from 'react-dom';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {Icon} from '../ui/Icon';
import {createDiagnosticsExport,type DiagnosticLocale,type DiagnosticExportResult,type DiagnosticsExportController} from './diagnosticsExportController';
import './diagnosticsExport.css';
const fr={
 open:'Exporter un diagnostic',desktop:'Disponible dans l’application desktop.',title:'Diagnostic à enregistrer',intro:'Une archive ZIP locale, sans envoi automatique.',session:'Journal technique de cette session',sessionDetail:'Jusqu’à 200 événements techniques : connexion, phases et réglages.',system:'Version de l’application, système et architecture',privacy:'Aucun compte, secret, chemin ni texte brut des logs League.',include:'Ajouter un résumé de logs League',includeDetail:'Vous choisirez les fichiers dans une fenêtre système : 5 fichiers maximum, 8 Mio chacun.',summary:'Seuls le nombre de lignes et les nombres de lignes contenant les marqueurs ERROR, WARN et INFO seront ajoutés. Ce résumé ne contient pas les messages.',save:'Choisir où enregistrer',retry:'Réessayer',close:'Fermer',pending:'Terminez le choix dans la fenêtre système. Aucun export supplémentaire ne sera lancé.',exported:'Diagnostic enregistré sur cet appareil.',cancelled:'Export annulé. Aucun diagnostic enregistré.',
 errors:{busy:'Un export est déjà en cours. Terminez-le avant de réessayer.',unavailable:'L’export est indisponible pour le moment. Réessayez dans l’application desktop.',read_failed:'Impossible de lire les fichiers choisis. Essayez avec d’autres fichiers.',write_failed:'Impossible d’enregistrer l’archive. Choisissez un autre emplacement.',too_large:'La sélection dépasse la limite : 5 fichiers de 8 Mio maximum chacun.',unsupported:'Un fichier sélectionné n’est pas un log texte pris en charge.'},
};
const en:typeof fr={
 open:'Export diagnostics',desktop:'Available in the desktop application.',title:'Diagnostics to save',intro:'A local ZIP archive, with no automatic upload.',session:'Technical log for this session',sessionDetail:'Up to 200 technical events: connection, phases and settings.',system:'Application version, operating system and architecture',privacy:'No account, secret, path or raw League log text.',include:'Add a League log summary',includeDetail:'Choose files in a system window: up to 5 files, 8 MiB each.',summary:'Only the total line count and counts of lines containing ERROR, WARN and INFO markers will be added. The summary contains no messages.',save:'Choose where to save',retry:'Try again',close:'Close',pending:'Complete the selection in the system window. No additional export will start.',exported:'Diagnostics saved on this device.',cancelled:'Export cancelled. No diagnostics saved.',
 errors:{busy:'An export is already running. Finish it before trying again.',unavailable:'Export is currently unavailable. Try again in the desktop application.',read_failed:'The selected files could not be read. Try other files.',write_failed:'The archive could not be saved. Choose another location.',too_large:'Selection exceeds the limit: up to 5 files of 8 MiB each.',unsupported:'A selected file is not a supported text log.'},
};
const copy={fr,en};
export function DiagnosticsExport({locale}:{locale:DiagnosticLocale}){
 const [controller]=useState(()=>createDiagnosticsExport({native:isTauri(),export:request=>invoke<DiagnosticExportResult>('export_diagnostics',{...request})}));
 const [open,setOpen]=useState(false),trigger=useRef<HTMLButtonElement>(null),hint=useId(),t=copy[locale];
 return <div className="diagnostics-launch"><button ref={trigger} className="button" type="button" disabled={!controller.native} aria-describedby={!controller.native?hint:undefined} onClick={()=>{controller.reset();setOpen(true)}}><Icon name="shield" size={15}/>{t.open}</button>{!controller.native&&<small id={hint}>{t.desktop}</small>}{open&&<DiagnosticsDialog locale={locale} controller={controller} trigger={trigger.current} close={()=>setOpen(false)}/>}</div>;
}
function DiagnosticsDialog({locale,controller,trigger,close}:{locale:DiagnosticLocale;controller:DiagnosticsExportController;trigger:HTMLButtonElement|null;close:()=>void}){
 const [includeLeagueSummary,setInclude]=useState(false),dialog=useRef<HTMLDialogElement>(null),heading=useRef<HTMLHeadingElement>(null),title=useId(),description=useId(),choice=useId();
 const state=useSyncExternalStore(controller.subscribe,controller.getSnapshot,controller.getSnapshot),pending=state.status==='pending',t=copy[locale];
 useEffect(()=>{const node=dialog.current;node?.showModal();heading.current?.focus({preventScroll:true});return()=>{node?.close();if(trigger?.isConnected)trigger.focus({preventScroll:true})}},[trigger]);
 const motion=useDialogMotion(close,pending);
 const status=state.status==='error'?t.errors[state.error??'unavailable']:state.status==='idle'?'':t[state.status];
 return createPortal(<dialog ref={dialog} className="diagnostics-dialog motion-surface" data-state={motion.state} inert={motion.closing} aria-labelledby={title} aria-describedby={description} onCancel={event=>{event.preventDefault();motion.close()}}>
  <header><h2 ref={heading} tabIndex={-1} id={title}>{t.title}</h2><button type="button" className="icon-button" aria-label={t.close} disabled={pending} onClick={motion.close}><Icon name="close" size={18}/></button></header>
  <div className="diagnostics-content"><p id={description}>{t.intro}</p><ul><li><strong>{t.session}</strong><span>{t.sessionDetail}</span></li><li>{t.system}</li></ul><p className="diagnostics-privacy"><Icon name="shield" size={17}/>{t.privacy}</p>
   <div className="diagnostics-option"><label><input type="checkbox" checked={includeLeagueSummary} disabled={pending} aria-describedby={choice} onChange={event=>setInclude(event.target.checked)}/><strong>{t.include}</strong></label><p id={choice}>{t.includeDetail}</p>{includeLeagueSummary&&<p>{t.summary}</p>}</div>
   <p className="diagnostics-status" role={state.status==='error'?'alert':'status'} aria-live="polite">{status}</p>
  </div>
  <footer><button type="button" className="button" disabled={pending} onClick={motion.close}>{t.close}</button><button type="button" className="button primary" disabled={pending} aria-busy={pending} onClick={()=>{void controller.run({includeLeagueSummary,locale})}}>{state.status==='error'?t.retry:t.save}</button></footer>
 </dialog>,document.body);
}
