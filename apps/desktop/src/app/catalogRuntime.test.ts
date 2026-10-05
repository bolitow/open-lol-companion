import {expect,it,vi} from 'vitest';
import {createCatalogRuntime} from './catalogRuntime';
const state=(snapshotId:string)=>({status:'ready' as const,version:'16.19.1',snapshotId,assetBase:`https://local/${snapshotId}/`,error:null});
const directory={version:'16.19.1',champions:[{id:999,key:'New',names:{fr:'Nouveau',en:'New'},titles:{fr:'Titre',en:'Title'},categories:['Mage'],image:'champions/999.png'}]};
const index={'999':{key:'New',fr:'Nouveau',en:'New'}};
it('active ensemble les index et une republication du même patch',async()=>{
 const read=vi.fn(async(_id:string,path:string)=>path==='cosmetics.json'?null:path==='champions.json'?index:directory);
 const runtime=createCatalogRuntime({read});
 await runtime.accept(state('a'));expect(runtime.getSnapshot().directory.champions[0]?.id).toBe(999);
 await runtime.accept(state('b'));expect(runtime.getSnapshot().generation).toBe(2);expect(runtime.getSnapshot().active?.snapshotId).toBe('b');
});
it('garde le catalogue précédent si un index est corrompu',async()=>{
 const runtime=createCatalogRuntime({read:async()=>({})});await runtime.accept(state('a'));
 expect(runtime.getSnapshot().active).toBeNull();expect(runtime.getSnapshot().status).toBe('error');
});
it('ignore une activation tardive après un nouveau snapshot',async()=>{
 let release!:()=>void;const blocked=new Promise<void>(r=>{release=r});
 const runtime=createCatalogRuntime({read:async(id,path)=>{if(id==='a')await blocked;return path==='cosmetics.json'?null:path==='champions.json'?index:directory}});
 const first=runtime.accept(state('a'));await runtime.accept(state('b'));release();await first;
 expect(runtime.getSnapshot().active?.snapshotId).toBe('b');expect(runtime.getSnapshot().generation).toBe(1);
});
it('conserve les index actifs pendant une mise à jour et après une erreur',async()=>{
 const runtime=createCatalogRuntime({read:async(_id,path)=>path==='cosmetics.json'?null:path==='champions.json'?index:directory});
 await runtime.accept(state('a'));
 await runtime.accept({...state('a'),status:'updating'});
 expect(runtime.getSnapshot().generation).toBe(1);expect(runtime.getSnapshot().directory.champions[0]?.id).toBe(999);
 await runtime.accept({...state('a'),status:'error',error:'network'});
 expect(runtime.getSnapshot().active?.snapshotId).toBe('a');
});
it('refuse les index dont les identités diffèrent et les chemins sortant du snapshot',async()=>{
 for(const invalid of [{...directory,champions:[{...directory.champions[0],image:'../escape.png'}]}, {...directory,champions:[{...directory.champions[0],id:998}]}]){
  const runtime=createCatalogRuntime({read:async(_id,path)=>path==='cosmetics.json'?null:path==='champions.json'?index:invalid});await runtime.accept(state('a'));
  expect(runtime.getSnapshot().status).toBe('error');expect(runtime.getSnapshot().generation).toBe(0);
 }
});
