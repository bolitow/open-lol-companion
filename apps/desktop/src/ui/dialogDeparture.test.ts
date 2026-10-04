import {afterEach,expect,it,vi} from 'vitest';
import {createDialogDeparture} from './dialogDeparture';
afterEach(()=>vi.useRealTimers());
it('ne ferme qu’une fois et garde la fenêtre présente pendant 180 ms',()=>{
 vi.useFakeTimers();const finish=vi.fn(),depart=vi.fn(),motion=createDialogDeparture(finish,depart,()=>false);
 motion.close();motion.close();expect(depart).toHaveBeenCalledTimes(1);expect(finish).not.toHaveBeenCalled();
 vi.advanceTimersByTime(179);expect(finish).not.toHaveBeenCalled();vi.advanceTimersByTime(1);motion.close();expect(finish).toHaveBeenCalledTimes(1);
});
it('annule la sortie au démontage ou à la réouverture et permet une nouvelle fermeture',()=>{
 vi.useFakeTimers();const finish=vi.fn(),motion=createDialogDeparture(finish,()=>{},()=>false);
 motion.close();motion.cancel();vi.runAllTimers();expect(finish).not.toHaveBeenCalled();motion.close();vi.runAllTimers();expect(finish).toHaveBeenCalledTimes(1);
});
it('ferme immédiatement si les mouvements sont réduits',()=>{
 const finish=vi.fn(),motion=createDialogDeparture(finish,()=>{},()=>true);motion.close();motion.close();expect(finish).toHaveBeenCalledTimes(1);
});
