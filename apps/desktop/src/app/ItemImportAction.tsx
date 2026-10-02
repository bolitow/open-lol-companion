import {useEffect,useSyncExternalStore} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {importErrorMessage,type ImportItemsRequest,type ImportResult} from '@olc/shared';
import {createImportController} from './runeImport';
import type {Locale} from './state';
const imports=createImportController<ImportItemsRequest>(request=>invoke<ImportResult>('import_items',{request}));
const copy={fr:{action:'Importer cette variante',pending:'Import en cours…',accepted:'Set accepté par League.',desktop:'Import disponible dans l’application desktop.',offline:'Connectez le client League.',invalid:'Variante vide ou objets indisponibles.',hint:'Remplace le set de l’app pour ce champion sur la Faille.'},en:{action:'Import this variant',pending:'Importing…',accepted:'Set accepted by League.',desktop:'Import available in the desktop app.',offline:'Connect the League client.',invalid:'Empty variant or unavailable items.',hint:'Replaces the app’s set for this champion on Summoner’s Rift.'}};
export function ItemImport({request,sourceKey,connected,locale}:{request:ImportItemsRequest|null;sourceKey:string;connected:boolean;locale:Locale}){
 const t=copy[locale],snapshot=useSyncExternalStore(imports.subscribe,imports.getSnapshot,imports.getSnapshot);
 const context=JSON.stringify([sourceKey,request,connected,locale]);
 useEffect(()=>{imports.activate(context);return()=>imports.activate(null)},[context]);
 const blocked=!isTauri()?t.desktop:!connected?t.offline:!request?t.invalid:null;
 const result=snapshot.result?.context===context?snapshot.result:null;
 const message=snapshot.pending?t.pending:blocked??(result?.status==='error'?importErrorMessage(result.error,locale):result?.status==='accepted'?t.accepted:t.hint);
 return <div className="item-import"><button className="button" disabled={!!blocked||snapshot.pending} onClick={()=>{if(!blocked&&request)void imports.submit(context,request)}}>{snapshot.pending?t.pending:t.action}</button><p role="status">{message}</p></div>;
}
