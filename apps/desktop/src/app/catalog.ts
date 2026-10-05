import {catalogRuntime,readCatalogJson,catalogAsset,validCatalogPath} from './catalogRuntime';
import type {CatalogRecord} from '@olc/shared';
import type {Locale} from './state';
export interface PreparationCatalog {version:string;snapshotId?:string;records:CatalogRecord[]}
const object=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==='object'&&!Array.isArray(v);
// Les fichiers sont produits par le normaliseur #61. Refuser un export incomplet,
// plutôt que présenter un catalogue vide comme une réponse métier valide.
export async function loadCatalog(locale:Locale):Promise<PreparationCatalog>{
 const language=locale==='fr'?'fr_FR':'en_US';
 const active=catalogRuntime.getSnapshot().active;
 const catalog=parseCatalog(await readCatalogJson(`catalog/${language}.json`,active?.snapshotId),language,['rune','rune_shard','item','summoner_spell','augment']);
 if(active&&catalog.version!==active.version)throw Error('catalog-invalid');
 return {...catalog,...(active?{snapshotId:active.snapshotId}:{}),records:catalog.records.map(record=>({...record,icon:record.icon?catalogAsset(record.icon.slice('/game-data/'.length),active):null}))};
}
function parseCatalog(value:unknown,language:string,kinds:string[]):PreparationCatalog{
 if(!object(value)||typeof value.version!=='string'||!/^\d+\.\d+\.\d+$/.test(value.version)||!Array.isArray(value.records)||!value.records.length)throw new Error('catalog-invalid');
 for(const record of value.records){
  if(!object(record)||typeof record.id!=='string'||typeof record.name!=='string'||record.locale!==language||!kinds.includes(String(record.kind))||record.namespace!=='standard'||!(record.description===null||typeof record.description==='string')||!(record.icon===null||typeof record.icon==='string'&&record.icon.startsWith('/game-data/catalog/icons/')&&validCatalogPath(record.icon.slice('/game-data/'.length)))||!object(record.fields)||!object(record.stats)||!object(record.coverage)||!Array.isArray(record.effects))throw new Error('catalog-invalid');
 }
 return value as unknown as PreparationCatalog;
}
/** Les fiches lourdes ne sont lues que pour le champion consulté. */
export async function loadChampionAbilities(locale:Locale,championId:number,version?:string,snapshotId?:string):Promise<CatalogRecord[]>{
 if(!Number.isSafeInteger(championId)||championId<=0)throw new Error('catalog-invalid');
 const language=locale==='fr'?'fr_FR':'en_US';
 const active=catalogRuntime.getSnapshot().active;
 if(snapshotId&&active?.snapshotId!==snapshotId)throw Error('catalog-stale');
 const captured=version!==undefined&&!snapshotId?null:active;
 const catalog=parseCatalog(await readCatalogJson(`catalog/champions/${championId}/${language}.json`,snapshotId??captured?.snapshotId),language,['champion','ability']);
 if((captured&&catalog.version!==captured.version)||(version!==undefined&&catalog.version!==version)||catalog.records.some(r=>r.kind==='champion'?r.id!==String(championId):!new RegExp(`^${championId}:(Q|W|E|R|passive)$`).test(r.id)))throw new Error('catalog-invalid');
 return catalog.records.map(record=>({...record,icon:record.icon?catalogAsset(record.icon.slice('/game-data/'.length),captured):null}));
}
