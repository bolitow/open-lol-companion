import {afterEach, describe, expect, it, vi} from 'vitest';
import {createSkinImagePreloader, type PreloadImage} from './skinImagePreload';
class ImageStub implements PreloadImage {
 src=''; decoding=''; referrerPolicy=''; fetchPriority='';
 handlers=new Map<string,()=>void>(); removed=false;
 addEventListener(type:'load'|'error',listener:()=>void){this.handlers.set(type,listener)}
 removeEventListener(type:'load'|'error',listener:()=>void){if(this.handlers.get(type)===listener)this.handlers.delete(type)}
 removeAttribute(name:string){if(name==='src'){this.src='';this.removed=true}}
 emit(type:'load'|'error'){this.handlers.get(type)?.()}
}
function setup(){vi.useFakeTimers();const images:ImageStub[]=[];const create=()=>{const image=new ImageStub();images.push(image);return image};return{images,session:createSkinImagePreloader(create)}}
const url=(i:number)=>`https://assets.example/skin-${i}.jpg`;
afterEach(()=>vi.useRealTimers());
describe('préchargement ciblé des splashs',()=>{
 it('attend 150 ms sur la carte et annule une intention trop courte',()=>{
  const {session,images}=setup();session.warm(url(1));vi.advanceTimersByTime(149);expect(images).toHaveLength(0);session.cancelIntent();vi.advanceTimersByTime(1);expect(images).toHaveLength(0);
  session.warm(url(2));vi.advanceTimersByTime(150);expect(images.map(i=>i.src)).toEqual([url(2)]);expect(images[0]?.decoding).toBe('async');expect(images[0]?.fetchPriority).toBe('low');session.dispose();
 });
 it('ne démarre que deux requêtes, reprend sa file et ne charge rien globalement',()=>{
  const {session,images}=setup();for(let i=0;i<5;i++){session.warm(url(i));vi.advanceTimersByTime(150)}
  expect(images).toHaveLength(2);images[0]!.emit('load');expect(images).toHaveLength(3);expect(images[2]?.src).toBe(url(2));session.dispose();
 });
 it('borne le cache à huit et conserve le dernier visuel revu',()=>{
  const {session,images}=setup();for(let i=0;i<8;i++){session.warm(url(i));vi.advanceTimersByTime(150);images.at(-1)!.emit('load')}
  session.warm(url(0));vi.advanceTimersByTime(150);expect(images).toHaveLength(8);
  session.warm(url(8));vi.advanceTimersByTime(150);images.at(-1)!.emit('load');expect(images[1]?.removed).toBe(true);expect(images[0]?.removed).toBe(false);
  session.warm(url(1));vi.advanceTimersByTime(150);expect(images).toHaveLength(10);session.dispose();
 });
 it('borne aussi la file lorsque les deux requêtes restent occupées',()=>{
  const {session,images}=setup();for(let i=0;i<20;i++){session.warm(url(i));vi.advanceTimersByTime(150)}
  expect(images).toHaveLength(2);for(let i=0;i<8;i++)images[i]?.emit('load');expect(images).toHaveLength(8);expect(images.at(-1)?.src).toBe(url(19));session.dispose();
 });
 it('libère une erreur et autorise un nouvel essai sans boucle automatique',()=>{
  const {session,images}=setup();session.warm(url(1));vi.advanceTimersByTime(150);images[0]!.emit('error');vi.advanceTimersByTime(500);expect(images).toHaveLength(1);
  session.warm(url(1));vi.advanceTimersByTime(150);expect(images).toHaveLength(2);session.dispose();
 });
 it('interrompt les temporisations, détache les images et ignore leurs événements tardifs',()=>{
  const {session,images}=setup();for(let i=0;i<4;i++){session.warm(url(i));vi.advanceTimersByTime(150)}
  session.warm(url(4));session.dispose();session.dispose();session.warm(url(5));vi.runAllTimers();expect(images).toHaveLength(2);expect(images.every(i=>i.removed&&i.handlers.size===0)).toBe(true);
 });
 it('libère une requête bloquée après le délai et refuse les URL non HTTPS',()=>{
  const {session,images}=setup();session.warm('http://assets.example/no.jpg');session.warm('javascript:alert(1)');vi.advanceTimersByTime(150);expect(images).toHaveLength(0);
  for(let i=0;i<3;i++){session.warm(url(i));vi.advanceTimersByTime(150)}vi.advanceTimersByTime(12000);expect(images.length).toBeGreaterThan(2);expect(images[0]?.removed).toBe(true);session.dispose();
 });
});
