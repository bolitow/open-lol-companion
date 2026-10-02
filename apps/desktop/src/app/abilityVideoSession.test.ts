import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import {createAbilityVideoSession} from './abilityVideoSession';
import type {AbilityVideo} from './abilityVideos';
class Video extends EventTarget {
 muted=false;loop=false;playsInline=false;preload='none';src='';paused=true;readyState=0;
 canPlayType=()=> 'probably' as const;
 play=vi.fn(async()=>{this.paused=false});pause=vi.fn(()=>{this.paused=true});load=vi.fn();
 removeAttribute(name:string){if(name==='src')this.src=''}
}
const media:AbilityVideo={page:'',poster:null,width:1056,height:720,sources:[{src:'https://example.test/Q.mp4',type:'video/mp4'}]};
beforeEach(()=>vi.useFakeTimers());afterEach(()=>vi.useRealTimers());
const flush=async()=>{await Promise.resolve();await Promise.resolve()};
it('précharge seulement les cinq sorts du champion sans aucune lecture',async()=>{
 const videos:Video[]=[],loader=vi.fn(async(_id:string)=>media);
 const session=createAbilityVideoSession(103,()=>{const v=new Video();videos.push(v);return v},loader);
 await flush();expect(loader.mock.calls.map(call=>call[0])).toEqual(['103:Q','103:W','103:E','103:R','103:passive']);
 expect(videos).toHaveLength(5);expect(videos.every(v=>v.preload==='auto'&&v.src===media.sources[0]!.src&&v.muted&&v.paused)).toBe(true);
 expect(videos.every(v=>v.play.mock.calls.length===0)).toBe(true);session.dispose();
});
it('réutilise le lecteur préchargé, refuse un autre champion, libère toutes les sources à la sortie',async()=>{
 const session=createAbilityVideoSession(103,()=>new Video(),async()=>media);await flush();
 const first=session.get('103:Q',media)!;expect(first).toBe(session.get('103:Q',media));expect(first.load).toHaveBeenCalledOnce();
 expect(session.get('99:Q',media)).toBeNull();session.dispose();expect(first.src).toBe('');expect(first.pause).toHaveBeenCalled();
 expect(session.get('103:Q',media)).toBeNull();session.dispose();expect(vi.getTimerCount()).toBe(0);
});
it('ignore les réponses tardives après changement de champion',async()=>{
 let resolve!:(value:AbilityVideo)=>void;const create=vi.fn(()=>new Video());
 const session=createAbilityVideoSession(103,create,()=>new Promise(done=>{resolve=done}));session.dispose();resolve(media);await flush();
 expect(create).not.toHaveBeenCalled();expect(vi.getTimerCount()).toBe(0);
});
it('arrête un préchargement bloqué et permet une nouvelle tentative à la consultation',async()=>{
 const videos:Video[]=[];const session=createAbilityVideoSession(103,()=>{const v=new Video();videos.push(v);return v},async()=>media);await flush();
 await vi.advanceTimersByTimeAsync(12000);expect(videos.every(v=>!v.src)).toBe(true);
 const video=session.get('103:Q',media)!;expect(video.src).toBe(media.sources[0]!.src);session.dispose();
});
it('ignore les médias absents, formats non supportés et erreurs du catalogue',async()=>{
 const create=vi.fn(()=>new Video());const session=createAbilityVideoSession(103,create,async id=>{if(id.endsWith(':Q'))throw Error('offline');return null});await flush();
 expect(create).not.toHaveBeenCalled();expect(session.get('103:Q',{...media,sources:[]})).toBeNull();session.dispose();
 const unsupported=createAbilityVideoSession(99,()=>Object.assign(new Video(),{canPlayType:()=>''}),async()=>media);await flush();expect(unsupported.get('99:Q',media)).toBeNull();unsupported.dispose();
});
