import type {CatalogRecord} from '@olc/shared';
import type {Locale} from './state';
export const abilityCopy={
 fr:{cooldown:'Récupération',cost:'Coût',range:'Portée',description:'Fonctionnement du sort',parameters:'Paramètres du sort',mana:'Mana',energy:'Énergie',health:'PV',perRocket:' / roquette',extra:'Autre variation de ressource non chiffrée',ranks:'Valeurs par rang',missing:'Effets chiffrés indisponibles dans le catalogue'},
 en:{cooldown:'Cooldown',cost:'Cost',range:'Range',description:'How the ability works',parameters:'Ability parameters',mana:'Mana',energy:'Energy',health:'Health',perRocket:' / rocket',extra:'Additional resource change not quantified',ranks:'Values by rank',missing:'Effect values unavailable in the catalog'},
};
type ResourceStat='mana'|'energy'|'health';
const resourceStat=(value:string):ResourceStat|null=>({mana:'mana',energy:'energy','énergie':'energy',health:'health',pv:'health',hp:'health'} as Record<string,ResourceStat>)[value.trim().toLocaleLowerCase()]??null;
/** Le coût numérique suit son propre marqueur, pas les autres ressources du texte. */
function costStat(record:CatalogRecord,champion?:CatalogRecord):ResourceStat|null{
 const field=record.fields.resource;
 if(!field||!['descriptive','verified','derived'].includes(field.status)||typeof field.value!=='string')return null;
 const text=field.value;
 const literal=text.match(/\{\{\s*cost\s*\}\}\s*(?:pts? (?:de |d['’]))?(mana|energy|énergie|health|pv|hp)\b/i);
 if(literal)return resourceStat(literal[1]??'');
 // Les références génériques ne sont résolues qu'avec la fiche du même champion.
 if(!/\{\{\s*cost\s*\}\}\s*\{\{\s*abilityresourcename\s*\}\}/i.test(text)||champion?.kind!=='champion'||record.id.split(':')[0]!==champion.id)return null;
 const resource=champion.fields.resource;
 return resource&&['descriptive','verified','derived'].includes(resource.status)&&typeof resource.value==='string'?resourceStat(resource.value):null;
}
function costSuffix(record:CatalogRecord,locale:Locale):string{
 const field=record.fields.resource;
 if(!field||!['descriptive','verified','derived'].includes(field.status)||typeof field.value!=='string')return '';
 // La fréquence doit suivre directement le coût, jamais un supplément de PV distinct.
 const match=field.value.match(/\{\{\s*cost\s*\}\}\s*(?:(?:pts? (?:de |d['’]))?(?:mana|energy|énergie|health|pv|hp)\b|\{\{\s*abilityresourcename\s*\}\})\s+(per second|par sec(?:onde)?|per rocket|par roquette)\b/i);
 return match?(/second|sec/i.test(match[1]??'')?' /s':abilityCopy[locale].perRocket):'';
}
export function abilityMetrics(record:CatalogRecord,locale:Locale,champion?:CatalogRecord){
 if(record.kind!=='ability')return [];
 const t=abilityCopy[locale],number=new Intl.NumberFormat(locale,{maximumFractionDigits:2});
 return (['cooldown','cost','range'] as const).flatMap(key=>{
  const field=record.fields[key],unit={cooldown:'seconds',cost:'resource_points',range:'game_units'}[key];
  if(!field||!['verified','derived'].includes(field.status)||field.unit!==unit||!Array.isArray(field.value)||!field.value.length||!field.value.every(n=>typeof n==='number'&&Number.isFinite(n)&&n>=0))return [];
  const values=field.value as number[],constant=values.every(n=>n===values[0]);
  const resource=record.fields.resource;
  const extra=key==='cost'&&resource?.status==='descriptive'&&typeof resource.value==='string'&&[...resource.value.matchAll(/\{\{\s*(.*?)\s*\}\}/g)].some(match=>!['cost','abilityresourcename'].includes((match[1]??'').trim()));
  const statKey=key==='cost'?(costStat(record,champion)??key):key;
  return [{key,statKey,label:statKey==='mana'||statKey==='energy'||statKey==='health'?t[statKey]:t[key],value:(constant?values.slice(0,1):values).map(n=>number.format(n)).join(' / ')+(key==='cooldown'?' s':key==='cost'?costSuffix(record,locale):''),byRank:!constant,note:extra?t.extra:null}];
 });
}
// Texte uniquement : les balises et formules du fournisseur ne sont jamais exécutées.
export function abilitySegments(text:string){
 const statWords="(?:bonus attack damage|total attack damage|attack damage|dégâts d['’]attaque|ability power|puissance|magic resistance|résistance magique|armure|armor|AD|AP)";
 const statExpression="(?:\\d+(?:[.,]\\d+)?\\s?%?\\s*(?:de (?:la |l['’])?)?)?"+statWords;
 const pattern=new RegExp('('+statExpression+"|boucliers?|shields?|soign\\p{L}*|soins?|heals?|healing|(?:récupère|rend|restaure)(?: immédiatement)? (?:des |les |ses )?PV|(?:recovers?|restor(?:es?|ing)) health|\\d+(?:[.,]\\d+)?(?:\\s?%)?)(?![\\p{L}])",'giu');
 const parts:{text:string;tone?:'shield'|'heal'|'value';statKey?:string}[]=[];let end=0;
 // Sans lookbehind : compatible avec le WebKit du minimum macOS 13.0.
 for(const match of text.matchAll(pattern)){
  if(/\p{L}$/u.test(text.slice(0,match.index)))continue;
  if(match.index>end)parts.push({text:text.slice(end,match.index)});
  const token=match[0];
  const statKey=/ability power|puissance|\bAP$/i.test(token)?'ability_power':/attack damage|dégâts d['’]attaque|\bAD$/i.test(token)?'attack_damage':/magic resistance|résistance magique/i.test(token)?'magic_resistance':/armure|armor/i.test(token)?'armor':undefined;
  const tone=/^\d/.test(token)?'value':/bouclier|shield/i.test(token)?'shield':'heal';
  parts.push({text:token,tone,statKey});end=match.index+token.length;
 }
 if(end<text.length)parts.push({text:text.slice(end)});return parts;
}
