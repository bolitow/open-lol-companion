import {expect,it} from 'vitest';
import {initialState,reduceApp} from './state';
it('conserve la préparation si seule l’icône change, mais la réinitialise pour un autre compte',()=>{
 const account={platform:'EUW1',game_name:'Alpha',tag_line:'TAG',profile_icon_id:1};
 const connected=reduceApp(initialState,{type:'session',session:{...initialState.session,revision:1,connected:true,account}});
 const prepared=reduceApp(connected,{type:'preparation',patch:{manual:103,roleOverride:'MIDDLE'}});
 const updated=reduceApp(prepared,{type:'session',session:{...connected.session,revision:2,account:{...account,profile_icon_id:2}}});
 expect(updated.preparation.manual).toBe(103);
 expect(updated.preparation.roleOverride).toBe('MIDDLE');
 const switched=reduceApp(updated,{type:'session',session:{...updated.session,revision:3,account:{...account,game_name:'Beta'}}});
 expect(switched.preparation.manual).toBeNull();
});
