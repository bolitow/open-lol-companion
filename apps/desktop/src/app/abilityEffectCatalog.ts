import type {CatalogRecord} from '@olc/shared';
import type {Locale} from './state';
import type {CompiledAbility} from '../../scripts/build-ability-effects.mjs';
import {compiledTooltip} from './abilityTooltip';
export type AbilityEffectEntry=CompiledAbility&{source:{url:string;sha256:string}};
export interface AbilityEffectManifest{version:string;abilities:Record<string,AbilityEffectEntry>;}
const object=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
const invalid=()=>new Error('ability-effects-invalid');
export function parseAbilityEffects(value:unknown):AbilityEffectManifest{
 if(!object(value)||value.schemaVersion!==1||typeof value.version!=='string'||!/^\d+\.\d+\.\d+$/.test(value.version)||!object(value.abilities))throw invalid();
 const patch=value.version.split('.').slice(0,2).join('.');
 for(const [id,entry] of Object.entries(value.abilities)){
  if(!/^(fr_FR|en_US):[1-9]\d*:[QWER]$/.test(id)||!object(entry)||typeof entry.technicalId!=='string'||typeof entry.spellPath!=='string'||typeof entry.tooltip!=='string'||!object(entry.formulas)||!Array.isArray(entry.unresolved)||!entry.unresolved.every(key=>typeof key==='string')||!object(entry.source)||typeof entry.source.url!=='string'||typeof entry.source.sha256!=='string'||! /^[a-f0-9]{64}$/.test(entry.source.sha256))throw invalid();
  const base=`https://raw.communitydragon.org/${patch}/game/data/characters/`;
  const source=entry.source.url.startsWith(base)?/^([a-z0-9]+)\/\1\.bin\.json$/.exec(entry.source.url.slice(base.length)):null;
  if(!source||!entry.spellPath.toLowerCase().startsWith(`characters/${source[1]}/spells/`))throw invalid();
  for(const formula of Object.values(entry.formulas)){
   if(!object(formula)||!Array.isArray(formula.terms)||formula.terms.length<1||formula.terms.length>32)throw invalid();
   for(const term of formula.terms){
    if(!object(term)||!Array.isArray(term.values)||term.values.length<1||term.values.length>6||!term.values.every(number=>typeof number==='number'&&Number.isFinite(number)&&Math.abs(number)<1e9)||term.stat!==undefined&&!['ability_power','attack_damage','bonus_attack_damage'].includes(String(term.stat)))throw invalid();
   }
  }
 }
 return value as unknown as AbilityEffectManifest;
}
export function matchingAbilityEffects(manifest:AbilityEffectManifest,record:CatalogRecord,version:string,locale:Locale):AbilityEffectEntry|null{
 if(manifest.version!==version||record.kind!=='ability'||record.namespace!=='standard'||record.locale!==(locale==='fr'?'fr_FR':'en_US'))return null;
 if(['technical_id','max_rank'].some(key=>!['verified','derived'].includes(record.fields[key]?.status??''))||!['descriptive','verified','derived'].includes(record.fields.tooltip?.status??''))return null;
 const entry=manifest.abilities[`${record.locale}:${record.id}`],tooltip=record.fields.tooltip?.value;
 if(!entry||entry.technicalId!==record.fields.technical_id?.value||typeof tooltip!=='string'||entry.tooltip!==compiledTooltip(tooltip))return null;
 const ranks=record.fields.max_rank?.value;
 if(typeof ranks!=='number'||!Number.isInteger(ranks)||ranks<1||ranks>6)return null;
 if(Object.values(entry.formulas).some(formula=>formula.terms.some(term=>term.values.length!==1&&term.values.length!==ranks)))return null;
 return entry;
}
let cached:Promise<AbilityEffectManifest>|null=null;
export async function loadAbilityEffects(record:CatalogRecord,version:string,locale:Locale){
 if(!cached){
  cached=(async()=>{
   const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),10000);
   try{const response=await fetch('/game-data/ability-effects.json',{signal:controller.signal});if(!response.ok)throw invalid();return parseAbilityEffects(await response.json())}finally{clearTimeout(timer)}
  })().catch(error=>{cached=null;throw error});
 }
 return matchingAbilityEffects(await cached,record,version,locale);
}
