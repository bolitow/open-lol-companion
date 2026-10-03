import {describe,it,expect} from 'vitest';
import {createDiagnosticsExport,type DiagnosticExportResult} from './diagnosticsExportController';
const request={locale:'fr' as const,includeLeagueSummary:false};
describe('export du diagnostic',()=>{
 it('attend la confirmation native avant de signaler un export',async()=>{
  let resolve!:(value:DiagnosticExportResult)=>void;
  const store=createDiagnosticsExport({native:true,export:()=>new Promise(r=>{resolve=r})});
  const task=store.run(request);expect(store.getSnapshot().status).toBe('pending');
  resolve('exported');await task;expect(store.getSnapshot()).toEqual({status:'exported',error:null});
 });
 it('ne transforme pas une annulation du dialogue OS en succès',async()=>{
  const store=createDiagnosticsExport({native:true,export:async()=>'cancelled'});await store.run(request);
  expect(store.getSnapshot()).toEqual({status:'cancelled',error:null});store.reset();expect(store.getSnapshot().status).toBe('idle');
 });
 it('refuse deux exports concurrents et transmet uniquement le choix explicite',async()=>{
  let calls=0,resolve!:(value:DiagnosticExportResult)=>void;
  const store=createDiagnosticsExport({native:true,export:async input=>{calls++;expect(input).toEqual(request);return new Promise(r=>{resolve=r})}});
  const first=store.run(request);await store.run({...request,includeLeagueSummary:true});store.reset();
  expect(calls).toBe(1);expect(store.getSnapshot().status).toBe('pending');resolve('cancelled');await first;
 });
 it('conserve un code fermé après erreur et permet une nouvelle tentative',async()=>{
  let fail=true;const store=createDiagnosticsExport({native:true,export:async()=>{if(fail)throw 'write_failed';return 'exported'}});
  await store.run(request);expect(store.getSnapshot()).toEqual({status:'error',error:'write_failed'});
  fail=false;await store.run(request);expect(store.getSnapshot()).toEqual({status:'exported',error:null});
 });
 it('ne transmet aucun détail d’erreur arbitraire à l’interface',async()=>{
  const store=createDiagnosticsExport({native:true,export:async()=>{throw new Error('/private/example')}});await store.run(request);
  expect(store.getSnapshot()).toEqual({status:'error',error:'unavailable'});
 });
 it('ne fait aucun appel natif depuis le navigateur',async()=>{
  let calls=0;const store=createDiagnosticsExport({native:false,export:async()=>{calls++;return 'exported'}});await store.run(request);
  expect(calls).toBe(0);expect(store.getSnapshot()).toEqual({status:'error',error:'unavailable'});
 });
});
it('transmet la langue et le résumé League uniquement après choix explicite',async()=>{
 let received:unknown;const store=createDiagnosticsExport({native:true,export:async input=>{received=input;return 'exported'}});
 await store.run({locale:'en',includeLeagueSummary:true});expect(received).toEqual({locale:'en',includeLeagueSummary:true});
});
it('conserve les codes fermés pour les messages précis et retire les abonnements',async()=>{
 for(const code of ['busy','unavailable','read_failed','write_failed','too_large','unsupported'] as const){
  const store=createDiagnosticsExport({native:true,export:async()=>{throw code}});const states:string[]=[];
  const stop=store.subscribe(()=>states.push(store.getSnapshot().status));await store.run(request);
  expect(store.getSnapshot().error).toBe(code);expect(states).toEqual(['pending','error']);
  stop();store.reset();expect(states).toEqual(['pending','error']);
 }
});
