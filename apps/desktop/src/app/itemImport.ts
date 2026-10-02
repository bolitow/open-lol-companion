import type {CatalogRecord,ImportItemsRequest} from '@olc/shared';

/** Le bloc reflète la variante consultée, sans inventer des conseils situationnels. */
export function itemSetRequest(championId:number,championName:string,label:string,selection:readonly number[],records:readonly CatalogRecord[]):ImportItemsRequest|null{
 const validId=(id:number)=>Number.isInteger(id)&&id>0&&id<=2147483647;
 if(!validId(championId)||!championName.trim()||!label.trim()||!selection.length)return null;
 if(selection.some(id=>!validId(id)||!records.some(record=>record.kind==='item'&&record.id===String(id))))return null;
 return {championId,championName,mapId:11,blocks:[{label,items:selection.map(id=>({id,count:1}))}]};
}
