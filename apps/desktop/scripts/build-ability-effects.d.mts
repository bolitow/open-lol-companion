import type {AbilityFormula} from './ability-calculations.mjs';
export interface CompiledAbility {technicalId:string;spellPath:string;tooltip:string;formulas:Record<string,AbilityFormula>;unresolved:string[];}
export function compileAbility(record:unknown,bin:unknown,championKey:string):CompiledAbility|null;
