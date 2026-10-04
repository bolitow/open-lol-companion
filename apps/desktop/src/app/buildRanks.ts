import type {PlayerProfile,PlayerRequest} from '@olc/shared';
import type {Locale} from './state';
import {buildCopy} from './buildCopy';
import {playerKey} from './playerStore';
export const simpleRanks=['IRON','BRONZE','SILVER','GOLD','PLATINUM','EMERALD','DIAMOND','MASTER','GRANDMASTER','CHALLENGER'] as const;
export const buildRankOptions=['ALL',...simpleRanks.slice(0,8).map(rank=>`${rank}_PLUS`),...simpleRanks];
export const PROFILE_RANK_MAX_AGE_MS=30*60*1000;
export function rankLabel(rank:string,locale:Locale):string {
 if(rank==='ALL')return buildCopy[locale].all;
 const index=simpleRanks.indexOf(rank.replace(/_PLUS$/,'') as typeof simpleRanks[number]);
 return index<0?rank:`${buildCopy[locale].rankTiers[index]}${rank.endsWith('_PLUS')?' +':''}`;
}
export const rankedQueue=(queue:number)=>queue===420||queue===440;
export const rankForQueue=(queue:number,rank:string)=>rankedQueue(queue)?rank:'ALL';
/** Profil du compte actif uniquement ; aucune recherche ni estimation de classement. */
export function defaultPlayerRank(account:PlayerRequest|null,profile:PlayerProfile|null,now=Date.now()):string {
 if(!account||!profile||profile.source!=='lcu'||playerKey(account)!==playerKey(profile)||!Number.isFinite(profile.fetched_at)||profile.fetched_at*1000>now+60000||now-profile.fetched_at*1000>=PROFILE_RANK_MAX_AGE_MS)return 'EMERALD_PLUS';
 for(const queue of [420,440]){
  const tier=profile.ranks.find(rank=>rank.queue_id===queue&&rank.status==='ranked')?.tier;
  if(tier&&simpleRanks.includes(tier as typeof simpleRanks[number]))return ['GRANDMASTER','CHALLENGER'].includes(tier)?'MASTER_PLUS':tier;
 }
 return 'EMERALD_PLUS';
}

/** Une échéance locale, jamais un polling ni une requête réseau. */
export function watchPlayerRank(account:PlayerRequest|null,profile:PlayerProfile|null,publish:(rank:string)=>void):()=>void {
 const rank=defaultPlayerRank(account,profile);publish(rank);
 if(!profile||rank==='EMERALD_PLUS')return ()=>{};
 const remaining=profile.fetched_at*1000+PROFILE_RANK_MAX_AGE_MS-Date.now();
 const timer=setTimeout(()=>publish(defaultPlayerRank(account,profile)),remaining+1);
 return ()=>clearTimeout(timer);
}
