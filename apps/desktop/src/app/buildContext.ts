import {rankForQueue} from './buildRanks';
import type {BuildRequest,DraftSession,LcuSession,Role} from '@olc/shared';
import type {PreparationState} from './state';

export function knownRole(position:string|null|undefined):Role|null {
 const role=position?.toUpperCase();
 return role&&['TOP','JUNGLE','MIDDLE','BOTTOM','UTILITY'].includes(role)?role as Role:null;
}
export function draftDefaults(session:LcuSession,customRole:Role|null=null) {
 const draft=session.draft?.supported?session.draft:null;
 const local=draft?.allies.find(player=>player.local);
 return {platform:session.account?.platform??null,queue:draft?(draft.customGame?420:draft.queueId??null):null,
  role:knownRole(local?.position)??(draft?.customGame?customRole:null)};
}
/** Même population pour l’aperçu et l’import ; aucune identité adverse ni inférence de poste. */
export function preparationRequest(session:LcuSession,value:PreparationState,version:string):BuildRequest|null {
 const draft=session.draft?.supported?session.draft:null;
 const local=draft?.allies.find(player=>player.local);
 const championId=value.manual??local?.championId;
 const allies=draft?.allies.filter(p=>p.championId===championId)??[];
 const inspected=value.manualCell!==null?allies.find(p=>p.cellId===value.manualCell):value.manual===null?local:allies.length===1?allies[0]:undefined;
 const role=value.roleOverride??knownRole(inspected?.position)??(value.manualCell===null?draftDefaults(session,value.customRole).role:null);
 if(!championId||!role||role==='UNKNOWN'||!/^\d+\.\d+\.\d+$/.test(version))return null;
 return {champion_id:championId,patch:version.split('.').slice(0,2).join('.'),platform:value.platform,queue:value.queue,role,rank:rankForQueue(value.queue,value.rank)};
}
export function draftSelection(draft:DraftSession,ally:boolean,cellId:number):Partial<PreparationState> {
 if(!draft.supported)return {};
 const player=(ally?draft.allies:draft.enemies).find(p=>p.cellId===cellId);
 if(!player?.championId)return {};
 if(!ally)return player.locked?{matchup:{championId:player.championId,kind:'chosen'}}:{};
 return {manual:player.local?null:player.championId,manualCell:player.local?null:player.cellId,roleOverride:null};
}
