/** Cadence indépendante du transport ; un retour au premier plan ne lance pas de rafale. */
export function createCatalogRevalidation(refresh:()=>void,now=Date.now,visible=()=>document.visibilityState==='visible'){
 let last=-Infinity;
 const force=()=>{last=now();refresh()};
 return {force,focus:()=>{if(visible()&&now()-last>=300000)force()},periodic:()=>{if(visible())force()}};
}
