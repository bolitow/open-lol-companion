import {expect,it} from 'vitest';
import {initialState,reduceApp} from './state';
it('conserve le contexte Champions au retour sans modifier la préparation',()=>{
 let s=reduceApp(initialState,{type:'navigate',screen:'champions'});
 s=reduceApp(s,{type:'champions',patch:{selected:103,query:'ah',category:'Mage',scrollTop:420,tab:'builds',role:'MIDDLE'}});
 const context=s.champions;
 expect(context.selected).toBe(103);
 s=reduceApp(s,{type:'navigate',screen:'settings'});
 s=reduceApp(s,{type:'back'});
 expect(s.screen).toBe('champions');expect(s.champions).toEqual(context);
 expect(s.preparation).toEqual(initialState.preparation);
});
it('garde la consultation lors du passage automatique en draft',()=>{
 let s=reduceApp(initialState,{type:'champions',patch:{selected:103,scrollTop:99}});
 s=reduceApp(s,{type:'navigate',screen:'champions'});
 s=reduceApp(s,{type:'session',session:{...initialState.session,revision:1,connected:true,phase:'ChampSelect'}});
 expect(s.screen).toBe('champ-select');
 expect(reduceApp(s,{type:'back'}).champions).toMatchObject({selected:103,scrollTop:99});
});
