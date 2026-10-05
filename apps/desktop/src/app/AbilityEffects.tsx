import {Fragment,useEffect,useState} from 'react';
import type {CatalogRecord} from '@olc/shared';
import type {AbilityFormula} from '../../scripts/ability-calculations.mjs';
import {loadAbilityEffects,type AbilityEffectEntry} from './abilityEffectCatalog';
import {AbilityText,AbilityDescription} from './AbilityInfo';
import {abilityDamageTone} from './abilityPresentation';
import {StatGlyph,StatAmount} from './StatVisual';
import type {Locale} from './state';
import './abilityEffects.css';

export const effectCopy={
 fr:{title:'Effets et ratios',missing:'Effets chiffrés encore indisponibles',partial:'Certains effets restent à compléter.',unknown:'Valeur indisponible',loading:'Chargement des effets…',ranks:'Valeurs par rang du sort',raw:'Valeurs de base et ratios, avant résistances et modificateurs.',more:'Tous les effets',bonusAD:'AD bonus',ap:'AP',ad:'AD total'},
 en:{title:'Effects and ratios',missing:'Effect values not yet available',partial:'Some effects are still incomplete.',unknown:'Value unavailable',loading:'Loading effects…',ranks:'Values by ability rank',raw:'Base values and ratios, before resistances and modifiers.',more:'All effects',bonusAD:'bonus AD',ap:'AP',ad:'total AD'},
};
export function AbilityFormulaView({formula,locale}:{formula:AbilityFormula;locale:Locale}){
 const t=effectCopy[locale],format=new Intl.NumberFormat(locale,{maximumFractionDigits:3});
 return <strong className="ability-formula" title={t.ranks}>{formula.terms.length>1?'(':null}{formula.terms.map((term,index)=>{
  const key=term.stat==='bonus_attack_damage'?'attack_damage':term.stat;
  return <Fragment key={index}>{index>0&&<span className="ability-formula-plus"> + </span>}{key?<StatAmount statKey={key}>{term.values.map(value=>format.format(value*100)).join(' / ')}% <StatGlyph statKey={key} size={14}/>{term.stat==='ability_power'?t.ap:term.stat==='bonus_attack_damage'?t.bonusAD:t.ad}</StatAmount>:term.values.map(value=>format.format(value)).join(' / ')}</Fragment>;
 })}{formula.terms.length>1?')':null}</strong>;
}
export function AbilityEffectText({entry,locale}:{entry:AbilityEffectEntry;locale:Locale}){
 const parts=entry.tooltip.replace(/%i:[a-z0-9_]+%/gi,'').split(/(\{\{[^{}]+\}\})/g),t=effectCopy[locale];
 return <>{parts.map((text,index)=>{
  if(!text.startsWith('{{'))return <AbilityText key={index} text={text}/>;
  const key=text.slice(2,-2).trim().toLowerCase(),formula=entry.formulas[key];
  const following=parts[index+1]??'',preceding=parts[index-1]??'';
  const tone=abilityDamageTone(following)??(/shield|bouclier/i.test(preceding.slice(-50))?'shield':/heal|soin|rend|récupère/i.test(preceding.slice(-50))?'heal':'');
  return formula?<span key={index} className={tone?`ability-${tone}`:undefined}><AbilityFormulaView formula={formula} locale={locale}/></span>:<abbr key={index} className="ability-effect-unknown" title={t.unknown} aria-label={t.unknown}>—</abbr>;
 })}</>;
}
export function AbilityEffects({record,version,locale,compact=false,onExpand}:{record:CatalogRecord;version:string;locale:Locale;compact?:boolean;onExpand?:()=>void}){
 const t=effectCopy[locale],[load,setLoad]=useState<{id:string;entry:AbilityEffectEntry|null}|null>(null);
 const identity=`${record.namespace}:${record.locale}:${record.id}:${version}`;
 useEffect(()=>{let active=true;setLoad(null);loadAbilityEffects(record,version,locale).then(entry=>{if(active)setLoad({id:identity,entry})},()=>{if(active)setLoad({id:identity,entry:null})});return()=>{active=false}},[record,version,locale,identity]);
 const entry=load?.id===identity?load.entry:null;
 if(!load)return <p className="ability-effect-note" role="status">{t.loading}</p>;
 if(!entry||!Object.keys(entry.formulas).length)return <><AbilityDescription record={record} locale={locale} expanded={!compact}/><p className="ability-effect-note">{t.missing}</p></>;
 return <section className={`ability-effects ${compact?'is-compact':''}`} aria-label={t.title}>
  <p className="ability-effects-text"><AbilityEffectText entry={entry} locale={locale}/></p>
  {entry.unresolved.length>0&&<small className="ability-effect-note">{t.partial}</small>}
  {compact&&onExpand?<button className="ability-effects-more" onClick={event=>{event.currentTarget.focus({preventScroll:true});onExpand()}}>{t.more} <span aria-hidden="true">↗</span></button>:<small className="ability-effect-note">{t.ranks} · {t.raw}</small>}
 </section>;
}
