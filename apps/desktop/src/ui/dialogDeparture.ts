import {scheduleDeparture} from './presence';
/** Une sortie par ouverture, annulable sans fermer le dialogue natif. */
export function createDialogDeparture(finish:()=>void,depart:()=>void,reduced:()=>boolean){
 let requested=false,cancelTimer:(()=>void)|undefined;
 return {
  close(){if(requested)return;requested=true;depart();if(reduced())finish();else cancelTimer=scheduleDeparture(()=>{cancelTimer=undefined;finish()},180,false)},
  cancel(){cancelTimer?.();cancelTimer=undefined;requested=false},
 };
}
