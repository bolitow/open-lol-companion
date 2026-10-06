import {expect,it} from 'vitest';
import type {CatalogRecord,CatalogValue} from '@olc/shared';
import {catalogTooltipSegments,effectTooltipSegments,sliceTooltipSegments} from './abilityTooltip';
const source={source_id:'a'.repeat(64),pointer:'/data/Test/spells/0/tooltip'};
function record():CatalogRecord{
 return {kind:'ability',namespace:'standard',locale:'fr_FR',id:'1:Q',fields:{tooltip:{value:'A {{ x }} B',status:'descriptive',unit:null,sources:[source]},tooltip_segments:{value:[{text:'A ',damage_type:null},{text:'{{ x }}',damage_type:'magic'},{text:' B',damage_type:null}],status:'derived',unit:null,sources:[source]}}} as unknown as CatalogRecord;
}
it('valide le texte brut et la provenance, avant le moindre nettoyage d’affichage',()=>{
 const value=record();expect(catalogTooltipSegments(value)?.map(segment=>segment.text).join('')).toBe(value.fields.tooltip?.value);
 for(const mutate of [
  (field:CatalogValue)=>{field.value={text:'A {{ x }} B',damage_type:'magic'}},
  (field:CatalogValue)=>{field.value=[{text:20,damage_type:'magic'}]},
  (field:CatalogValue)=>{field.value=[{text:'A {{ x }} B'}]},
  (field:CatalogValue)=>{field.unit='seconds'},
  (field:CatalogValue)=>{field.sources=[{...source,pointer:'/tooltip'}]},
  (field:CatalogValue)=>{field.sources=[source,source]},
 ]){
  const invalid=record();mutate(invalid.fields.tooltip_segments!);expect(catalogTooltipSegments(invalid)).toBeNull();
 }
 const invalid=record();invalid.fields.tooltip!.status='conflict';expect(catalogTooltipSegments(invalid)).toBeNull();
});
it('découpe aux frontières de longueur sans fusionner les types',()=>{
 const segments=catalogTooltipSegments(record())!;
 expect(sliceTooltipSegments(segments,0,5)).toEqual([{text:'A ',damage_type:null},{text:'{{ ',damage_type:'magic'}]);
 expect(sliceTooltipSegments(segments,3,8).map(segment=>segment.text).join('')).toBe('{ x }');
});
it('nettoie un jeton réparti sur deux segments sans perdre le type de la variable suivante',()=>{
 const value=record();value.fields.tooltip!.value='A %i:scaleAP% {{ x }}';
 value.fields.tooltip_segments!.value=[{text:'A %i:sc',damage_type:null},{text:'aleAP% ',damage_type:'physical'},{text:'{{ x }}',damage_type:'true'}];
 expect(effectTooltipSegments(value,'A %i:scaleAP% {{ x }}')?.map(segment=>segment.text).join('')).toBe('A  {{ x }}');
 expect(effectTooltipSegments(value,'A %i:scaleAP% {{ x }}')?.at(-1)?.damage_type).toBe('true');
});
