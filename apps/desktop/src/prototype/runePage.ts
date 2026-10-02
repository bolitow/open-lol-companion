import type {DraftChampion} from "./draft";

/** Fixtures de présentation : aucun calcul de recommandation ou import réel. */
export function runePage(champion:DraftChampion,stage:number){
 if(champion==="Ahri"&&stage<2)return {primary:8100,secondary:8200,selected:[8112,8139,8140,8106,8224,8210]};
 const keystone=champion==="Orianna"?8229:champion==="Lux"?8214:8230;
 return {primary:8200,secondary:8100,selected:[keystone,8226,8210,8237,8139,8106]};
}
