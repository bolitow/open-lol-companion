import {expect,it} from 'vitest';
import {resolveAbilityVariable,calculationHash} from '../../scripts/ability-calculations.mjs';
import fixtures from './__fixtures__/ability-calculations.json';

it('résout Ahri Q avec ses cinq rangs et son ratio AP, sans rang zéro',()=>{
 expect(resolveAbilityVariable(fixtures.AhriQ.spell,'totaldamage',5)).toEqual({terms:[{values:[35,60,85,110,135]},{values:[.5],stat:'ability_power'}]});
});
it('résout le bouclier et la durée de Lux sans attribuer un effectBurn arbitraire',()=>{
 expect(resolveAbilityVariable(fixtures.LuxPrismaticWave.spell,'totalshieldtt',5)).toEqual({terms:[{values:[40,55,70,85,100]},{values:[.4],stat:'ability_power'}]});
 expect(resolveAbilityVariable(fixtures.LuxPrismaticWave.spell,'shieldduration',5)).toEqual({terms:[{values:[2.5]}]});
});
it('garde les ratios AD et AP distincts pour Ezreal Q',()=>{
 expect(resolveAbilityVariable(fixtures.EzrealQ.spell,'damage',5)).toEqual({terms:[{values:[20,45,70,95,120]},{values:[1.3],stat:'attack_damage'},{values:[.4],stat:'ability_power'}]});
});
it('refuse la formule entière si un terme ou modificateur est inconnu',()=>{
 const spell={mSpellCalculations:{Damage:{__type:'GameCalculation',mFormulaParts:[{__type:'NumberCalculationPart',mNumber:50},{__type:'UnknownPart',value:20}]}}};
 expect(resolveAbilityVariable(spell,'damage',5)).toBeNull();
 expect(resolveAbilityVariable({mSpellCalculations:{Damage:{...spell.mSpellCalculations.Damage,mFormulaParts:[{__type:'NumberCalculationPart',mNumber:50}],mMultiplier:{__type:'UnknownPart'}}}},'damage',5)).toBeNull();
});
it('refuse les données absentes, non finies ou les rangs incomplets',()=>{
 expect(resolveAbilityVariable({},'damage',5)).toBeNull();
 expect(resolveAbilityVariable({DataValues:[{name:'damage',values:[0,10,20]}]},'damage',5)).toBeNull();
 expect(resolveAbilityVariable({DataValues:[{name:'damage',values:[0,Infinity,10,20,30,40]}]},'damage',5)).toBeNull();
});
it('applique un multiplicateur explicite sans exécuter de texte',()=>{
 expect(resolveAbilityVariable({DataValues:[{name:'slow',values:[-.2,-.2,-.2,-.2,-.2,-.2]}]},'slow*-100',5)).toEqual({terms:[{values:[20]}]});
 expect(resolveAbilityVariable(fixtures.AhriQ.spell,'totaldamage;alert(1)',5)).toBeNull();
});
it('résout les références hachées et rejette les cycles et les conditions inconnues',()=>{
 const spell={DataValues:[{name:calculationHash('BaseDamage'),values:[0,10,20,30,40,50]}],mSpellCalculations:{Damage:{__type:'GameCalculation',mFormulaParts:[{__type:'NamedDataValueCalculationPart',mDataValue:'BaseDamage'}]},Twice:{__type:'GameCalculationModified',mModifiedGameCalculation:'Damage',mMultiplier:{__type:'NumberCalculationPart',mNumber:2}},Loop:{__type:'GameCalculationModified',mModifiedGameCalculation:'Loop'}}};
 expect(resolveAbilityVariable(spell,'twice',5)).toEqual({terms:[{values:[20,40,60,80,100]}]});
 expect(resolveAbilityVariable(spell,'loop',5)).toBeNull();
 expect(resolveAbilityVariable({mSpellCalculations:{Damage:{...spell.mSpellCalculations.Damage,mDisplayAsPercent:true}}},'damage',5)).toBeNull();
});
it('distingue AD bonus et total et refuse les enums ou le nouveau système non interprétés',()=>{
 const spell=(part:Record<string,unknown>)=>({mSpellCalculations:{Damage:{__type:'GameCalculation',mFormulaParts:[{__type:'StatByCoefficientCalculationPart',mCoefficient:.5,mStat:2,...part}]}}});
 expect(resolveAbilityVariable(spell({mStatFormula:2}),'damage',5)).toEqual({terms:[{values:[.5],stat:'bonus_attack_damage'}]});
 expect(resolveAbilityVariable(spell({mStatFormula:1}),'damage',5)).toBeNull();
 expect(resolveAbilityVariable(spell({UseNewStats:true}),'damage',5)).toBeNull();
 expect(resolveAbilityVariable(spell({mStat:999}),'damage',5)).toBeNull();
});
it('ne traite pas les valeurs falsy mal formées comme un défaut omis',()=>{
 const spell=(extra:Record<string,unknown>)=>({mSpellCalculations:{Damage:{__type:'GameCalculation',mFormulaParts:[{__type:'NumberCalculationPart',mNumber:50}],...extra}}});
 for(const mMultiplier of [0,false,'',null,NaN])expect(resolveAbilityVariable(spell({mMultiplier}),'damage',5)).toBeNull();
 expect(resolveAbilityVariable(spell({mFormulaParts:[{__type:'StatByCoefficientCalculationPart',mCoefficient:.5,mStat:null}]}),'damage',5)).toBeNull();
});
