import {screenForPhase,type LcuSession} from '@olc/shared';
import type {Screen} from './state';
export type Scene={screen:Screen;phase:LcuSession['phase'];connected:boolean};
/** Les événements de données ne relancent jamais une transition de navigation. */
export function transitionKind(previous:Scene|null,next:Scene):'page'|'phase'|null{
 if(!previous||previous.screen===next.screen)return null;
 return next.connected&&next.phase&&screenForPhase(next.phase)===next.screen&&
  (!previous.connected||previous.phase!==next.phase)?'phase':'page';
}
