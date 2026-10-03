import type {CatalogRecord} from '@olc/shared';
export function abilitySequence(record:CatalogRecord,records:CatalogRecord[]):CatalogRecord[]{
 if(record.kind!=='ability')return [];
 const champion=record.id.split(':')[0];
 return ['passive','Q','W','E','R'].flatMap(slot=>{
  const found=records.find(candidate=>candidate.kind==='ability'&&candidate.id===`${champion}:${slot}`&&candidate.namespace===record.namespace&&candidate.locale===record.locale);
  return found?[found]:[];
 });
}
export function adjacentAbility(record:CatalogRecord,records:CatalogRecord[],direction:1|-1):CatalogRecord|null{
 const sequence=abilitySequence(record,records),index=sequence.findIndex(candidate=>candidate.id===record.id);
 return sequence.length>1&&index>=0?sequence[(index+direction+sequence.length)%sequence.length]??null:null;
}
