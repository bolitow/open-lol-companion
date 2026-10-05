import {useEffect,useMemo,useState,useSyncExternalStore} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import type {DraftModelResult,DraftSession,DraftStatsReport} from '@olc/shared';
import type {Locale} from './state';
import {usePreparation} from './PreparationContext';
import {buildPatchStore} from './useBuildPatch';
import {resolveBuildPatch} from './buildPatch';
import {catalogRuntime} from './catalogRuntime';
import {rankLabel} from './buildRanks';
import {createDraftStatsStore,draftStatsRequest,draftStatsKey,estimateDraft} from './draftEstimate';
import './draftEstimate.css';
const copy={
 fr:{title:'Estimation de la draft',ally:'Notre équipe',enemy:'Équipe adverse',coverage:'champions couverts',loading:'Statistiques en cours de chargement…',error:'Statistiques indisponibles. Vérifiez la connexion au service dans les paramètres.',missing:'Données insuffisantes',unsupported:'Estimation disponible en draft classée Solo/Duo ou Flex.',explain:'Résumé descriptif de l’échantillon publié : ne prédit pas l’issue de la partie. Matchups et synergies non pris en compte.',retry:'Recharger'},
 en:{title:'Draft estimate',ally:'Our team',enemy:'Enemy team',coverage:'champions covered',loading:'Loading statistics…',error:'Statistics unavailable. Check the service connection in settings.',missing:'Insufficient data',unsupported:'Available in ranked Solo/Duo or Flex drafts.',explain:'Descriptive summary of the published sample: does not predict the game outcome. Matchups and synergies are not included.',retry:'Reload'},
};
type Status='ready'|'loading'|'error'|'unavailable';
export function DraftEstimateSummary({locale,teams,status,side,population,retry}:{locale:Locale;teams:DraftModelResult['teams']|null;status:Status;side:DraftSession['allySide'];population:string;retry?:()=>void}){
 const t=copy[locale],camps=side==='red'?['enemy','ally'] as const:['ally','enemy'] as const;
 return <section className="draft-estimate" aria-label={t.title}><header><strong>{t.title}</strong>{population&&<small>{population}</small>}</header>
  {status==='ready'?<><div className="draft-estimate-camps">{camps.map(camp=>{const team=teams?.[camp],rate=team?.estimated_win_rate;return <div key={camp}><span>{t[camp]}</span><strong>{rate===null||rate===undefined?t.missing:`${new Intl.NumberFormat(locale,{maximumFractionDigits:1}).format(rate)} %`}</strong><small>{team?`${team.eligible}/${team.champions} ${t.coverage}`:t.missing}</small></div>})}</div><p>{t.explain}</p></>:<p role={status==='error'?'status':undefined}>{status==='loading'?t.loading:status==='error'?t.error:t.unsupported}{status==='error'&&retry&&<button className="button" onClick={retry}>{t.retry}</button>}</p>}
 </section>;
}
/** Monté uniquement si le réglage est actif : aucune lecture ni calcul lorsqu'il est masqué. */
export function DraftEstimate({draft,locale}:{draft:DraftSession;locale:Locale}){
 const {session,value,rankReady}=usePreparation();
 const active=!!session?.connected&&session.phase==='ChampSelect';
 const candidate=draftStatsRequest(active,draft,'pending',session?.account?.platform??null,value.rank,rankReady!==false);
 const selection=candidate?JSON.stringify([candidate.platform,candidate.queue,candidate.rank]):'';
 const patches=useSyncExternalStore(buildPatchStore.subscribe,buildPatchStore.getSnapshot,buildPatchStore.getSnapshot);
 useEffect(()=>{if(selection)void buildPatchStore.load()},[selection]);
 const choice=resolveBuildPatch(patches.value,catalogRuntime.getSnapshot().directory.version);
 // Pas de repli silencieux sur une ancienne version pour cette estimation.
 const request=draftStatsRequest(active,draft,choice.patch===choice.requested?choice.patch:null,session?.account?.platform??null,value.rank,rankReady!==false);
 const key=request?draftStatsKey(request,patches.publicationRevision):'';
 const [store]=useState(()=>createDraftStatsStore(r=>isTauri()?invoke<DraftStatsReport>('community_draft_stats',{request:r}):Promise.reject('desktop_required')));
 const snapshot=useSyncExternalStore(store.subscribe,store.getSnapshot,store.getSnapshot);
 useEffect(()=>{void store.select(request,patches.publicationRevision);return()=>{void store.select(null,patches.publicationRevision)}},[store,key]);
 const report=key&&snapshot.key===key?snapshot.report:null;
 const estimate=useMemo(()=>estimateDraft(active,draft,request,report),[active,draft,key,report]);
 const status:Status=!candidate?'unavailable':!request?(choice.kind==='loading'?'loading':'error'):snapshot.key!==key||snapshot.status==='idle'?'loading':snapshot.status;
 const population=request?`${request.patch} · ${request.platform} · ${request.queue===420?'Solo/Duo':'Flex'} · ${rankLabel(request.rank,locale)}`:'';
 return <DraftEstimateSummary locale={locale} teams={estimate?.teams??null} status={status} side={draft.allySide} population={population} retry={()=>{if(request)void store.retry();else void buildPatchStore.load()}}/>;
}
