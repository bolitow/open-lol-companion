import {useEffect,useState} from 'react';
import type {CatalogRecord,DraftPlayer} from '@olc/shared';
import {Icon} from '../ui/Icon';
import {CatalogButton} from './GameDetails';
import {loadChampionAbilities} from './catalog';
import {catalogDescription} from './preparation';
import {championDetails} from './draft';
import type {Copy} from './copy';
import type {Locale} from './state';

export function ChampionCard({player,locale,t,selected,onSelect}:{player?:DraftPlayer;locale:Locale;t:Copy;selected:boolean;onSelect:()=>void}){
 const champion=championDetails(player?.championId??null,locale),[inspect,setInspect]=useState(false),[details,setDetails]=useState<{id:number;locale:Locale;record:CatalogRecord}|null>(null);
 const championId=player?.championId;
 useEffect(()=>{
  let active=true;
  if(inspect&&championId)loadChampionAbilities(locale,championId).then(records=>{
   const record=records.find(r=>r.kind==='champion'),passive=records.find(r=>r.id===`${championId}:passive`);
   const description=passive?catalogDescription(passive):null;
   if(active&&record)setDetails({id:championId,locale,record:{...record,description:description?`${passive?.name} · ${description}`:record.description}});
  },()=>{});
  return()=>{active=false};
 },[inspect,championId,locale]);
 const name=champion?.name??(championId?t.draft.unknown:t.game.waiting),status=player?.locked?t.draft.locked:championId?t.draft.prepick:t.game.waiting;
 const contents=<><span className="card-badges">{player?.local&&<span>{t.draft.you}</span>}{player?.position&&<small>{t.draft.roles[player.position]}</small>}</span><span className="card-caption"><strong>{name}</strong><span>{player?.locked&&<Icon name="check" size={12}/>} {status}</span></span></>;
 const className=`live-champion-card ${player?.locked?'locked':'prepick'} ${player?.local?'local':''} ${player?.acting?'acting':''} ${selected?'inspected':''}`;
 if(!champion||!championId)return <div className={className} aria-label={`${name} · ${status}`}><span className="champion-fallback" aria-hidden="true"><Icon name="shield" size={30}/></span>{contents}</div>;
 const record:CatalogRecord=details?.id===championId&&details.locale===locale?{...details.record,icon:champion.image}:{kind:'champion',id:String(championId),namespace:'standard',locale:locale==='fr'?'fr_FR':'en_US',name,icon:champion.image,description:null,fields:{},stats:{},effects:[],coverage:{source_fields:0,normalized_fields:0,unmapped_fields:[],issues:[]}};
 return <div className="champion-card-slot" onPointerEnter={()=>setInspect(true)} onFocus={()=>setInspect(true)}><CatalogButton record={record} locale={locale} selected={selected} selectedLabel={t.draft.inspected} accessibleLabel={`${name} · ${status}${player?.local?` · ${t.draft.you}`:''}${player?.position?` · ${t.draft.roles[player.position]}`:''}${selected?` · ${t.draft.inspected}`:''} · ${t.draft.browse}`} actionLabel={t.draft.browse} onOpen={onSelect} className={className}>{contents}</CatalogButton></div>;
}
