import {useSettings} from './SettingsContext';
import {useEffect,useState,useSyncExternalStore,type ReactNode} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {importErrorMessage,type CatalogRecord,type DraftSession,type FlashSlot,type ImportDraftSpellsRequest,type ImportResult} from '@olc/shared';
import {CatalogButton} from './GameDetails';
import {Icon} from '../ui/Icon';
import {createImportController,importEligibility} from './runeImport';
import {editSpellSelection,placeFlash,spellChoices,spellsMatch,validSpellPair,type SpellPair} from './spellEditing';
import {spellCopy} from './spellCopy';
import type {Locale} from './state';
import './spells.css';

const imports=createImportController<ImportDraftSpellsRequest>(request=>invoke<ImportResult>('import_draft_spells',{request}));
interface Props {draft:DraftSession|null;championId:number;sourceKey:string;source?:readonly number[]|null;records:CatalogRecord[];locale:Locale;onOpen:(r:CatalogRecord)=>void;statistics?:ReactNode;variantPicker?:ReactNode}
export function SpellWorkbench({draft,championId,sourceKey,source,records,locale,onOpen,statistics,variantPicker}:Props){
 const t=spellCopy[locale],[edited,setEdited]=useState<SpellPair|null>(null);
 const settings=useSettings(),flashSlot=settings.state.values.flashSlot,saveError=settings.state.flashStorageFailed;
 const base=source??draft?.localSpells??[0,0],basePair:SpellPair=[base[0]??0,base[1]??0];
 const current=placeFlash(edited??basePair,flashSlot),choices=spellChoices(records);
 const modified=[...current].sort().join(':')!==[...basePair].sort().join(':');
 const eligibility=importEligibility(isTauri(),draft,championId,validSpellPair(current,records));
 const needsFlash=current.includes(4)&&flashSlot===null;
 const local=draft?.allies.find(p=>p.local);
 const context=JSON.stringify([sourceKey,championId,local?.championId,draft?.supported,current,flashSlot]);
 const snapshot=useSyncExternalStore(imports.subscribe,imports.getSnapshot,imports.getSnapshot);
 useEffect(()=>{imports.activate(context);return()=>imports.activate(null)},[context]);
 const result=snapshot.result?.context===context?snapshot.result:null;
 const confirmed=!eligibility&&spellsMatch(draft?.localSpells??null,current);
 const failure=result?.status==='error'?result.error:null;
 const error=failure?failure in t?t[failure as keyof typeof t]:importErrorMessage(failure,locale):null;
 const status=snapshot.pending?t.pending:error??(confirmed?t.confirmed:result?.status==='accepted'?t.accepted:eligibility?t[eligibility]:needsFlash?t.chooseFlash:t.ready);
 const selectFlash=(slot:FlashSlot)=>settings.store.change('flashSlot',slot);
 const selectSpell=(index:0|1,id:number)=>{
  const next=editSpellSelection(current,index,id,flashSlot);setEdited(next.pair);
  // Choisir explicitement Flash dans un emplacement modifie aussi sa préférence.
  if(next.flashSlot!==null&&next.flashSlot!==flashSlot)selectFlash(next.flashSlot);
 };
 const submit=()=>{
  if(importEligibility(isTauri(),draft,championId,validSpellPair(current,records))||needsFlash||confirmed)return;
  // Sans Flash, la valeur technique D ne réordonne rien et ne crée aucune préférence.
  void imports.submit(context,{championId,spells:{spellIds:current,flashSlot:flashSlot??'D'}});
 };
 return <section className="spell-workbench" aria-label={t.title}>
  <header className="build-panel-heading"><h2>{t.title}</h2>{variantPicker??<small>{modified?t.custom:source?t.source:t.client}</small>}</header>
  <div className="spell-slot-pair">{([0,1] as const).map((index)=>{const record=records.find(r=>r.kind==='summoner_spell'&&r.id===String(current[index]));return <div className="spell-slot" key={index}><b>{index===0?'D':'F'}</b>{record?<CatalogButton record={record} locale={locale} onOpen={onOpen}/>:<span className="spell-empty"><Icon name="info" size={20}/></span>}<select aria-label={`${t.choose} · ${index===0?'D':'F'}`} value={current[index]} disabled={snapshot.pending} onChange={e=>selectSpell(index,Number(e.target.value))}><option value={0}>{t.choose}</option>{record&&!choices.includes(record)&&<option value={record.id}>{record.name}</option>}{choices.map(r=><option value={r.id} key={r.id}>{r.name}</option>)}</select></div>})}</div>
  <div className="spell-controls"><div className="spell-preference"><span>{t.flash}</span>{(['D','F'] as const).map(slot=><button key={slot} aria-label={`${t.flash} ${slot}`} aria-pressed={slot===flashSlot} disabled={snapshot.pending} onClick={()=>selectFlash(slot)}>{slot}</button>)}{edited&&<button className="icon-button spell-reset" aria-label={t.reset} disabled={snapshot.pending} onClick={()=>setEdited(null)}><Icon name="replay" size={14}/></button>}</div>
  <button className="rune-import-button spell-import" disabled={!!eligibility||needsFlash||confirmed||snapshot.pending} onClick={submit}>{snapshot.pending?t.pending:t.import}<Icon name={confirmed?'check':'arrow'} size={14}/></button></div>
  {modified?<small className="spell-custom-label">{t.custom}</small>:statistics}
  <p className={`rune-import-status ${error?'has-error':''}`} role="status" aria-live="polite">{status}</p>{saveError&&<p className="rune-import-status has-error" role="status">{t.saveError}</p>}
 </section>;
}
