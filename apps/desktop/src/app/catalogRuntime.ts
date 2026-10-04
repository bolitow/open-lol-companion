import {invoke,isTauri} from '@tauri-apps/api/core';
import type {CatalogRuntimeState} from '@olc/shared';
import embeddedDirectory from '../../public/game-data/champion-directory.json';
import embeddedIndex from '../../public/game-data/champions.json';
export type RuntimeChampion=typeof embeddedDirectory.champions[number]&{image?:string};
export type ChampionIndex=Record<string,{key:string;fr:string;en:string}>;
type Active=CatalogRuntimeState&{snapshotId:string;version:string;assetBase:string};
interface Snapshot {status:CatalogRuntimeState['status'];error:string|null;active:Active|null;generation:number;directory:{version:string;champions:RuntimeChampion[]};index:ChampionIndex}
const object=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==='object'&&!Array.isArray(v);
const localized=(v:unknown)=>object(v)&&typeof v.fr==='string'&&typeof v.en==='string';
export function validCatalogPath(path:string){return /^(?:[a-zA-Z0-9_-]+\/)*[a-zA-Z0-9_-]+\.(?:json|png|jpg|webp)$/.test(path)}
function validate(directory:unknown,index:unknown,version:string):asserts directory is Snapshot['directory']{
 if(!object(directory)||directory.version!==version||!Array.isArray(directory.champions)||!directory.champions.length||!object(index))throw Error('catalog-invalid');
 const ids=new Set<number>();
 for(const c of directory.champions){
  if(!object(c)||!Number.isSafeInteger(c.id)||Number(c.id)<=0||ids.has(Number(c.id))||typeof c.key!=='string'||!localized(c.names)||!localized(c.titles)||!Array.isArray(c.categories)||!c.categories.every(k=>['Assassin','Fighter','Mage','Marksman','Support','Tank'].includes(String(k)))||(c.image!==undefined&&(typeof c.image!=='string'||!validCatalogPath(c.image))))throw Error('catalog-invalid');
  const entry=index[String(c.id)];if(!object(entry)||entry.key!==c.key||!localized(entry)||!object(c.names)||entry.fr!==c.names.fr||entry.en!==c.names.en)throw Error('catalog-invalid');ids.add(Number(c.id));
 }
 if(Object.keys(index).length!==ids.size)throw Error('catalog-invalid');
}
export function createCatalogRuntime(transport:{read:(snapshotId:string,path:string)=>Promise<unknown>}){
 let current:Snapshot={status:'embedded',error:null,active:null,generation:0,directory:embeddedDirectory,index:embeddedIndex};
 let serial=0;const listeners=new Set<()=>void>();
 const publish=(next:Snapshot)=>{current=next;listeners.forEach(fn=>fn())};
 return {getSnapshot:()=>current,subscribe:(fn:()=>void)=>{listeners.add(fn);return()=>{listeners.delete(fn)}},
 async accept(state:CatalogRuntimeState){
  const request=++serial;
  if(!state.snapshotId||!state.version||!state.assetBase||state.snapshotId===current.active?.snapshotId){publish({...current,status:state.status,error:state.error});return}
  try{
   const [directory,index]=await Promise.all([transport.read(state.snapshotId,'champion-directory.json'),transport.read(state.snapshotId,'champions.json')]);
   validate(directory,index,state.version);
   if(request!==serial)return;
   publish({status:state.status,error:state.error,active:state as Active,generation:current.generation+1,directory,index:index as ChampionIndex});
  }catch{if(request===serial)publish({...current,status:'error',error:'catalog-invalid'})}
 }};
}
export const catalogRuntime=createCatalogRuntime({read:(snapshotId,path)=>invoke('catalog_read',{snapshotId,path})});
export function catalogAsset(path:string,active=catalogRuntime.getSnapshot().active){
 if(!validCatalogPath(path))throw Error('catalog-invalid');
 return active?`${active.assetBase}${path}`:`/game-data/${path}`;
}
export function championImage(id:number){const snapshot=catalogRuntime.getSnapshot();return catalogAsset(snapshot.directory.champions.find(c=>c.id===id)?.image??`champions/${id}.jpg`,snapshot.active)}
export async function readCatalogJson(path:string,snapshotId?:string|null):Promise<unknown>{
 if(snapshotId)return invoke('catalog_read',{snapshotId,path});
 const response=await fetch(`/game-data/${path}`);if(!response.ok)throw Error('catalog-unavailable');return response.json();
}
let pending:Promise<void>|null=null;
let initialized=false;
let eventRevision=0,lastEvent:CatalogRuntimeState|null=null;
export function receiveCatalogState(state:CatalogRuntimeState){eventRevision++;lastEvent=state;return catalogRuntime.accept(state)}
async function requestState(command:string){
 const revision=eventRevision;
 const state=await invoke<CatalogRuntimeState>(command);
 // Un événement d'une autre fenêtre peut dépasser la réponse de cette commande.
 if(revision!==eventRevision&&lastEvent?.snapshotId!==state.snapshotId)return;
 await catalogRuntime.accept(state);
}
export function refreshCatalog(){
 if(!isTauri())return Promise.resolve();
 if(pending)return pending;
 const initialize=initialized?Promise.resolve():requestState('catalog_state');
 initialized=true;
 pending=initialize.catch(()=>{}).then(()=>requestState('catalog_sync')).catch(()=>catalogRuntime.accept({...catalogRuntime.getSnapshot().active,status:'error',version:null,snapshotId:null,assetBase:null,error:'catalog-unavailable'})).finally(()=>{pending=null});return pending;
}
