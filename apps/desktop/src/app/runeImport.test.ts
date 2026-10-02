import {expect,it} from 'vitest';
import type {ImportDraftRunesRequest} from '@olc/shared';
import {createRuneImportController,importEligibility} from './runeImport';
const request:ImportDraftRunesRequest={championId:103,runes:{championName:'Ahri',primaryStyleId:8100,subStyleId:8200,selectedPerkIds:[8112,8126,8140,8106,8210,8236,5008,5008,5011]}};
const deferred=()=>{let resolve!:()=>void;let reject!:(e:unknown)=>void;const promise=new Promise<void>((a,b)=>{resolve=a;reject=b});return {promise,resolve,reject}};
it('ne dispatch qu’une écriture lors de deux clics et attend le résultat',async()=>{
 const pending=deferred(),sent:ImportDraftRunesRequest[]=[];
 const controller=createRuneImportController(async r=>{sent.push(r);await pending.promise});
 controller.activate('ahri');const first=controller.submit('ahri',request);await controller.submit('ahri',request);
 expect(sent).toEqual([request]);expect(controller.getSnapshot().pending).toBe(true);
 pending.resolve();await first;expect(controller.getSnapshot()).toMatchObject({pending:false,result:{context:'ahri',status:'accepted'}});
});
it('un retour au même champion après changement ne récupère pas un ancien succès',async()=>{
 const pending=deferred();const controller=createRuneImportController(()=>pending.promise);
 controller.activate('ahri');const job=controller.submit('ahri',request);
 controller.activate('jinx');controller.activate('ahri');pending.resolve();await job;
 expect(controller.getSnapshot()).toEqual({pending:false,result:null});
});
it('refuse un événement périmé et masque une erreur IPC inconnue sans retry',async()=>{
 let calls=0;const controller=createRuneImportController(async()=>{calls++;throw 'message brut à ne pas afficher'});
 controller.activate('jinx');await controller.submit('ahri',request);expect(calls).toBe(0);
 controller.activate('ahri');await controller.submit('ahri',request);
 expect(calls).toBe(1);expect(controller.getSnapshot().result).toMatchObject({status:'error',error:'unknown'});
});
it('annule la présentation du résultat après fermeture de l’éditeur',async()=>{
 const pending=deferred();const controller=createRuneImportController(()=>pending.promise);
 controller.activate('ahri');const job=controller.submit('ahri',request);controller.activate(null);pending.reject('clientUnavailable');await job;
 expect(controller.getSnapshot()).toEqual({pending:false,result:null});
});
it('ne rend importable que le champion local d’une draft prise en charge',()=>{
 const local={cellId:2,championId:103,locked:false,local:true,position:'middle' as const,acting:false};
 const draft={supported:true,allySide:'blue' as const,allies:[local],enemies:[],allyBans:[],enemyBans:[],timer:null,localSpells:null};
 expect(importEligibility(false,draft,103,true)).toBe('desktop');
 expect(importEligibility(true,null,103,true)).toBe('draft');
 expect(importEligibility(true,{...draft,supported:false},103,true)).toBe('draft');
 expect(importEligibility(true,draft,222,true)).toBe('champion');
 expect(importEligibility(true,draft,103,false)).toBe('incomplete');
 expect(importEligibility(true,draft,103,true)).toBe(null);
});
