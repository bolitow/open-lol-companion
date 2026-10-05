import {Disclosure} from '../ui/Disclosure';
import {StatLabel,StatAmount} from './StatVisual';
import type {CatalogRecord} from '@olc/shared';
import type {Locale} from './state';
import {championsCopy} from './championsCopy';
import {AbilityPreviewButton} from './AbilityVideoPreview';
import {AbilityMetrics} from './AbilityInfo';
import {AbilityEffects} from './AbilityEffects';
import {itemStats} from './preparation';
import {formatStat} from './catalogFormat';
import {statLabels} from './preparationCopy';

export function ChampionAbilities({championId,records,locale,version,onOpen}:{championId:number;records:CatalogRecord[];locale:Locale;version:string;onOpen:(record:CatalogRecord)=>void}){
 const t=championsCopy[locale],self=records.find(record=>record.kind==='champion'&&record.id===String(championId));
 const stats=self?itemStats(self).filter(stat=>statLabels[stat.key]&&formatStat(stat.value,stat.unit,locale)!==null):[];
 return <div className="champion-ability-panel">
  {stats.length>0&&<Disclosure className="champion-base-stats" label={t.baseStats}><dl className="champion-stat-strip">{stats.map(stat=><div key={stat.key}><dt><StatLabel statKey={stat.key} locale={locale}/></dt><dd><StatAmount statKey={stat.key}>{formatStat(stat.value,stat.unit,locale)}</StatAmount></dd></div>)}</dl></Disclosure>}
  <div className="champion-abilities">{['passive','Q','W','E','R'].map((slot,index)=>{
   const ability=records.find(record=>record.kind==='ability'&&record.id===`${championId}:${slot}`);
   return ability?<article key={slot}><AbilityPreviewButton record={ability} locale={locale} onOpen={onOpen}/><div className="champion-ability-content"><header className="champion-ability-heading"><span className="champion-ability-key">{index===0?t.passive:locale==='fr'?['','A','Z','E','R'][index]:slot}</span><h3>{ability.name}</h3></header><AbilityMetrics champion={self} record={ability} locale={locale}/><AbilityEffects record={ability} version={version} locale={locale} compact onExpand={()=>onOpen(ability)}/></div></article>:null;
  })}</div>
 </div>;
}
