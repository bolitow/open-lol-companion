import {useDialogMotion} from '../ui/useDialogMotion';
import {useEffect,useRef,useState,useSyncExternalStore,type ReactNode} from 'react';
import {createPortal} from 'react-dom';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {importErrorMessage,type CatalogRecord,type DraftSession,type ImportResult,type RunePage} from '@olc/shared';
import {Icon} from '../ui/Icon';
import {RunePanel,type RunePanelEditor} from './PreparationPanels';
import {changeRuneStyle,chooseRune,runeDraftFromPage,runeDraftKey,runePagesMatch,validRuneDraft,type RuneDraft} from './runeEditing';
import {createRuneImportController,importEligibility} from './runeImport';
import {runeEditorCopy} from './runeEditorCopy';
import {preparationCopy} from './preparationCopy';
import type {Locale} from './state';
import './runeEditor.css';

// Le verrou survit au changement de variante, de langue et au démontage du panneau.
const imports=createRuneImportController(request=>invoke<ImportResult>('import_draft_runes',{request}));
export interface RuneImportContext {draft:DraftSession|null;equipped:RunePage|null;championId:number;championName:string;sourceKey:string}
interface Props extends RuneImportContext {page:RunePage|null;sourceLabel:string;records:CatalogRecord[];locale:Locale;onOpen:(record:CatalogRecord)=>void;statistics?:ReactNode}

function RuneEditorDialog({children,locale,onClose}:{children:ReactNode;locale:Locale;onClose:()=>void}){
 const motion=useDialogMotion(onClose);
 const dialog=useRef<HTMLDialogElement>(null),[trigger]=useState(()=>document.activeElement instanceof HTMLElement?document.activeElement:null);
 useEffect(()=>{const node=dialog.current;node?.showModal();return()=>{node?.close();if(trigger?.isConnected)trigger.focus({preventScroll:true})}},[trigger]);
 return createPortal(<dialog ref={dialog} className="expanded-runes rune-editor-dialog motion-surface" data-state={motion.state} inert={motion.closing} aria-label={preparationCopy[locale].runes} onCancel={event=>{event.preventDefault();motion.close()}}><button className="icon-button expanded-close" onClick={motion.close} aria-label={preparationCopy[locale].close}><Icon name="close"/></button>{children}</dialog>,document.body);
}

export function RuneWorkbench({page,sourceLabel,records,locale,onOpen,statistics,draft,equipped,championId,championName,sourceKey}:Props){
 const t=runeEditorCopy[locale],[editing,setEditing]=useState(false),[edited,setEdited]=useState<RuneDraft|null>(null),[expanded,setExpanded]=useState(false);
 const current=edited??runeDraftFromPage(page),key=runeDraftKey(current),modified=key!==runeDraftKey(runeDraftFromPage(page));
 const valid=validRuneDraft(current,records),eligibility=importEligibility(isTauri(),draft,championId,valid);
 const localChampion=draft?.allies.find(player=>player.local)?.championId??0;
 const context=JSON.stringify([sourceKey,championId,localChampion,draft?.supported??false,key]);
 const snapshot=useSyncExternalStore(imports.subscribe,imports.getSnapshot,imports.getSnapshot);
 useEffect(()=>{imports.activate(context);return()=>imports.activate(null)},[context]);
 const result=snapshot.result?.context===context?snapshot.result:null;
 const confirmed=!eligibility&&runePagesMatch(equipped,current);
 const failure=result?.status==='error'?result.error:null;
 const error=failure?failure in t?t[failure as keyof typeof t]:importErrorMessage(failure,locale):null;
 const status=snapshot.pending?t.pending:error??(confirmed?t.equipped:result?.status==='accepted'?t.accepted:eligibility?t[eligibility]:modified?t.manual:t.ready);
 const shown: RunePage|null=edited||editing?{...current,isValid:valid,isTemporary:false,autoModifiedSelections:[]}:page;
 const editor:RunePanelEditor|undefined=editing?{
  disabled:snapshot.pending,chooseLabel:t.choose,incompleteLabel:t.incomplete,
  onStyle:(branch,id)=>setEdited(changeRuneStyle(current,branch,id,records)),
  onChoose:(branch,slot,id)=>setEdited(chooseRune(current,branch,slot,id,records)),
 }:undefined;
 const submit=()=>{
  // Réévaluer aussi dans l’action : un ancien événement ne doit pas écrire une page invalide.
  if(importEligibility(isTauri(),draft,championId,validRuneDraft(current,records))||confirmed)return;
  void imports.submit(context,{championId,runes:{championName,...current}});
 };
 const contents=(large=false)=><>
  <RunePanel records={records} page={shown} locale={locale} onOpen={onOpen} sourceLabel={modified||!page?t.custom:sourceLabel} incompleteLabel={t.incomplete} editor={editor} onExpand={large?undefined:()=>setExpanded(true)}/>
  {!modified&&statistics}
  <div className="rune-editor-actions"><button className="rune-edit-toggle" aria-pressed={editing} disabled={snapshot.pending} onClick={()=>setEditing(value=>!value)}>{editing?t.done:t.edit}</button>{edited&&<button className="icon-button" disabled={snapshot.pending} aria-label={t.reset} onClick={()=>setEdited(null)}><Icon name="replay" size={14}/></button>}<button className="rune-import-button" disabled={!!eligibility||snapshot.pending||confirmed} onClick={submit}>{snapshot.pending?t.pending:t.import}<Icon name={confirmed?'check':'arrow'} size={14}/></button></div>
  <p className={`rune-import-status ${error?'has-error':''}`} role="status" aria-live="polite">{status}</p>
 </>;
 return <div className={`rune-workbench ${editing?'is-editing':''}`}>
  {contents()}
  {expanded&&<RuneEditorDialog locale={locale} onClose={()=>setExpanded(false)}><div className={`rune-workbench ${editing?'is-editing':''}`}>{contents(true)}</div></RuneEditorDialog>}
 </div>;
}
