export interface AbilityTerm {values:number[];stat?:'ability_power'|'attack_damage'|'bonus_attack_damage';}
export interface AbilityFormula {terms:AbilityTerm[];}
export function calculationHash(name:string):string;
export function resolveAbilityVariable(spell:unknown,name:string,ranks:number):AbilityFormula|null;
