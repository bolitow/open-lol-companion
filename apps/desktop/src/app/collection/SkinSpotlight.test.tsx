import {renderToStaticMarkup} from 'react-dom/server';
import {expect,it,vi} from 'vitest';
import {SkinSpotlightView,openSkinSpotlight,createSpotlightPlayback} from './SkinSpotlight';
vi.mock('@tauri-apps/api/core',()=>({invoke:vi.fn(async()=>({revision:3})),isTauri:()=>true}));
import {invoke} from '@tauri-apps/api/core';
const video={skinId:103007,championId:103,name:'Arcade Ahri',videoId:'IPU9_WRcsj4',title:'Arcade Ahri Skin Spotlight - League of Legends',channelUrl:'https://www.youtube.com/@SkinSpotlights',publishedAt:'2023-02-05',checkedAt:'2026-10-03',source:'https://www.youtube.com/watch?v=IPU9_WRcsj4'};
const render=(props:Partial<Parameters<typeof SkinSpotlightView>[0]>={})=>renderToStaticMarkup(<SkinSpotlightView locale="fr" status="ready" video={video} busy={false} openError={false} onOpen={()=>{}} onRetry={()=>{}} {...props}/>);
it('propose le lecteur à la demande et le repli externe sans télécharger la vidéo',()=>{
 const html=render();expect(html).toContain('Lire la vidéo');expect(html).toContain('YouTube');expect(html).toContain('SkinSpotlights');expect(html).toContain('2023');expect(html).toContain('https://i.ytimg.com/vi/IPU9_WRcsj4/hqdefault.jpg');expect(html).not.toContain('<iframe');expect(html).not.toContain('<video');
});
it('signale une association inconnue et permet de réessayer un catalogue inaccessible',()=>{
 expect(render({video:null})).toContain('Vidéo non référencée');expect(render({video:null})).not.toContain('Lire la vidéo');expect(render({status:'error',video:null})).toContain('Réessayer');expect(render({openError:true})).toContain('Impossible');
});
it('traduit les états et empêche les doubles ouvertures pendant la commande',()=>{const html=render({locale:'en',busy:true});expect(html).toContain('Opening');expect(html).toContain('disabled');});
it('passe seulement identifiant validé et mode au cœur natif',async()=>{
 await openSkinSpotlight(video,false);expect(invoke).toHaveBeenCalledWith('skin_spotlight_control',{revision:3,action:{type:'open',skinId:103007,championId:103,locale:'fr',kind:'full'}});
 await openSkinSpotlight(video,true);expect(invoke).toHaveBeenLastCalledWith('open_skin_spotlight',{videoId:'IPU9_WRcsj4',external:true});
});

it('garde la fiche latérale compacte et réserve les passages à la visionneuse',()=>{
 const withSegments={...video,segments:[{kind:'q' as const,start:40,end:55},{kind:'recall' as const,start:80,end:95}]};
 expect(render({video:withSegments})).toContain('A');
 expect(render({video:withSegments})).not.toContain('Rappel');
 expect(render({video:withSegments,locale:'en'})).toContain('In-app viewer');
 expect(render({video:withSegments})).not.toContain('Emotes');
});
it('transmet les bornes du passage au lecteur natif',async()=>{
 await openSkinSpotlight(video,false,{kind:'q',start:40,end:55});
 expect(invoke).toHaveBeenLastCalledWith('skin_spotlight_control',{revision:3,action:{type:'open',skinId:103007,championId:103,locale:'fr',kind:'q'}});
});

it('conserve le passage après échec pour le repli et le réinitialise à la vidéo suivante',async()=>{
 const segment={kind:'q' as const,start:88,end:97};
 const transport=vi.fn().mockRejectedValueOnce(new Error('player')).mockResolvedValue(undefined);
 const playback=createSpotlightPlayback(video,transport);
 await expect(playback(false,segment)).rejects.toThrow('player');
 await playback(true);
 expect(transport).toHaveBeenLastCalledWith(video,true,segment);
 await playback(false);
 await playback(true);
 expect(transport).toHaveBeenLastCalledWith(video,true,undefined);
 const other={...video,skinId:103015,videoId:'7lS2jAaJG8E'};
 await createSpotlightPlayback(other,transport)(true);
 expect(transport).toHaveBeenLastCalledWith(other,true,undefined);
});
