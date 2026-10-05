import {catalogRuntime,type RuntimeChampion} from './catalogRuntime';
import type {Locale} from './state';
export type ChampionCategory='Assassin'|'Fighter'|'Mage'|'Marksman'|'Support'|'Tank';
export type ChampionSummary=RuntimeChampion;
export const championDirectory={get version(){return catalogRuntime.getSnapshot().directory.version},get champions(){return catalogRuntime.getSnapshot().directory.champions}};
const normalize=(value:string)=>value.normalize('NFD').replace(/[\u0300-\u036f]/g,'').toLowerCase().replace(/[^a-z0-9]/g,'');
export function findChampions(query:string,category:string,locale:Locale,descending=false):ChampionSummary[]{
 const needle=normalize(query);
 return championDirectory.champions.filter(c=>(category==='ALL'||c.categories.includes(category))&&[c.names.fr,c.names.en,c.key].some(name=>normalize(name).includes(needle)))
 .sort((a,b)=>{
  const exact=(c:ChampionSummary)=>needle&&[c.names.fr,c.names.en,c.key].some(name=>normalize(name)===needle)?0:1;
  return exact(a)-exact(b)||(descending?-1:1)*a.names[locale].localeCompare(b.names[locale],locale);
 });
}
