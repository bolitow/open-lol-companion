import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import {createPreviewGate,attachAbilityPlayback,type PlaybackState} from './abilityPreviewLifecycle';
beforeEach(()=>vi.useFakeTimers());afterEach(()=>vi.useRealTimers());
it('un passage rapide ne charge aucun aperçu, un survol maintenu en ouvre un',()=>{
 const states:boolean[]=[];const gate=createPreviewGate(value=>states.push(value));
 gate.enter();vi.advanceTimersByTime(200);gate.leave();vi.advanceTimersByTime(500);
 expect(states).not.toContain(true);
 gate.enter();vi.advanceTimersByTime(300);expect(states.at(-1)).toBe(true);gate.dispose();
});
it('garde le lecteur en passant dans son panneau et ferme le précédent',()=>{
 let a=false,b=false;const first=createPreviewGate(v=>a=v),second=createPreviewGate(v=>b=v);
 first.enter();vi.advanceTimersByTime(300);first.leave();vi.advanceTimersByTime(60);first.retain();vi.advanceTimersByTime(200);expect(a).toBe(true);
 second.enter();vi.advanceTimersByTime(300);expect(a).toBe(false);expect(b).toBe(true);
 first.dispose();expect(b).toBe(true);second.dispose();expect(b).toBe(false);
});
it('annule une ouverture programmée à la fermeture ou au démontage',()=>{
 let open=false;const gate=createPreviewGate(v=>open=v);gate.enter();gate.close();vi.advanceTimersByTime(500);expect(open).toBe(false);
 gate.enter();gate.dispose();vi.advanceTimersByTime(500);expect(open).toBe(false);
});
class Video extends EventTarget {
 muted=false;loop=false;playsInline=false;preload='none';src='';paused=true;
 play=vi.fn(async()=>{this.paused=false;this.dispatchEvent(new Event('playing'))});
 pause(){this.paused=true;this.dispatchEvent(new Event('pause'))}
 load=vi.fn();removeAttribute(name:string){if(name==='src')this.src=''}
}
const setup=(autoplay=true)=>{const video=new Video(),states:PlaybackState[]=[];const playback=attachAbilityPlayback(video,'https://example.test/video.mp4',autoplay,state=>states.push(state));return {video,states,playback}};
it('met en pause le lecteur partagé sans jeter son tampon à la fermeture de l’aperçu',async()=>{
 const video=new Video();video.src='https://example.test/video.mp4';video.preload='auto';
 const playback=attachAbilityPlayback(video,video.src,true,()=>{},true);await Promise.resolve();playback.dispose();
 expect(video.paused).toBe(true);expect(video.src).toBe('https://example.test/video.mp4');expect(video.load).not.toHaveBeenCalled();
});
it('lit sans son en boucle, puis libère la ressource au démontage',async()=>{
 const {video,states,playback}=setup();await vi.runAllTimersAsync();
 expect(video.muted&&video.loop&&video.playsInline).toBe(true);expect(video.paused).toBe(false);expect(states.at(-1)).toBe('playing');
 playback.dispose();expect(video.paused).toBe(true);expect(video.src).toBe('');expect(video.load).toHaveBeenCalledOnce();
});
it('ne demande aucun octet vidéo en mouvements réduits avant Lecture',async()=>{
 const {video,states,playback}=setup(false);expect(video.src).toBe('');expect(video.play).not.toHaveBeenCalled();expect(states.at(-1)).toBe('paused');
 await playback.play();expect(video.src).toBe('https://example.test/video.mp4');expect(states.at(-1)).toBe('playing');playback.dispose();
});
it('propose la lecture manuelle si le moteur refuse autoplay',async()=>{
 const {video,states,playback}=setup(false);video.play.mockRejectedValueOnce(new DOMException('blocked','NotAllowedError'));
 await playback.play();expect(states.at(-1)).toBe('paused');
 await playback.play();expect(states.at(-1)).toBe('playing');playback.dispose();
});
it('signale un format illisible et arrête un chargement qui reste bloqué',async()=>{
 const {video,states,playback}=setup(false);video.play.mockRejectedValueOnce(new DOMException('unsupported','NotSupportedError'));
 await playback.play();expect(states.at(-1)).toBe('error');playback.dispose();
 const stalled=setup(false);stalled.video.play.mockImplementation(()=>new Promise(()=>{}));void stalled.playback.play();
 await vi.advanceTimersByTimeAsync(15000);expect(stalled.states.at(-1)).toBe('error');expect(stalled.video.src).toBe('');stalled.playback.dispose();
});
it('ignore un résultat de lecture tardif après fermeture',async()=>{
 const {video,states,playback}=setup(false);let reject:(reason:Error)=>void=()=>{};
 video.play.mockImplementation(()=>new Promise((_,no)=>{reject=no}));const pending=playback.play();playback.dispose();const count=states.length;
 reject(new DOMException('blocked','NotAllowedError'));await pending;expect(states.length).toBe(count);expect(video.paused).toBe(true);
});
