import {it,expect} from 'vitest';
import {connectPatchPublications} from './patchPublication';
it('rafraîchit une seule fois par publication et nettoie une écoute tardive',async()=>{
 let receive!:(v:{revision:number})=>void,resolve!:(stop:()=>void)=>void,calls=0,stops=0;
 const dispose=connectPatchPublications(fn=>{receive=fn;return new Promise(r=>{resolve=r})},async()=>({revision:1}),()=>calls++);
 receive({revision:2});receive({revision:2});expect(calls).toBe(1);
 resolve(()=>stops++);await Promise.resolve();await Promise.resolve();expect(calls).toBe(1);
 receive({revision:3});expect(calls).toBe(2);dispose();receive({revision:4});expect(calls).toBe(2);expect(stops).toBe(1);
 let late!:(stop:()=>void)=>void;
 const close=connectPatchPublications(()=>new Promise(r=>{late=r}),async()=>({revision:0}),()=>calls++);close();late(()=>stops++);await Promise.resolve();expect(stops).toBe(2);
});
