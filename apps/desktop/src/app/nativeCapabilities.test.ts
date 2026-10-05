import {expect,it} from 'vitest';

const nativeFiles=import.meta.glob('../../src-tauri/{build.rs,capabilities/*.json}',{query:'?raw',import:'default',eager:true}) as Record<string,string>;
const nativeFile=(path:string)=>nativeFiles[`../../src-tauri/${path}`]!;
const systemCommands=['export_diagnostics','desktop_settings','set_desktop_setting','set_desktop_locale','collection_state','collection_refresh','collection_set_wish','open_skin_spotlight'];
it('déclare et autorise les commandes système dans la fenêtre principale après intégration du manifeste',()=>{
 const manifest=nativeFile('build.rs');
 const main=JSON.parse(nativeFile('capabilities/default.json'));
 for(const command of systemCommands){
  expect(manifest).toContain(`"${command}"`);
  expect(main.permissions).toContain(`allow-${command.replaceAll('_','-')}`);
 }
 expect(main.windows).toBeUndefined();expect(main.webviews).toEqual(['main']);
});
it('limite le panneau passif à ses commandes de lecture et de taille',()=>{
 const overlay=JSON.parse(nativeFile('capabilities/overlay.json'));
 expect(overlay.windows).toEqual(['game-overlay']);
 expect(overlay.permissions).toEqual(['core:event:allow-listen','core:event:allow-unlisten','allow-live-session','allow-overlay-state','allow-community-builds','allow-overlay-content-height','allow-build-patch-context','allow-publication-state','allow-catalog-state','allow-catalog-read','allow-catalog-sync','allow-overlay-edit']);
});

it('isole le shell vidéo et ne donne aucun droit à la Webview YouTube',()=>{
 const video=JSON.parse(nativeFile('capabilities/spotlight.json'));
 expect(video.windows).toBeUndefined();expect(video.webviews).toEqual(['skin-spotlight-controls']);
 expect(video.permissions).toEqual(['core:event:allow-listen','core:event:allow-unlisten','allow-skin-spotlight-state','allow-skin-spotlight-media','allow-skin-spotlight-control','allow-skin-spotlight-layout']);
});
