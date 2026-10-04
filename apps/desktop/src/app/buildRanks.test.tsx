import {expect,it,vi} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {PlayerProfile} from '@olc/shared';
import {buildRankOptions,defaultPlayerRank,watchPlayerRank,rankForQueue,rankLabel,PROFILE_RANK_MAX_AGE_MS} from './buildRanks';
import {RankSelect} from './RankSelect';
import {initialState,reduceApp} from './state';
const account={game_name:'Player',tag_line:'EUW',platform:'EUW1'};
const now=1_800_000_000_000;
const profile:PlayerProfile={...account,source:'lcu',fetched_at:now/1000,profile_icon_id:null,summoner_level:null,ranks:[{queue_id:440,status:'ranked',tier:'DIAMOND',division:'I',league_points:10},{queue_id:420,status:'ranked',tier:'GOLD',division:'I',league_points:20}]};
it('priorise Solo puis Flex et refuse un profil externe, ancien ou d’un autre compte',()=>{
 expect(defaultPlayerRank(account,profile,now)).toBe('GOLD');
 expect(defaultPlayerRank(account,{...profile,ranks:profile.ranks.slice(0,1)},now)).toBe('DIAMOND');
 for(const p of [null,{...profile,source:'api' as const},{...profile,game_name:'Other'},{...profile,fetched_at:(now-PROFILE_RANK_MAX_AGE_MS-1)/1000},{...profile,ranks:[]}])expect(defaultPlayerRank(account,p,now)).toBe('EMERALD_PLUS');
 expect(defaultPlayerRank(account,{...profile,ranks:[{...profile.ranks[0]!,tier:'CHALLENGER'}]},now)).toBe('MASTER_PLUS');
});
it('partage 8 cumulés et des libellés honnêtes ; la file normale désactive le filtre',()=>{
 expect(buildRankOptions.filter(r=>r.endsWith('_PLUS'))).toHaveLength(8);
 expect(buildRankOptions).toHaveLength(19);
 expect(buildRankOptions).not.toContain('GRANDMASTER_PLUS');
 expect(rankLabel('ALL','fr')).toBe('Échantillon collecté');
 expect(rankLabel('ALL','en')).toBe('Collected sample');
 expect(rankForQueue(400,'DIAMOND')).toBe('ALL');
 const html=renderToStaticMarkup(<RankSelect queue={400} rank="DIAMOND" locale="fr" onChange={()=>{}}/>);
 expect(html).toContain('disabled');expect(html).not.toContain('Diamant');
});
it('garde le choix explicite pendant la session et applique ALL hors classé',()=>{
 let s=reduceApp(initialState,{type:'default-rank',rank:'GOLD'});
 expect(s.preparation.rank).toBe('GOLD');expect(s.champions.rank).toBe('GOLD');
 s=reduceApp(s,{type:'preparation',patch:{rank:'DIAMOND_PLUS'}});
 s=reduceApp(s,{type:'default-rank',rank:'SILVER'});
 expect(s.preparation.rank).toBe('DIAMOND_PLUS');expect(s.champions.rank).toBe('SILVER');
 s=reduceApp(s,{type:'preparation',patch:{queue:400}});
 expect(s.preparation.rank).toBe('ALL');
 s=reduceApp(s,{type:'preparation',patch:{queue:420}});
 expect(s.preparation.rank).toBe('DIAMOND_PLUS');
});

it('expire une seule fois le profil local et annule l’échéance au changement de compte',()=>{
 vi.useFakeTimers();vi.setSystemTime(now);
 try{
  const publish=vi.fn();const stop=watchPlayerRank(account,profile,publish);
  expect(publish).toHaveBeenLastCalledWith('GOLD');
  vi.advanceTimersByTime(PROFILE_RANK_MAX_AGE_MS+1);
  expect(publish).toHaveBeenLastCalledWith('EMERALD_PLUS');
  expect(publish).toHaveBeenCalledTimes(2);stop();
  vi.setSystemTime(now);const next=vi.fn();const cancel=watchPlayerRank(account,profile,next);cancel();
  vi.advanceTimersByTime(PROFILE_RANK_MAX_AGE_MS+1);expect(next).toHaveBeenCalledTimes(1);
 }finally{vi.useRealTimers();}
});
