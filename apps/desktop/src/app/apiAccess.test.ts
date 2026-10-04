import {describe,it,expect} from 'vitest';
import type {ApiAccessInput,ApiAccessStatus} from '@olc/shared';
import {canSaveApiAccess,createApiAccessStore} from './apiAccess';

const empty:ApiAccessStatus={source:null,url:null,error:null};
const saved:ApiAccessStatus={source:'keychain',url:'https://api.example.com',error:null};
const fake=(initial:ApiAccessStatus=empty)=>{
 let status=initial,failure:unknown=null;const inputs:ApiAccessInput[]=[];let calls=0;
 return {native:true,inputs,calls:()=>calls,fail:(error:unknown)=>{failure=error},
  read:async()=>{calls++;return status},
  save:async(input:ApiAccessInput)=>{calls++;if(failure)throw failure;inputs.push(input);status=saved;return status},
  clear:async()=>{calls++;if(failure)throw failure;status=empty;return status}};
};

describe('accès à l’API desktop',()=>{
 it('ne fait aucun appel système dans le navigateur',async()=>{
  const adapter={...fake(),native:false},store=createApiAccessStore(adapter);
  await store.load();expect(await store.save('https://api.example.com','jeton-de-test')).toBe(false);await store.clear();
  expect(adapter.calls()).toBe(0);expect(store.getSnapshot().status).toBeNull();
 });
 it('enregistre une seule fois et ne conserve jamais le jeton dans l’état',async()=>{
  const adapter=fake(),store=createApiAccessStore(adapter);await store.load();
  expect(await store.save('https://api.example.com','jeton-de-test')).toBe(true);
  expect(adapter.inputs).toEqual([{url:'https://api.example.com',token:'jeton-de-test'}]);
  expect(store.getSnapshot()).toMatchObject({status:saved,result:'saved',error:null,pending:false});
  expect(JSON.stringify(store.getSnapshot())).not.toContain('jeton-de-test');
 });
 it('garde le code d’erreur du cœur Rust et remplace un code inconnu',async()=>{
  const adapter=fake(),store=createApiAccessStore(adapter);await store.load();
  adapter.fail('invalid_configuration');expect(await store.save('http://example.com','jeton-de-test')).toBe(false);
  expect(store.getSnapshot()).toMatchObject({status:empty,error:'invalid_configuration',result:null});
  adapter.fail(new Error('détail système'));await store.save('https://api.example.com','jeton-de-test');
  expect(store.getSnapshot().error).toBe('write_failed');
 });
 it('refuse une saisie vide et laisse les variables d’environnement prioritaires',async()=>{
  expect(canSaveApiAccess(' ','jeton')).toBe(false);expect(canSaveApiAccess('https://api.example.com','  ')).toBe(false);
  expect(canSaveApiAccess('https://api.example.com','jeton')).toBe(true);
  const adapter=fake({source:'environment',url:'http://127.0.0.1:3030',error:null}),store=createApiAccessStore(adapter);await store.load();
  expect(await store.save('https://api.example.com','jeton-de-test')).toBe(false);await store.clear();
  expect(adapter.inputs).toEqual([]);expect(adapter.calls()).toBe(1);
 });
 it('retire le jeton du trousseau sur demande',async()=>{
  const adapter=fake(saved),store=createApiAccessStore(adapter);await store.load();await store.clear();
  expect(store.getSnapshot()).toMatchObject({status:empty,result:'cleared',error:null});
 });
 it('signale une lecture impossible sans inventer d’état',async()=>{
  const store=createApiAccessStore({...fake(),read:async()=>{throw 'unexpected'}});await store.load();
  expect(store.getSnapshot()).toMatchObject({status:null,error:'read_failed'});
 });
});
