import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {HomePlayer,PlayerHistory,PlayerIdentity} from './PlayersScreen';
import {createPlayerStore,playerKey,type PlayerEntry} from './playerStore';
import type {PlayerProfile} from '@olc/shared';
const identity={platform:'EUW1',game_name:'Fixture',tag_line:'TEST',profile_icon_id:42};
const store=createPlayerStore({profile:async()=>{throw 'not_configured'},matches:async()=>{throw 'unavailable'}},identity,()=>{});
store.start();
const entry=():PlayerEntry=>({...store.getSnapshot().entries[playerKey(identity)]!,profile:null,loading:false,error:null,historyLoading:false,historyError:null,matches:[]});
const history=(value:PlayerEntry,locale:'fr'|'en')=>renderToStaticMarkup(<PlayerHistory entry={value} locale={locale} store={store} onChampion={()=>{}}/>);
it('montre le chargement et l’indisponibilité du profil dans un historique sans parties',()=>{
 for(const locale of ['fr','en'] as const){
  expect(history({...entry(),loading:true},locale)).toContain(locale==='fr'?'Chargement des parties':'Loading matches');
  const failed=history({...entry(),error:'not_configured'},locale);
  expect(failed).toContain(locale==='fr'?'Parties indisponibles':'Matches unavailable');
  expect(failed).not.toContain(locale==='fr'?'Aucune partie publique':'No public matches');
 }
});
it('conserve les parties déjà reçues quand une actualisation du profil échoue',()=>{
 const html=history({...entry(),error:'unavailable',matches:[{match_id:'EUW1_1',queue_id:420,patch:null,game_start_ms:1000,duration_s:1200,champion_id:103,win:true,kills:3,deaths:2,assists:7,items:[],role:null}]},'fr');
 expect(html).toContain('Ahri');expect(html).not.toContain('Parties indisponibles');
});
it('utilise l’avatar public mémorisé et ne transforme pas des PL inconnus en zéro',()=>{
 const profile:PlayerProfile={...identity,source:'lcu',summoner_level:50,fetched_at:0,ranks:[{queue_id:420,status:'ranked',tier:'GOLD',division:'II',league_points:null}]};
 const html=renderToStaticMarkup(<PlayerIdentity profile={profile} identity={identity} locale="fr"/>);
 expect(html).toContain('/profileicon/42.png');expect(html).toContain('— PL');expect(html).not.toContain('0 PL');
 const remembered=renderToStaticMarkup(<PlayerIdentity profile={null} identity={identity} locale="fr"/>);
 expect(remembered).toContain('/profileicon/42.png');
});

it('ne présente pas une recharge du profil comme un historique vide',()=>{
 const profile:PlayerProfile={...identity,source:'lcu',summoner_level:50,fetched_at:0,ranks:[]};
 const html=history({...entry(),profile,loading:true},'fr');
 expect(html).toContain('Chargement des parties');expect(html).not.toContain('Le client ne fournit aucune partie actuellement');
});

it('garde le repère graphique borné après une longue pagination sans tronquer les parties',()=>{
 const matches=Array.from({length:250},(_,i)=>({match_id:`EUW1_${i}`,queue_id:420,patch:null,game_start_ms:1000,duration_s:1200,champion_id:103,win:i%2===0,kills:3,deaths:2,assists:7,items:[],role:null}));
 const html=history({...entry(),matches},'fr');
 const graphic=html.match(/class="history-results"[^>]*>(.*?)<\/div>/)?.[1]??'';
 expect(graphic.match(/<i\b/g)?.length).toBeLessThanOrEqual(2);
 expect(html.match(/<article\b/g)).toHaveLength(250);
 expect(html).toContain('125 V');expect(html).toContain('125 D');
});

it('affiche les emblèmes officiels sans inventer de rang absent',()=>{
 for(const tier of ['IRON','BRONZE','SILVER','GOLD','PLATINUM','EMERALD','DIAMOND','MASTER','GRANDMASTER','CHALLENGER']){
  const profile:PlayerProfile={...identity,source:'lcu',summoner_level:50,fetched_at:0,ranks:[{queue_id:420,status:'ranked',tier,division:'II',league_points:12}]};
  const html=renderToStaticMarkup(<PlayerIdentity profile={profile} identity={identity} locale="fr"/>);
  expect(html).toContain(`/game-data/ranks/${tier.toLowerCase()}.png`);
  expect(html.match(/class="rank-emblem"/g)).toHaveLength(1);
 }
 const profile:PlayerProfile={...identity,source:'lcu',summoner_level:50,fetched_at:0,ranks:[{queue_id:420,status:'unranked',tier:null,division:null,league_points:null}]};
 expect(renderToStaticMarkup(<PlayerIdentity profile={profile} identity={identity} locale="fr"/>)).not.toContain('class="rank-emblem"');
});

it('retire le bandeau actif et le bouton de retrait pendant une connexion League',()=>{
 const state={...store.getSnapshot(),home:identity,active:identity,connected:true};
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<HomePlayer locale={locale} state={state} store={store} onBrowse={()=>{}}/>);
  expect(html).not.toContain('home-account-status');
  expect(html).not.toContain(locale==='fr'?'Retirer de l’accueil':'Remove from home');
  expect(html).toContain(locale==='fr'?'Voir le profil':'View profile');
  expect(html).toContain(locale==='fr'?'Actualiser':'Refresh');
 }
 const html=renderToStaticMarkup(<HomePlayer locale="fr" state={{...state,active:null,connected:false}} store={store} onBrowse={()=>{}}/>);
 expect(html).toContain('League déconnecté');expect(html).toContain('Retirer de l’accueil');
});

it('affiche les rangs apex sans division et conserve leurs PL en FR/EN',()=>{
 for(const locale of ['fr','en'] as const) for(const tier of ['MASTER','GRANDMASTER','CHALLENGER']){
  const profile:PlayerProfile={...identity,source:'api',summoner_level:50,fetched_at:0,ranks:[{queue_id:420,status:'ranked',tier,division:'I',league_points:321}]};
  const html=renderToStaticMarkup(<PlayerIdentity profile={profile} identity={identity} locale={locale}/>);
  expect(html).not.toMatch(/<strong>[^<]* I<\/strong>/);
  expect(html).toContain(`321 ${locale==='fr'?'PL':'LP'}`);
 }
 const profile:PlayerProfile={...identity,source:'lcu',summoner_level:50,fetched_at:0,ranks:[{queue_id:420,status:'ranked',tier:'DIAMOND',division:'II',league_points:20}]};
 expect(renderToStaticMarkup(<PlayerIdentity profile={profile} identity={identity} locale="fr"/>)).toContain('Diamant II');
});
