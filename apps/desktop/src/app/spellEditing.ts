import type {CatalogRecord,FlashSlot} from '@olc/shared';
export type SpellPair=[number,number];
export function parseFlashSlot(raw:string|null):FlashSlot|null{
 try{const value:unknown=JSON.parse(raw??'null');return value==='D'||value==='F'?value:null}catch{return null}
}
export function placeFlash(pair:SpellPair,slot:FlashSlot|null):SpellPair{
 return slot==='D'&&pair[1]===4||slot==='F'&&pair[0]===4?[pair[1],pair[0]]:[...pair];
}
/** Aperçu de la même règle appliquée en Rust sur une relecture de la draft locale. */
export function placeRecommendedSpells(pair:SpellPair,equipped:readonly number[]|null,slot:FlashSlot|null):SpellPair{
 if(pair.includes(4))return placeFlash(pair,slot);
 const valid=equipped?.length===2&&equipped[0]!==equipped[1]&&equipped.every(id=>Number.isSafeInteger(id)&&id>0);
 return valid&&(equipped[0]===pair[1]||equipped[1]===pair[0])?[pair[1],pair[0]]:[...pair];
}
export function chooseSpell(pair:SpellPair,index:0|1,id:number):SpellPair{
 const next:SpellPair=[...pair],other=index===0?1:0;
 if(next[other]===id)next[other]=next[index];
 next[index]=id;return next;
}
/** Le lot #13 est borné aux drafts classiques de la Faille. Le client reste l'autorité des sorts débloqués. */
export function spellChoices(records:readonly CatalogRecord[]):CatalogRecord[]{
 return records.filter(r=>r.kind==='summoner_spell'&&Array.isArray(r.fields.modes?.value)&&r.fields.modes.value.includes('CLASSIC'));
}
export function validSpellPair(pair:readonly number[],records:readonly CatalogRecord[]):pair is SpellPair{
 return pair.length===2&&pair[0]!==pair[1]&&pair.every(id=>Number.isSafeInteger(id)&&id>0&&spellChoices(records).some(r=>r.id===String(id)));
}
export function spellsMatch(equipped:readonly number[]|null,pair:SpellPair):boolean{
 return pair[0]>0&&pair[1]>0&&pair[0]!==pair[1]&&equipped?.[0]===pair[0]&&equipped?.[1]===pair[1];
}
/** Seule une action sur Flash ou sur son emplacement peut choisir sa préférence. */
export function editSpellSelection(pair:SpellPair,index:0|1,id:number,preference:FlashSlot|null):{pair:SpellPair;flashSlot:FlashSlot|null}{
 const next=chooseSpell(pair,index,id);
 return {pair:next,flashSlot:next.includes(4)&&(id===4||pair[index]===4)?next[0]===4?'D':'F':preference};
}
