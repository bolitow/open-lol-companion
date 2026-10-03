import {afterEach,expect,it,vi} from 'vitest';
import {parseAbilityVideos} from './abilityVideos';
import shippedManifest from '../../public/game-data/ability-videos.json';

const media={page:'https://www.leagueoflegends.com/en-us/champions/ahri/',poster:'https://lol.dyn.riotcdn.net/x/videos/champion-abilities/0103/ability_0103_Q1.jpg',width:1056,height:720,sources:[{src:'https://lol.dyn.riotcdn.net/x/videos/champion-abilities/0103/ability_0103_Q1.mp4',type:'video/mp4'}]};
const manifest=()=>({schemaVersion:1,checkedAt:'2026-10-02T20:00:00Z',abilities:{'103:Q':media,'81:passive':{...media,poster:null,sources:[]}}});
afterEach(()=>vi.unstubAllGlobals());
it('accepte le catalogue livré, avec MP4, WebM et absence explicite',()=>{
 const parsed=parseAbilityVideos(shippedManifest);
 expect(parsed['103:Q']?.sources[0]?.type).toBe('video/mp4');
 expect(parsed['555:Q']?.sources[0]?.type).toBe('video/webm');
 expect(parsed['81:passive']?.sources).toEqual([]);
});
it('associe les vidéos aux identifiants de sorts et conserve une absence explicite',()=>{
 const parsed=parseAbilityVideos(manifest());
 expect(parsed['103:Q']?.sources[0]?.src).toBe(media.sources[0]?.src);
 expect(parsed['81:passive']?.sources).toEqual([]);
});
it.each(['https://example.com/video.mp4','http://lol.dyn.riotcdn.net/x/videos/champion-abilities/0103/ability_0103_Q1.mp4','https://lol.dyn.riotcdn.net.evil.test/video.mp4','https://lol.dyn.riotcdn.net/x/videos/champion-abilities/0081/ability_0081_Q1.mp4'])('refuse une URL non officielle ou un autre champion : %s',src=>{
 const value=manifest();value.abilities['103:Q']={...media,sources:[{src,type:'video/mp4'}]};
 expect(()=>parseAbilityVideos(value)).toThrow();
});
it('refuse une version inconnue et des dimensions non utilisables',()=>{
 expect(()=>parseAbilityVideos({...manifest(),schemaVersion:2})).toThrow();
 expect(()=>parseAbilityVideos({...manifest(),abilities:{'103:Q':{...media,width:0}}})).toThrow();
});
it('partage le chargement du catalogue, sans requête de média',async()=>{
 vi.resetModules();const {loadAbilityVideo:load}=await import('./abilityVideos');
 const requests:string[]=[];vi.stubGlobal('fetch',async(url:string)=>{requests.push(url);return {ok:true,json:async()=>manifest()}});
 const [q,missing]=await Promise.all([load('103:Q'),load('81:passive')]);
 expect(q?.sources[0]?.type).toBe('video/mp4');expect(missing?.sources).toEqual([]);
 expect(requests).toEqual(['/game-data/ability-videos.json']);
});
it('permet une nouvelle tentative après un échec du catalogue',async()=>{
 vi.resetModules();const {loadAbilityVideo:load}=await import('./abilityVideos');
 vi.stubGlobal('fetch',async()=>({ok:false}));await expect(load('103:Q')).rejects.toThrow();
 vi.stubGlobal('fetch',async()=>({ok:true,json:async()=>manifest()}));
 expect((await load('103:Q'))?.sources.length).toBe(1);
 expect(await load('999:Q')).toBeNull();
});
