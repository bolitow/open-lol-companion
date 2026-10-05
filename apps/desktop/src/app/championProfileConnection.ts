import type {CatalogRecord} from '@olc/shared';
import {loadCatalog,loadChampionAbilities,type PreparationCatalog} from './catalog';
import type {Locale} from './state';
export type ProfileData={catalog:PreparationCatalog;abilities:CatalogRecord[]};
export type ProfileLoad={status:'loading'}|{status:'error'}|{status:'ready';data:ProfileData};
export async function loadProfile(locale:Locale,id:number):Promise<ProfileData>{const catalog=await loadCatalog(locale);return {catalog,abilities:await loadChampionAbilities(locale,id,catalog.version,catalog.snapshotId)}}
export function connectProfile(read:()=>Promise<ProfileData>,receive:(state:ProfileLoad)=>void):()=>void{
 let active=true;receive({status:'loading'});
 Promise.resolve().then(()=>active?read():undefined).then(data=>{if(active&&data)receive({status:'ready',data})}).catch(()=>{if(active)receive({status:'error'})});
 return()=>{active=false};
}
