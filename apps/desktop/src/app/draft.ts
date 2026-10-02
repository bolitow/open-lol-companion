import type {DraftSession,DraftTimer} from '@olc/shared';
import champions from '../../public/game-data/champions.json';
import type {Locale} from './state';
export function championDetails(id:number|null,locale:Locale){
 if(id===null)return null;
 const entry=champions[String(id) as keyof typeof champions];
 return entry?{name:entry[locale],image:`/game-data/champions/${id}.jpg`}:null;
}
export function draftTeams(draft:DraftSession|null){
 const allies={side:draft?.allySide??null,ally:true,players:draft?.allies??[],bans:draft?.allyBans??[]};
 const enemies={side:draft?.allySide==='blue'?'red' as const:draft?.allySide==='red'?'blue' as const:null,ally:false,players:draft?.enemies??[],bans:draft?.enemyBans??[]};
 return draft?.allySide==='red'?[enemies,allies]:[allies,enemies];
}
export function secondsRemaining(timer:DraftTimer|null,now:number){
 return timer?Math.floor(Math.max(0,timer.remainingMs-Math.max(0,now-timer.observedAtMs))/1000):null;
}
