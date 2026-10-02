import type {Role} from '@olc/shared';
/** Un poste doit être accepté dans Rust avant de préparer son import ; les anciens accusés sont ignorés. */
export function syncCustomRole(send:(role:Role|null)=>Promise<void>,role:Role|null,publish:(role:Role|null|undefined)=>void):()=>void {
 let active=true;publish(undefined);
 void send(role).then(()=>{if(active)publish(role)},()=>{if(active)publish(undefined)});
 return ()=>{active=false};
}
