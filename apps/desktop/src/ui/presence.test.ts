import {afterEach,expect,it,vi} from 'vitest';
import {scheduleDeparture} from './presence';
afterEach(()=>vi.useRealTimers());
it('conserve la surface pendant la sortie et annule un démontage lors de sa réouverture',()=>{
 vi.useFakeTimers();const finish=vi.fn();const cancel=scheduleDeparture(finish,180,false);
 vi.advanceTimersByTime(90);expect(finish).not.toHaveBeenCalled();cancel();vi.advanceTimersByTime(200);expect(finish).not.toHaveBeenCalled();
 scheduleDeparture(finish,180,false);vi.advanceTimersByTime(180);expect(finish).toHaveBeenCalledOnce();
});
it('ne retarde pas le démontage en mouvement réduit',()=>{vi.useFakeTimers();const finish=vi.fn();scheduleDeparture(finish,180,true);vi.advanceTimersByTime(0);expect(finish).toHaveBeenCalledOnce()});
