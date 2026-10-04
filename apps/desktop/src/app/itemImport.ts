import type {CatalogRecord,ImportItemsRequest} from '@olc/shared';
import {readableValue} from './preparation';

/** Objet remplacé par son ancêtre achetable, ou retiré faute d'ancêtre unique. */
export interface ItemSetPlan{request:ImportItemsRequest;converted:{from:number;to:number}[];dropped:number[]}

/** Nombre d'objets remplacés (formes distinctes) et retirés, affiché tel quel à l'utilisateur. */
export interface ItemAdjustments{converted:number;dropped:number}
export const itemAdjustments=(plan:ItemSetPlan):ItemAdjustments=>({converted:new Set(plan.converted.map(c=>c.from)).size,dropped:plan.dropped.length});

const MAX_CHAIN=8;

/** Seules les catégories de parcours donnent un set cohérent : un objet isolé ou la seule relique écraserait un set utile. */
const IMPORTABLE_CATEGORIES:readonly string[]=['purchase_order','final_items'];
export const isImportableCategory=(category:string)=>IMPORTABLE_CATEGORIES.includes(category);

/**
 * Statut boutique d'un objet : `unknown` quand `purchasable` ou `in_store` n'est pas lisible
 * (absent, non vérifié, non booléen), à ne jamais confondre avec un `false` lisible.
 */
function shopState(record:CatalogRecord):'shop'|'no'|'unknown'{
 const purchasable=readableValue(record,'purchasable'),inStore=readableValue(record,'in_store');
 if(typeof purchasable!=='boolean'||typeof inStore!=='boolean')return 'unknown';
 return purchasable&&inStore?'shop':'no';
}

/**
 * Remonte d'un objet non achetable (forme évoluée de Larme, quête, bottes de niveau 3…) vers
 * l'objet achetable dont il découle : `special_recipe` d'abord, sinon l'unique composant de
 * `builds_from`. Plusieurs composants ou aucun lien rendraient le choix arbitraire : `null`
 * (objet retiré). Un maillon au statut boutique illisible rend toute la résolution incertaine :
 * `'unknown'`, la variante est alors rejetée.
 */
function purchasableAncestor(id:number,byId:ReadonlyMap<string,CatalogRecord>):number|null|'unknown'{
 const seen=new Set<number>();
 for(let current=id;seen.size<MAX_CHAIN;){
  const record=byId.get(String(current));
  if(!record||seen.has(current))return null;
  const state=shopState(record);
  if(state==='unknown')return 'unknown';
  if(state==='shop')return current;
  seen.add(current);
  const recipe=readableValue(record,'special_recipe'),parents=readableValue(record,'builds_from');
  const next=typeof recipe==='number'&&recipe>0?recipe:Array.isArray(parents)&&parents.length===1?Number(parents[0]):0;
  if(!Number.isSafeInteger(next)||next<=0)return null;
  current=next;
 }
 return null;
}

/**
 * Le set importé ne contient que des objets achetables du catalogue (#88). La conversion
 * se fait ici, sur le catalogue du patch, et non plus par une table codée dans le cœur Rust.
 * Un identifiant absent du catalogue, ou dont `purchasable` / `in_store` n'est pas lisible
 * (sélection ou maillon parcouru), rejette la variante : on ne devine rien. Seul un `false`
 * lisible sans ancêtre achetable unique retire l'objet, signalé dans `dropped`.
 */
export function itemSetPlan(championId:number,championName:string,label:string,selection:readonly number[],records:readonly CatalogRecord[]):ItemSetPlan|null{
 const validId=(id:number)=>Number.isInteger(id)&&id>0&&id<=2147483647;
 if(!validId(championId)||!championName.trim()||!label.trim()||!selection.length||selection.some(id=>!validId(id)))return null;
 const byId=new Map<string,CatalogRecord>();
 for(const record of records)if(record.kind==='item'&&!byId.has(record.id))byId.set(record.id,record);
 if(selection.some(id=>!byId.has(String(id))))return null;
 const items:{id:number;count:number}[]=[],converted:ItemSetPlan['converted']=[],dropped:number[]=[];
 const emitted=new Set<number>();
 for(const id of selection){
  const ancestor=purchasableAncestor(id,byId);
  if(ancestor==='unknown')return null;
  if(ancestor===null){dropped.push(id);continue}
  if(ancestor===id){items.push({id,count:1});emitted.add(id);continue}
  converted.push({from:id,to:ancestor});
  // Plusieurs formes évoluées d'une même lignée ne doivent pas répéter leur ancêtre.
  if(!emitted.has(ancestor)){items.push({id:ancestor,count:1});emitted.add(ancestor)}
 }
 if(!items.length)return null;
 return {request:{championId,championName,mapId:11,blocks:[{label,items}]},converted,dropped};
}

/** Le bloc reflète la variante consultée, sans inventer des conseils situationnels. */
export function itemSetRequest(championId:number,championName:string,label:string,selection:readonly number[],records:readonly CatalogRecord[]):ImportItemsRequest|null{
 return itemSetPlan(championId,championName,label,selection,records)?.request??null;
}
