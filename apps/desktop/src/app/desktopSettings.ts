import type {DesktopSettings,DesktopSettingKey,DesktopSettingsError} from '@olc/shared';
export interface DesktopSettingsAdapter {native:boolean;read:()=>Promise<DesktopSettings>;change:(key:DesktopSettingKey,value:boolean)=>Promise<DesktopSettings>}
interface Snapshot {native:boolean;value:DesktopSettings|null;pending:boolean;error:DesktopSettingsError|null;lastChange:{key:DesktopSettingKey;previous:boolean;applied:boolean}|null}
const errors:DesktopSettingsError[]=['unavailable','read_failed','write_failed','autostart_failed','verification_failed'];
export function createDesktopSettingsStore(adapter:DesktopSettingsAdapter,onChange=()=>{}){
 let state:Snapshot={native:adapter.native,value:null,pending:false,error:null,lastChange:null};const listeners=new Set<()=>void>();
 const update=(patch:Partial<Snapshot>)=>{state={...state,...patch};listeners.forEach(fn=>fn())};
 const errorCode=(e:unknown):DesktopSettingsError=>errors.includes(e as DesktopSettingsError)?e as DesktopSettingsError:'unavailable';
 async function load(){
  if(!adapter.native||state.pending)return;update({pending:true,error:null});
  try{const value=await adapter.read();const last=state.lastChange;update({value,lastChange:last&&value[last.key]===last.applied?last:null})}
  catch(error){update({error:errorCode(error),value:null,lastChange:null})}
  finally{update({pending:false})}
 }
 async function change(key:DesktopSettingKey,next:boolean,undo=false){
  if(!adapter.native||state.pending||!state.value)return;
  const previous=state.value[key];if(previous===null||previous===next)return;
  update({pending:true,error:null});
  try{
   const value=await adapter.change(key,next);update({value});
   if(value[key]!==next)throw 'verification_failed';
   update({lastChange:undo?null:{key,previous,applied:next}});onChange();
  }catch(error){
   let value:DesktopSettings|null=null;try{value=await adapter.read()}catch{/* Pas d'état inventé si la relecture échoue. */}
   update({value,error:errorCode(error),lastChange:null});
  }finally{update({pending:false})}
 }
 return {getSnapshot:()=>state,subscribe:(fn:()=>void)=>{listeners.add(fn);return()=>{listeners.delete(fn)}},load,change,
  undo:async()=>{const last=state.lastChange;if(last)await change(last.key,last.previous,true)},
  clearUndo:()=>{if(state.lastChange)update({lastChange:null})}};
}
export type DesktopSettingsStore=ReturnType<typeof createDesktopSettingsStore>;
