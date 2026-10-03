import {it,expect} from 'vitest';
import {syncCustomRole} from './customRoleSync';
import type {Role} from '@olc/shared';
it('un accusé tardif du poste précédent ne réactive pas ses imports',async()=>{
 let finish!:()=>void;let ready:Role|null|undefined;
 const stop=syncCustomRole(()=>new Promise<void>(resolve=>{finish=resolve}),'UTILITY',role=>{ready=role});
 expect(ready).toBeUndefined();stop();
 syncCustomRole(()=>Promise.resolve(),'MIDDLE',role=>{ready=role});await Promise.resolve();
 expect(ready).toBe('MIDDLE');finish();await Promise.resolve();expect(ready).toBe('MIDDLE');
});
it('un échec de synchronisation garde les imports suspendus',async()=>{
 let ready:Role|null|undefined='UTILITY';
 syncCustomRole(()=>Promise.reject('unavailable'),'MIDDLE',role=>{ready=role});
 await Promise.resolve();await Promise.resolve();expect(ready).toBeUndefined();
});
