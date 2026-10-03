import {afterEach,expect,it,vi} from 'vitest';
import {readFlashPreference,saveFlashPreference,subscribeFlashPreference} from './flashPreference';
afterEach(()=>vi.unstubAllGlobals());
it('partage immédiatement D/F entre les deux panneaux et recharge le choix enregistré',()=>{
    const values=new Map<string,string>([['olc.flash-slot','"D"']]);
    vi.stubGlobal('window',new EventTarget());
    vi.stubGlobal('localStorage',{getItem:(key:string)=>values.get(key)??null,setItem:(key:string,value:string)=>values.set(key,value)});
    const observed:(string|null)[]=[];
    const unsubscribe=subscribeFlashPreference(()=>observed.push(readFlashPreference()));
    expect(readFlashPreference()).toBe('D');
    expect(saveFlashPreference('F')).toBe(true);
    expect(observed).toEqual(['F']);
    expect(readFlashPreference()).toBe('F');
    unsubscribe();saveFlashPreference('D');expect(observed).toEqual(['F']);
});
it('un stockage inaccessible ne choisit aucune position implicite et signale l’échec',()=>{
    vi.stubGlobal('window',new EventTarget());
    vi.stubGlobal('localStorage',{getItem:()=>{throw Error('denied')},setItem:()=>{throw Error('denied')}});
    expect(readFlashPreference()).toBeNull();
    expect(saveFlashPreference('F')).toBe(false);
});
