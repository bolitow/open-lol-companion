import {CATALOG_DAMAGE_TYPES,CATALOG_TOOLTIP_SEGMENTS_FIELD,type CatalogDamageType,type CatalogRecord,type CatalogTooltipSegment} from '@olc/shared';

const object=(value:unknown):value is Record<string,unknown>=>value!==null&&typeof value==='object'&&!Array.isArray(value);
const readable=(status:unknown)=>status==='descriptive'||status==='verified'||status==='derived';
const sourceKey=(value:unknown):string|null=>object(value)&&typeof value.source_id==='string'&&/^[a-f0-9]{64}$/.test(value.source_id)&&typeof value.pointer==='string'&&value.pointer.startsWith('/data/')&&value.pointer.endsWith('/tooltip')?`${value.source_id}:${value.pointer}`:null;

/** Le champ est exploitable uniquement pour le texte et la provenance exacts de son tooltip. */
export function catalogTooltipSegments(record?:CatalogRecord):CatalogTooltipSegment[]|null{
 if(!record||!['ability','summoner_spell'].includes(record.kind)||record.namespace!=='standard')return null;
 const tooltip=record.fields?.tooltip,field=record.fields?.[CATALOG_TOOLTIP_SEGMENTS_FIELD];
 if(!tooltip||!field||!readable(tooltip.status)||typeof tooltip.value!=='string'||field.status!=='derived'||field.unit!==null||!Array.isArray(field.value)||!Array.isArray(field.sources)||!Array.isArray(tooltip.sources)||!field.sources.length||field.sources.length!==tooltip.sources.length)return null;
 const origins=tooltip.sources.map(sourceKey),derived=field.sources.map(sourceKey);
 if(origins.some(key=>key===null)||derived.some(key=>key===null)||new Set(origins).size!==origins.length||new Set(derived).size!==derived.length||derived.some(key=>!origins.includes(key)))return null;
 const segments:CatalogTooltipSegment[]=[];
 for(const value of field.value){
  if(!object(value)||typeof value.text!=='string'||!(value.damage_type===null||CATALOG_DAMAGE_TYPES.some(type=>type===value.damage_type)))return null;
  segments.push({text:value.text,damage_type:value.damage_type as CatalogDamageType|null});
 }
 return segments.map(segment=>segment.text).join('')===tooltip.value?segments:null;
}

export const damageTone=(type:CatalogDamageType)=>({physical:'damage',magic:'magic-damage',true:'true-damage'} as const)[type];
const append=/\{\{\s*spellmodifierdescriptionappend\s*\}\}/gi;
const icons=/%i:[a-z0-9_]+%/gi;
export const compiledTooltip=(text:string)=>text.replace(append,'').trim();
export const displayTooltip=(text:string)=>text.replace(icons,'');

/** Découpe par positions : un jeton supprimé peut chevaucher une frontière de segment. */
export function sliceTooltipSegments(segments:readonly CatalogTooltipSegment[],start:number,end:number):CatalogTooltipSegment[]{
 let offset=0;
 return segments.flatMap(segment=>{
  const from=Math.max(0,start-offset),to=Math.min(segment.text.length,end-offset);offset+=segment.text.length;
  return to>from?[{text:segment.text.slice(from,to),damage_type:segment.damage_type}]:[];
 });
}
function strip(segments:CatalogTooltipSegment[],pattern:RegExp):CatalogTooltipSegment[]{
 const text=segments.map(segment=>segment.text).join(''),result:CatalogTooltipSegment[]=[];let offset=0;
 for(const match of text.matchAll(pattern)){
  result.push(...sliceTooltipSegments(segments,offset,match.index));offset=match.index+match[0].length;
 }
 return [...result,...sliceTooltipSegments(segments,offset,text.length)];
}
export function effectTooltipSegments(record:CatalogRecord|undefined,tooltip:string):CatalogTooltipSegment[]|null{
 const segments=catalogTooltipSegments(record);
 if(!segments||typeof record?.fields.tooltip?.value!=='string'||compiledTooltip(record.fields.tooltip.value)!==tooltip)return null;
 const withoutAppend=strip(segments,append),text=withoutAppend.map(segment=>segment.text).join('');
 return strip(sliceTooltipSegments(withoutAppend,text.length-text.trimStart().length,text.trimEnd().length),icons);
}
