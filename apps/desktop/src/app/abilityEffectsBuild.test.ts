import {expect,it} from 'vitest';
import {compileAbility} from '../../scripts/build-ability-effects.mjs';
import fixtures from './__fixtures__/ability-calculations.json';
const record={kind:'ability',id:'103:Q',namespace:'standard',fields:{technical_id:{value:'AhriQ'},max_rank:{value:5},tooltip:{value:'{{ totaldamage }} dégâts ; {{ unknown }} secondes.{{ spellmodifierdescriptionappend }}'}}};
const path='Characters/Ahri/Spells/AhriQAbility/AhriQ';
const bin={'Characters/Ahri/CharacterRecords/Root':{spellNames:['AhriQAbility/AhriQ']},[path]:{mScriptName:'AhriQ',mSpell:fixtures.AhriQ.spell}};
it('compile le tooltip exact et garde les inconnues séparées sans les remplacer par zéro',()=>{
 expect(compileAbility(record,bin,'Ahri')).toEqual({technicalId:'AhriQ',spellPath:path,tooltip:'{{ totaldamage }} dégâts ; {{ unknown }} secondes.',formulas:{totaldamage:{terms:[{values:[35,60,85,110,135]},{values:[.5],stat:'ability_power'}]}},unresolved:['unknown']});
});
it('refuse un autre mode, une ancienne identité ou un sort alternatif',()=>{
 expect(compileAbility({...record,namespace:'classic'},bin,'Ahri')).toBeNull();
 expect(compileAbility({...record,fields:{...record.fields,technical_id:{value:'Wrong'}}},bin,'Ahri')).toBeNull();
 expect(compileAbility(record,{[path]:bin[path]},'Ahri')).toBeNull();
});
