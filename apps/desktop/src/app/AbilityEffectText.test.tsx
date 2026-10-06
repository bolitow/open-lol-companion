import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {AbilityEffectText} from './AbilityEffects';
import type {AbilityEffectEntry} from './abilityEffectCatalog';
import type {CatalogRecord} from '@olc/shared';
import {readFileSync} from 'node:fs';
import {matchingAbilityEffects,parseAbilityEffects} from './abilityEffectCatalog';
const origin={source_id:'a'.repeat(64),pointer:'/data/Ahri/spells/0/tooltip'};
const typedRecord=(tooltip:string,segments:unknown,status='derived')=>({kind:'ability',namespace:'standard',locale:'fr_FR',id:'103:Q',description:'Inflige des dégâts magiques.',fields:{tooltip:{value:tooltip,status:'descriptive',unit:null,sources:[origin]},tooltip_segments:{value:segments,status,unit:null,sources:[origin]}}} as unknown as CatalogRecord);
it('colore chaque occurrence du même placeholder selon son segment, indépendamment des mots',()=>{
 const tooltip='X {{ totaldamage }} Y {{ totaldamage }}';
 const formula={terms:[{values:[35,60]},{values:[.5],stat:'ability_power'},{values:[.3],stat:'attack_damage'}]};
 const entry={tooltip,formulas:{totaldamage:formula},unresolved:[]} as unknown as AbilityEffectEntry;
 const record=typedRecord(tooltip,[{text:'X ',damage_type:null},{text:'{{ totaldamage }}',damage_type:'magic'},{text:' Y ',damage_type:null},{text:'{{ totaldamage }}',damage_type:'true'}]);
 const html=renderToStaticMarkup(<AbilityEffectText entry={entry} record={record} locale="fr"/>);
 expect(html).toContain('<span class="ability-magic-damage"><strong class="ability-formula"');
 expect(html).toContain('<span class="ability-true-damage"><strong class="ability-formula"');
 expect(html.match(/data-stat="ability_power"/g)).toHaveLength(2);
 expect(html.match(/data-stat="attack_damage"/g)).toHaveLength(2);
});
it('colore les montants depuis les types servis et conserve les couleurs des ratios',()=>{
 for(const tooltip of ['{{ first }} pts de dégâts magiques et {{ second }} pts de dégâts bruts ; {{ third }} pts de dégâts physiques','{{ first }} magic damage and {{ second }} true damage; {{ third }} physical damage']){
  const formula={terms:[{values:[35,60]},{values:[.5],stat:'ability_power'}]};
  const entry={tooltip,formulas:{first:formula,second:formula,third:formula},unresolved:[]} as unknown as AbilityEffectEntry;
  const split=tooltip.split(/(\{\{[^{}]+\}\})/g);
  const record=typedRecord(tooltip,split.map(text=>({text,damage_type:text==='{{ first }}'?'magic':text==='{{ second }}'?'true':text==='{{ third }}'?'physical':null})));
  const html=renderToStaticMarkup(<AbilityEffectText entry={entry} record={record} locale="fr"/>);
  expect(html).toContain('<span class="ability-magic-damage"><strong class="ability-formula"');
  expect(html).toContain('<span class="ability-true-damage"><strong class="ability-formula"');
  expect(html).toContain('<span class="ability-damage"><strong class="ability-formula"');
  expect(html.match(/data-stat="ability_power"/g)).toHaveLength(3);
 }
});
it('reste neutre en l’absence de segments valides, même si le texte dit magic damage',()=>{
 const tooltip='{{ amount }} magic damage',entry={tooltip,formulas:{amount:{terms:[{values:[10]}]}},unresolved:[]} as unknown as AbilityEffectEntry;
 const valid=typedRecord(tooltip,[{text:tooltip,damage_type:'magic'}]);
 const records=[undefined,typedRecord(tooltip,[{text:tooltip,damage_type:'magic'}],'conflict'),typedRecord(tooltip,[{text:tooltip,damage_type:'new'}]),typedRecord(tooltip,[{text:'autre texte',damage_type:'magic'}]),{...valid,fields:{...valid.fields,tooltip_segments:{...valid.fields.tooltip_segments!,sources:[]}}},{...valid,fields:{...valid.fields,tooltip_segments:{...valid.fields.tooltip_segments!,sources:[{...origin,source_id:'b'.repeat(64)}]}}}];
 for(const record of records){
  const html=renderToStaticMarkup(<AbilityEffectText entry={entry} record={record} locale="en"/>);
  expect(html).not.toContain('ability-magic-damage');expect(html).not.toContain('ability-damage');expect(html).toContain('10');
 }
});
it('applique le même retrait append/icônes et trim aux segments et au tooltip compilé',()=>{
 const raw='  A {{ value }}% %i:scaleAP%{{ spellmodifierdescriptionappend }}  ',tooltip=raw.replace(/\{\{\s*spellmodifierdescriptionappend\s*\}\}/gi,'').trim();
 const entry={tooltip,formulas:{value:{terms:[{values:[15]}]}},unresolved:[]} as unknown as AbilityEffectEntry;
 const record=typedRecord(raw,[{text:'  A ',damage_type:null},{text:'{{ value }}%',damage_type:'physical'},{text:' %i:scaleAP%{{ spellmodifierdescriptionappend }}  ',damage_type:null}]);
 const html=renderToStaticMarkup(<AbilityEffectText entry={entry} record={record} locale="en"/>);
 expect(html).toContain('ability-damage');expect(html).not.toContain('%i:');expect(html).not.toContain('spellmodifier');expect(html).toContain('15');expect(html).toContain('%');
});
it('ignore des segments valides rattachés à un autre tooltip compilé',()=>{
 const record=typedRecord('A {{ value }}',[{text:'A {{ value }}',damage_type:'true'}]);
 const entry={tooltip:'B {{ value }} true damage',formulas:{value:{terms:[{values:[10]}]}},unresolved:[]} as unknown as AbilityEffectEntry;
 expect(renderToStaticMarkup(<AbilityEffectText entry={entry} record={record} locale="en"/>)).not.toContain('ability-true-damage');
});
it('retire les jetons d’icônes Riot sans supprimer les pourcentages utiles',()=>{
 const entry={tooltip:'Gain +{{ speed }}% %i:scaleAS%%i:scaleAP% ; 50% permanent.',formulas:{speed:{terms:[{values:[25]}]}},unresolved:[]} as unknown as AbilityEffectEntry;
 const html=renderToStaticMarkup(<AbilityEffectText entry={entry} locale="fr"/>);
 expect(html).not.toContain('%i:');expect(html).toContain('25');expect(html).toContain('50');expect(html).toContain('%');
});
it('distingue les ratios AP/AD et signale une variable inconnue sans zéro ni nom technique',()=>{
 const entry={tooltip:'{{ damage }} damage; {{ missing }} seconds',formulas:{damage:{terms:[{values:[20,45]},{values:[1.3],stat:'attack_damage'},{values:[.4],stat:'ability_power'}]}},unresolved:['missing']} as unknown as AbilityEffectEntry;
 const html=renderToStaticMarkup(<AbilityEffectText entry={entry} locale="en"/>);
 expect(html).toContain('data-stat="ability_power"');expect(html).toContain('data-stat="attack_damage"');
 expect(html).toContain('130%');expect(html).toContain('40%');expect(html).toContain('20 / 45');expect(html).toContain('Value unavailable');expect(html).not.toContain('{{');
});
it('raccorde les segments réellement embarqués d’Ahri aux formules existantes en FR et EN',()=>{
 const manifest=parseAbilityEffects(JSON.parse(readFileSync(new URL('../../public/game-data/ability-effects.json',import.meta.url),'utf8')));
 for(const locale of ['fr','en'] as const){
  const language=locale==='fr'?'fr_FR':'en_US';
  const document=JSON.parse(readFileSync(new URL(`../../public/game-data/catalog/champions/103/${language}.json`,import.meta.url),'utf8')) as {records:CatalogRecord[]};
  const record=document.records.find(record=>record.id==='103:Q')!;
  const entry=matchingAbilityEffects(manifest,record,'16.19.1',locale)!;
  expect(entry).not.toBeNull();
  const html=renderToStaticMarkup(<AbilityEffectText record={record} entry={entry} locale={locale}/>);
  expect(html).toContain('ability-magic-damage');expect(html).toContain('ability-true-damage');
  expect(html).toContain('data-stat="ability_power"');expect(html).not.toContain('{{');
 }
});
