export type Phase='anticipate'|'appear'|'wave'|'idle'|'turn'|'evil'|'return';
const sequence:Partial<Record<Phase,[number,Phase]>>={appear:[.7,'wave'],wave:[1.2,'idle'],anticipate:[.24,'turn'],turn:[1.25,'evil'],evil:[2.4,'return'],return:[1.4,'idle']};
export class CompanionState {
 phase:Phase='appear';elapsed=0;motion=true;
 enter(phase:Phase){this.phase=phase;this.elapsed=0;}
 skip(){this.enter('idle');}
 interact(){if(this.phase==='idle')this.enter(this.motion?'anticipate':'evil');}
 setMotion(enabled:boolean){this.motion=enabled;this.skip();}
 advance(dt:number,visible=true){if(!visible)return;this.elapsed+=Math.max(0,dt);let next=sequence[this.phase];while(next&&this.elapsed>=next[0]){this.elapsed-=next[0];this.phase=next[1];next=sequence[this.phase];}}
}
const smooth=(x:number)=>{x=Math.max(0,Math.min(1,x));return x*x*(3-2*x);};
export function poseFor(s:CompanionState){
 const {phase,elapsed:t,motion}=s;
 const clip=phase==='evil'?'IdleEvil':phase==='turn'?'ToEvil':phase==='return'?(motion?'ToFriendly':'IdleFriendly'):phase==='wave'?'WaveFriendly':'IdleFriendly';
 const heat=phase==='evil'?1:phase==='turn'?smooth(t/1.25):phase==='return'?(motion?1-smooth(t/1.4):0):0;
 return {clip,heat,squash:phase==='anticipate'?1-.12*Math.sin(Math.PI*t/.24):1,clipDuration:phase==='wave'?1.2:phase==='turn'?1.25:phase==='return'?1.4:0,scale:phase==='appear'?.08+.92*smooth(t/.7):1,energy:!motion?0:phase==='turn'||phase==='return'?Math.sin(Math.PI*Math.min(t/(phase==='turn'?1.25:1.4),1)):phase==='wave'?.15:0};
}

// Mémoire limitée à la page : aucun stockage durable entre deux lancements.
export class WelcomeSession {
 private greeted=false;
 claim(){if(this.greeted)return false;this.greeted=true;return true;}
}
