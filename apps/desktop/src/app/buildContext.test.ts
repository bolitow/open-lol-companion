import {expect,it} from 'vitest';
import type {LcuSession} from '@olc/shared';
import {initialState,initialPreparation,reduceApp} from './state';
import {preparationRequest,draftSelection} from './buildContext';
import {autoImportTarget} from './autoImport';
import type {PreparationCatalog} from './catalog';
const session:LcuSession={revision:1,connected:true,phase:'ChampSelect',draftId:'one',account:{game_name:'Player',tag_line:'NA',platform:'NA1'},runePage:null,draft:{supported:true,queueId:440,allySide:'blue',allies:[{cellId:0,championId:432,locked:false,local:true,position:'utility',acting:false},{cellId:1,championId:86,locked:true,local:false,position:'top',acting:false}],enemies:[{cellId:5,championId:103,locked:true,local:false,position:null,acting:false}],allyBans:[],enemyBans:[],localSpells:null,timer:null}};
it('suit région et file puis respecte les surcharges jusqu’à la nouvelle draft',()=>{
 let s=reduceApp(initialState,{type:'session',session});
 expect(s.preparation).toMatchObject({platform:'NA1',queue:440});
 expect(s.champions).toMatchObject({platform:'NA1',queue:440,role:'UTILITY'});
 s=reduceApp(s,{type:'preparation',patch:{platform:'KR',queue:420}});
 s=reduceApp(s,{type:'session',session:{...session,revision:2}});
 expect(s.preparation).toMatchObject({platform:'KR',queue:420});
 s=reduceApp(s,{type:'session',session:{...session,revision:3,draftId:'two'}});
 expect(s.preparation).toMatchObject({platform:'NA1',queue:440});
});
it('consulte le poste connu de l’allié et choisit uniquement un ennemi verrouillé comme matchup',()=>{
 const s=reduceApp(initialState,{type:'session',session});
 const ally=draftSelection(session.draft!,true,1);
 expect(preparationRequest(session,{...s.preparation,...ally},'16.19.1')).toMatchObject({champion_id:86,role:'TOP'});
 const enemy=draftSelection(session.draft!,false,5);
 expect(enemy).toEqual({matchup:{championId:103,kind:'chosen'}});
 expect(preparationRequest(session,{...s.preparation,...enemy},'16.19.1')?.champion_id).toBe(432);
 expect(draftSelection({...session.draft!,enemies:[{...session.draft!.enemies[0]!,locked:false}]},false,5)).toEqual({});
});
it('partage le contexte prévisualisé et importé, sans inventer le poste en personnalisée',()=>{
 const custom={...session,draft:{...session.draft!,customGame:true,queueId:3100,allies:[{...session.draft!.allies[0]!,position:null}]}};
 const s=reduceApp(initialState,{type:'session',session:custom});
 expect(preparationRequest(custom,s.preparation,'16.19.1')).toBeNull();
 const value={...s.preparation,customRole:'UTILITY' as const};
 const request=preparationRequest(custom,value,'16.19.1');
 expect(request).toMatchObject({platform:'NA1',queue:420,role:'UTILITY'});
 expect(autoImportTarget(custom,{version:'16.19.1'} as PreparationCatalog,'en','UTILITY',value)?.request).toEqual(request);
 expect(autoImportTarget(custom,{version:'16.19.1'} as PreparationCatalog,'en','UTILITY',{...value,manual:86})).toBeNull();
});
it('efface un matchup devenu invisible et conserve le contexte en coupure',()=>{
 let s=reduceApp(initialState,{type:'session',session});
 s=reduceApp(s,{type:'preparation',patch:{matchup:{championId:103,kind:'chosen'}}});
 s=reduceApp(s,{type:'session',session:{...session,revision:2,connected:false,draft:null,account:null}});
 expect(s.preparation.platform).toBe('NA1');
 s=reduceApp(s,{type:'session',session:{...session,revision:3,draft:{...session.draft!,enemies:[]}}});
 expect(s.preparation.matchup).toBeNull();
 expect(preparationRequest({...session,draft:null},initialPreparation,'16.19.1')).toBeNull();
});
it('distingue deux alliés avec le même champion et ne devine pas un poste inconnu',()=>{
 const repeated={...session,draft:{...session.draft!,allies:[session.draft!.allies[0]!,{...session.draft!.allies[1]!,championId:432}]}};
 const s=reduceApp(initialState,{type:'session',session:repeated});
 const value={...s.preparation,...draftSelection(repeated.draft,true,1)};
 expect(preparationRequest(repeated,value,'16.19.1')?.role).toBe('TOP');
 const unknown={...repeated,draft:{...repeated.draft,allies:[repeated.draft.allies[0]!,{...repeated.draft.allies[1]!,position:null}]}};
 expect(preparationRequest(unknown,value,'16.19.1')).toBeNull();
});
it('distingue reconnexion du même compte et changement de compte après une coupure',()=>{
 let s=reduceApp(initialState,{type:'session',session});
 s=reduceApp(s,{type:'preparation',patch:{platform:'KR',manual:86}});
 s=reduceApp(s,{type:'session',session:{...session,revision:2,connected:false,account:null,draft:null}});
 const same=reduceApp(s,{type:'session',session:{...session,revision:3}});
 expect(same.preparation).toMatchObject({platform:'KR',manual:86});
 const other=reduceApp(s,{type:'session',session:{...session,revision:3,account:{...session.account!,game_name:'Other'}}});
 expect(other.preparation).toMatchObject({platform:'NA1',manual:null});
});

it('oublie un poste implicite devenu inconnu mais conserve coupure et choix explicite',()=>{
 const before=reduceApp(initialState,{type:'session',session});
 const unknown={...session,revision:2,draftId:'two',draft:{...session.draft!,customGame:true,allies:[{...session.draft!.allies[0]!,position:null}]}};
 expect(reduceApp(before,{type:'session',session:unknown}).champions.role).toBe('UNKNOWN');
 expect(reduceApp(before,{type:'session',session:{...session,revision:2,connected:false,draft:null,account:null}}).champions.role).toBe('UTILITY');
 const explicit=reduceApp(before,{type:'champions',patch:{role:'TOP'}});
 expect(reduceApp(explicit,{type:'session',session:unknown}).champions.role).toBe('TOP');
});
