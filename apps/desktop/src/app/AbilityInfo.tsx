import {Disclosure} from '../ui/Disclosure';
import {StatGlyph,StatAmount,StatMention} from './StatVisual';
import type {CatalogRecord} from '@olc/shared';
import type {Locale} from './state';
import {abilityCopy,abilityMetrics,abilitySegments} from './abilityPresentation';
import {catalogDescription} from './preparation';
import './abilityInfo.css';
export function AbilityMetrics({record,locale,champion}:{record:CatalogRecord;locale:Locale;champion?:CatalogRecord}){
 const metrics=abilityMetrics(record,locale,champion),t=abilityCopy[locale];
 return metrics.length?<dl className="ability-metrics" aria-label={t.parameters}>{metrics.map(metric=><div key={metric.key} className="ability-metric" data-metric={metric.key}><dt><span className="ability-metric-icon"><StatGlyph statKey={metric.statKey} size={14}/></span><span>{metric.label}</span></dt><dd title={metric.byRank?t.ranks:undefined}><StatAmount statKey={metric.statKey}>{metric.value}</StatAmount></dd>{metric.note&&<small>{metric.note}</small>}</div>)}</dl>:null;
}
export function AbilityText({text}:{text:string}){return <>{abilitySegments(text).map((part,index)=>part.statKey?<StatMention key={index} statKey={part.statKey}>{part.text}</StatMention>:part.tone?<strong key={index} className={`ability-${part.tone}`}>{part.text}</strong>:part.text)}</>}
export function AbilityDescription({record,locale,compact=false,expanded=false}:{record:CatalogRecord;locale:Locale;compact?:boolean;expanded?:boolean}){
 const text=catalogDescription(record),t=abilityCopy[locale];
 if(!text)return null;
 if(expanded)return <p className="ability-description is-expanded"><AbilityText text={text}/></p>;
 if(compact)return <p className="ability-description"><AbilityText text={text.length>230?text.slice(0,230)+'…':text}/></p>;
 return <Disclosure className="ability-description" label={t.description}><p><AbilityText text={text}/></p></Disclosure>;
}
