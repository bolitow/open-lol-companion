import {expect,it} from 'vitest';
import type {CatalogRecord} from '@olc/shared';
import {parseAbilityEffects,matchingAbilityEffects} from './abilityEffectCatalog';
const entry={technicalId:'AhriQ',spellPath:'Characters/Ahri/Spells/AhriQAbility/AhriQ',tooltip:'Inflige {{ totaldamage }}.',formulas:{totaldamage:{terms:[{values:[35,60,85,110,135]},{values:[.5],stat:'ability_power'}]}},unresolved:[],source:{url:'https://raw.communitydragon.org/16.19/game/data/characters/ahri/ahri.bin.json',sha256:'a'.repeat(64)}};
const manifest={schemaVersion:1,version:'16.19.1',abilities:{'fr_FR:103:Q':entry}};
const record={kind:'ability',id:'103:Q',namespace:'standard',locale:'fr_FR',fields:{technical_id:{value:'AhriQ',status:'verified'},max_rank:{value:5,status:'verified'},tooltip:{value:'Inflige {{ totaldamage }}.{{ spellmodifierdescriptionappend }}',status:'descriptive'}}} as unknown as CatalogRecord;
it('exige la même version, langue, identité et description avant d’enrichir la fiche',()=>{
 const parsed=parseAbilityEffects(manifest);
 expect(matchingAbilityEffects(parsed,record,'16.19.1','fr')).toEqual(entry);
 expect(matchingAbilityEffects(parsed,record,'16.20.1','fr')).toBeNull();
 expect(matchingAbilityEffects(parsed,record,'16.19.1','en')).toBeNull();
 expect(matchingAbilityEffects(parsed,{...record,namespace:'classic'},'16.19.1','fr')).toBeNull();
 expect(matchingAbilityEffects(parsed,{...record,fields:{...record.fields,tooltip:{...record.fields.tooltip!,value:'Autre texte'}}},'16.19.1','fr')).toBeNull();
});
it('ne réutilise pas les effets si le catalogue signale un conflit ou un rang invalide',()=>{
 for(const key of ['tooltip','technical_id','max_rank']){
  expect(matchingAbilityEffects(parseAbilityEffects(manifest),{...record,fields:{...record.fields,[key]:{...record.fields[key]!,status:'conflict'}}},'16.19.1','fr')).toBeNull();
 }
 expect(matchingAbilityEffects(parseAbilityEffects(manifest),{...record,fields:{...record.fields,max_rank:{...record.fields.max_rank!,value:NaN}}},'16.19.1','fr')).toBeNull();
});
it('refuse les formules non finies, sources hors patch et statistiques inconnues',()=>{
 const altered=(patch:Record<string,unknown>)=>({...manifest,abilities:{'fr_FR:103:Q':{...entry,...patch}}});
 expect(()=>parseAbilityEffects(altered({formulas:{totaldamage:{terms:[{values:[NaN]}]}}}))).toThrow();
 expect(()=>parseAbilityEffects(altered({formulas:{totaldamage:{terms:[{values:[1],stat:'invented'}]}}}))).toThrow();
 expect(()=>parseAbilityEffects(altered({source:{...entry.source,url:entry.source.url.replace('/16.19/','/latest/')}}))).toThrow();
});
