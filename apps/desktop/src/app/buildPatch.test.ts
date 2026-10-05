import type {BuildRequest} from '@olc/shared';
import {it,expect} from 'vitest';
import {resolveBuildPatch,createPatchStore,readPatchBuilds} from './buildPatch';
const context={client:{gameVersion:'16.20.123+build',patch:'16.20'},manifest:{live_version:'16.20.1',versions:['16.20.1','16.19.1','16.18.1']},clientError:null,manifestError:null};
it('préfère le client, trie numériquement, ne choisit jamais un patch futur',()=>{
 expect(resolveBuildPatch(context,'16.19')).toMatchObject({patch:'16.20',kind:'current',canImport:false,previous:'16.19'});
 expect(resolveBuildPatch({...context,manifest:{...context.manifest,versions:['16.21.1','16.19.1','16.9.1']}},'16.19')).toMatchObject({patch:'16.19',kind:'fallback',canImport:false});
 expect(resolveBuildPatch({...context,manifest:{...context.manifest,versions:['16.21.1']}},'16.19').patch).toBeNull();
});
it('signale le client fermé et le manifeste indisponible sans prétendre suivre le realm EUW',()=>{
 expect(resolveBuildPatch({...context,client:null,clientError:'unavailable'},'16.19')).toMatchObject({patch:'16.19',kind:'client_unavailable',canImport:false});
 expect(resolveBuildPatch({...context,manifest:null,manifestError:'not_configured'},'16.19')).toMatchObject({patch:'16.20',kind:'manifest_unavailable',canImport:false});
 expect(resolveBuildPatch({...context,client:{gameVersion:'16.19.1',patch:'16.19'}},'16.19').canImport).toBe(true);
});
it('mutualise les lectures et ignore une réponse invalidée',async()=>{
 let calls=0,done!:(v:typeof context)=>void;
 const store=createPatchStore(()=>{calls++;return new Promise(resolve=>{done=resolve})});
 const first=store.load();void store.load();expect(calls).toBe(1);
 store.invalidate();done(context);await first;expect(store.getSnapshot().value).toBeNull();
});
it('ne replie que le groupe vide, pas une panne ou une réponse étrangère',async()=>{
 const request:BuildRequest={champion_id:103,patch:'16.20',platform:'EUW1',queue:420,role:'MIDDLE' as const,rank:'GOLD'};
 const report=(r:BuildRequest,builds:unknown[]=[])=>({request:r,builds,meta:{}} as any);
 let seen:string[]=[];
 const result=await readPatchBuilds(request,'16.19',async r=>{seen.push(r.patch);return report(r,r.patch==='16.19'?[{}]:[])});
 expect(seen).toEqual(['16.20','16.19']);expect(result.request.patch).toBe('16.19');
 seen=[];await expect(readPatchBuilds(request,'16.19',async()=>{seen.push('error');throw 'unauthorized'})).rejects.toBe('unauthorized');expect(seen).toHaveLength(1);
 await expect(readPatchBuilds(request,'16.19',async r=>report({...r,champion_id:1}))).rejects.toBe('invalid_response');
});
it('conserve les observations courantes et ne multiplie pas les replis',async()=>{
 const request:BuildRequest={champion_id:103,patch:'16.20',platform:'EUW1',queue:420,role:'MIDDLE',rank:'GOLD'};
 let calls=0;
 const current={request,builds:[],summary:{games:10},meta:{}} as any;
 expect(await readPatchBuilds(request,'16.19',async()=>{calls++;return current})).toBe(current);expect(calls).toBe(1);
 calls=0;const result=await readPatchBuilds(request,'16.19',async r=>{calls++;return {request:r,builds:[],meta:{}} as any});expect(calls).toBe(2);expect(result.request.patch).toBe('16.20');
});
it('la nouvelle lecture reste prioritaire après invalidation',async()=>{
 const completions:((v:typeof context)=>void)[]=[];
 const store=createPatchStore(()=>new Promise(resolve=>completions.push(resolve)));
 const first=store.load();store.invalidate();const second=store.load();
 completions[1]!({...context,client:{patch:'16.21',gameVersion:'16.21.1'}});await second;
 completions[0]!(context);await first;expect(store.getSnapshot().value?.client?.patch).toBe('16.21');
});
it('sépare invalidation de publication et vérification de patch lors des picks',async()=>{
 const store=createPatchStore(async()=>context);
 const initial=store.getSnapshot().publicationRevision;
 await store.load();await store.load();
 expect(store.getSnapshot().publicationRevision).toBe(initial);
 store.invalidate();expect(store.getSnapshot().publicationRevision).toBe(initial+1);
});
