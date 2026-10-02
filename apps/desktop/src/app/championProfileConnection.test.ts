import {expect,it,vi} from 'vitest';
import {connectProfile,type ProfileData} from './championProfileConnection';
const data:ProfileData={catalog:{version:'16.19.1',records:[]},abilities:[]};
it('ignore une réponse tardive après changement de champion ou de langue',async()=>{
 let finish!:(value:ProfileData)=>void;const receive=vi.fn();
 const stop=connectProfile(()=>new Promise(resolve=>{finish=resolve}),receive);
 await Promise.resolve();expect(receive).toHaveBeenLastCalledWith({status:'loading'});
 stop();finish(data);await Promise.resolve();await Promise.resolve();
 expect(receive).toHaveBeenCalledTimes(1);
});
it('expose une erreur relançable sans données périmées',async()=>{
 const receive=vi.fn();connectProfile(()=>Promise.reject(new Error('offline')),receive);
 await vi.waitFor(()=>expect(receive).toHaveBeenLastCalledWith({status:'error'}));
 const second=vi.fn();connectProfile(()=>Promise.resolve(data),second);
 await vi.waitFor(()=>expect(second).toHaveBeenLastCalledWith({status:'ready',data}));
});
