import data from "../../public/prototype/assets/rune-trees.json";
import {assets} from "./fixtures";
import {draftCopy} from "./draftCopy";
import {runePage} from "./runePage";
import {Icon} from "./Icon";
import type {DraftChampion} from "./draft";
import type {Locale} from "./model";

export function RuneTrees({champion,stage,locale}:{champion:DraftChampion;stage:number;locale:Locale}){
 const page=runePage(champion,stage),t=draftCopy[locale];
 return <div className="rune-trees">{[page.primary,page.secondary].map((id,index)=>{
  const tree=data[locale].find(tree=>tree.id===id)!;
  const rows=index===0?tree.rows:tree.rows.slice(1);
  return <section key={id} className={`rune-tree ${id===8100?"domination":"sorcery"}`} aria-label={`${index===0?t.primaryTree:t.secondaryTree} · ${tree.name}`}>
   <header><img src={`${assets}${tree.icon}`} alt=""/><div><small>{index===0?t.primaryTree:t.secondaryTree}</small><strong>{tree.name}</strong></div></header>
   <div className="rune-rows">{rows.map((row,r)=><ul role="list" className={`rune-row ${index===0&&r===0?"keystone-row":""}`} key={r}>{row.map(rune=>{
    const selected=page.selected.includes(rune.id);
    return <li key={rune.id} className={`rune-node ${selected?"is-selected":""}`} tabIndex={0} aria-label={`${rune.name} · ${selected?t.runeSelected:t.runeAlternative}`}>
     <img src={`${assets}${rune.icon}`} alt=""/>{selected&&<span className="rune-check"><Icon name="check" size={9}/></span>}
     <span className="rune-tooltip" aria-hidden="true">{rune.name}</span>
    </li>;
   })}</ul>)}</div>
  </section>;
 })}</div>;
}
