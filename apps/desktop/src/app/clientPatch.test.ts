import {it,expect} from 'vitest';
import {publicClientPatch,createClientPatchStore} from './clientPatch';
it('sépare le numéro public du numéro technique et refuse les versions ambiguës',()=>{
 expect(publicClientPatch('16.19')).toBe('26.19');
 expect(publicClientPatch('16.19.1')).toBe('26.19');
 expect(publicClientPatch('16.19.123.4567')).toBe('26.19');
 expect(publicClientPatch('15.1')).toBe('25.1');
 expect(publicClientPatch('14.24')).toBe('14.24');
 for(const value of ['','bad','16.19.foo','16.0','016.19','16.019','1000.19','16.19.1.2.3'])expect(publicClientPatch(value)).toBeNull();
});
it('ne lit que sur demande, sans appel navigateur et sans chargements concurrents',async()=>{
 let calls=0,resolve!:(v:{gameVersion:string;patch:string})=>void;
 const store=createClientPatchStore(true,()=>{calls++;return new Promise(r=>{resolve=r})});
 const first=store.load();await store.load();expect(calls).toBe(1);
 resolve({gameVersion:'16.19.123.4567',patch:'16.19'});await first;
 expect(store.getSnapshot()).toMatchObject({patch:'26.19',error:null,pending:false});
 const browser=createClientPatchStore(false,async()=>{calls++;throw 'unavailable'});
 await browser.load();expect(calls).toBe(1);
});
it('remplace les données anciennes par une erreur locale traduisible',async()=>{
 let error:unknown=null;
 const store=createClientPatchStore(true,async()=>{if(error)throw error;return {gameVersion:'16.19.123.4567',patch:'16.19'}});
 await store.load();error='invalid_response';await store.load();
 expect(store.getSnapshot()).toMatchObject({patch:null,error:'invalid_response'});
 error=new Error('private');await store.load();expect(store.getSnapshot().error).toBe('unavailable');
});

it('affiche la version longue observée sur le client macOS',async()=>{
 const store=createClientPatchStore(true,async()=>({patch:'16.19',gameVersion:'16.19.8230722+branch.releases-16-19.code.public.content.release.anticheat.vanguard'}));
 await store.load();
 expect(store.getSnapshot()).toMatchObject({patch:'26.19',error:null});
});

it('refuse les suffixes corrompus et les patchs contradictoires',async()=>{
 for(const raw of ['16.19.bad suffix','16.19..build','16.20.123+build',`16.19.${'x'.repeat(256)}`]){
  const store=createClientPatchStore(true,async()=>({patch:'16.19',gameVersion:raw}));
  await store.load();expect(store.getSnapshot().error).toBe('invalid_response');
 }
});
