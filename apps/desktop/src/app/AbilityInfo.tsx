import {Disclosure} from '../ui/Disclosure';
import {StatGlyph,StatAmount,StatMention} from './StatVisual';
import type {CatalogDamageType,CatalogRecord} from '@olc/shared';
import {catalogTooltipSegments,damageTone,sliceTooltipSegments} from './abilityTooltip';
import type {Locale} from './state';
import {abilityCopy,abilityMetrics,abilitySegments} from './abilityPresentation';
import {catalogDescription} from './preparation';
import './abilityInfo.css';
export function AbilityMetrics({record,locale,champion}:{record:CatalogRecord;locale:Locale;champion?:CatalogRecord}){
 const metrics=abilityMetrics(record,locale,champion),t=abilityCopy[locale];
 return metrics.length?<dl className="ability-metrics" aria-label={t.parameters}>{metrics.map(metric=><div key={metric.key} className="ability-metric" data-metric={metric.key}><dt><span className="ability-metric-icon"><StatGlyph statKey={metric.statKey} size={14}/></span><span>{metric.label}</span></dt><dd title={metric.byRank?t.ranks:undefined}><StatAmount statKey={metric.statKey}>{metric.value}</StatAmount></dd>{metric.note&&<small>{metric.note}</small>}</div>)}</dl>:null;
}
export function AbilityText({text,damageType}:{text:string;damageType?:CatalogDamageType|null}){
 const content=abilitySegments(text).map((part,index)=>part.statKey?<StatMention key={index} statKey={part.statKey}>{part.text}</StatMention>:part.tone&&!damageType?<strong key={index} className={`ability-${part.tone}`}>{part.text}</strong>:part.text);
 return damageType?<strong className={`ability-${damageTone(damageType)}`}>{content}</strong>:<>{content}</>;
}
export function AbilityDescription({record,locale,compact=false,expanded=false}:{record:CatalogRecord;locale:Locale;compact?:boolean;expanded?:boolean}){
 const text=catalogDescription(record),t=abilityCopy[locale];
 if(!text)return null;
 // Une description courte différente du tooltip ne reçoit jamais ses types.
 const segments=catalogTooltipSegments(record),matching=segments?.map(segment=>segment.text).join('')===text?segments:null;
 const visible=compact?text.slice(0,230):text,content=matching?sliceTooltipSegments(matching,0,visible.length).map((segment,index)=><AbilityText key={index} text={segment.text} damageType={segment.damage_type}/>):<AbilityText text={visible}/>;
 if(expanded)return <p className="ability-description is-expanded">{content}</p>;
 if(compact)return <p className="ability-description">{content}{text.length>230?'…':null}</p>;
 return <Disclosure className="ability-description" label={t.description}><p>{content}</p></Disclosure>;
}
