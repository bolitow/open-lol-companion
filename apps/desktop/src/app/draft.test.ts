import {it,expect} from 'vitest';
import type {DraftSession} from '@olc/shared';
import {draftTeams,secondsRemaining,championDetails} from './draft';
const d:DraftSession={supported:true,allySide:'red',allies:[],enemies:[],allyBans:[103],enemyBans:[99],timer:null,localSpells:null};
it('place le bleu à gauche même si on joue rouge, sans inventer un côté',()=>{
 expect(draftTeams(d).map(t=>[t.side,t.ally,t.bans])).toEqual([['blue',false,[99]],['red',true,[103]]]);
 expect(draftTeams({...d,allySide:null}).map(t=>t.side)).toEqual([null,null]);
});
it('décompte le timer réel sans dérive et borne à zéro',()=>{
 expect(secondsRemaining({remainingMs:36183,observedAtMs:1000},2183)).toBe(35);
 expect(secondsRemaining({remainingMs:1000,observedAtMs:1000},5000)).toBe(0);
 expect(secondsRemaining(null,100)).toBeNull();
});
it('résout les noms officiels et garde un repli pour les nouveaux champions',()=>{
 expect(championDetails(103,'fr')?.name).toBe('Ahri');
 expect(championDetails(999999,'fr')).toBeNull();
 expect(championDetails(null,'en')).toBeNull();
});
