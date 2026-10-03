import {expect,it} from 'vitest';

const nativeFiles=import.meta.glob('../../src-tauri/{build.rs,capabilities/*.json}',{query:'?raw',import:'default',eager:true}) as Record<string,string>;
const nativeFile=(path:string)=>nativeFiles[`../../src-tauri/${path}`]!;
const systemCommands=['export_diagnostics','desktop_settings','set_desktop_setting','set_desktop_locale'];
it('déclare et autorise les commandes système dans la fenêtre principale après intégration du manifeste',()=>{
 const manifest=nativeFile('build.rs');
 const main=JSON.parse(nativeFile('capabilities/default.json'));
 for(const command of systemCommands){
  expect(manifest).toContain(`"${command}"`);
  expect(main.permissions).toContain(`allow-${command.replaceAll('_','-')}`);
 }
 expect(main.windows).toEqual(['main']);
});
it('limite le panneau passif à ses commandes de lecture et de taille',()=>{
 const overlay=JSON.parse(nativeFile('capabilities/overlay.json'));
 expect(overlay.windows).toEqual(['game-overlay']);
 expect(overlay.permissions).toEqual(['core:event:allow-listen','core:event:allow-unlisten','allow-live-session','allow-overlay-state','allow-community-builds','allow-overlay-content-height']);
});
