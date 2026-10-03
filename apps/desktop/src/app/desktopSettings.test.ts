import {describe,it,expect} from 'vitest';
import {createDesktopSettingsStore} from './desktopSettings';
import type {DesktopSettings,DesktopSettingKey} from '@olc/shared';
const initial:DesktopSettings={closeToTray:true,autostartEnabled:false,trayAvailable:true,storageError:false};
const fake=()=>{let value={...initial};let fail=false;return {native:true,read:async()=>({...value}),change:async(key:DesktopSettingKey,next:boolean)=>{if(fail)throw 'write_failed';value={...value,[key]:next};return {...value}},setExternal:(v:DesktopSettings)=>{value=v},setFail:()=>{fail=true}}};
describe('réglages système',()=>{
 it('affiche uniquement la vérité native et conserve une erreur distincte de désactivé',async()=>{
  const adapter=fake(),store=createDesktopSettingsStore(adapter);await store.load();expect(store.getSnapshot().value).toEqual(initial);
  adapter.setExternal({...initial,autostartEnabled:null});await store.load();expect(store.getSnapshot().value?.autostartEnabled).toBeNull();
 });
 it('modifie et annule le réglage natif après confirmation du système',async()=>{
  const adapter=fake();let changes=0;const store=createDesktopSettingsStore(adapter,()=>{changes++});await store.load();await store.change('autostartEnabled',true);
  expect(store.getSnapshot().value?.autostartEnabled).toBe(true);expect(changes).toBe(1);await store.undo();
  expect((await adapter.read()).autostartEnabled).toBe(false);expect(store.getSnapshot().lastChange).toBeNull();
 });
 it('ne confirme pas une modification refusée et conserve la valeur relue',async()=>{
  const adapter=fake(),store=createDesktopSettingsStore(adapter);await store.load();adapter.setFail();await store.change('closeToTray',false);
  expect(store.getSnapshot().value?.closeToTray).toBe(true);expect(store.getSnapshot().error).toBe('write_failed');expect(store.getSnapshot().lastChange).toBeNull();
 });
 it('empêche deux mutations simultanées et ne réapplique pas un état optimiste',async()=>{
  let resolve!:(v:DesktopSettings)=>void;let calls=0;
  const store=createDesktopSettingsStore({native:true,read:async()=>initial,change:async()=>{calls++;return new Promise<DesktopSettings>(r=>{resolve=r})}});await store.load();
  const first=store.change('closeToTray',false);await store.change('autostartEnabled',true);expect(calls).toBe(1);expect(store.getSnapshot().pending).toBe(true);expect(store.getSnapshot().value?.closeToTray).toBe(true);
  resolve({...initial,closeToTray:false});await first;expect(store.getSnapshot().value?.closeToTray).toBe(false);
 });
 it('ne fait aucun appel système dans le navigateur',async()=>{
  const store=createDesktopSettingsStore({native:false,read:async()=>{throw 'unexpected'},change:async()=>{throw 'unexpected'}});await store.load();await store.change('autostartEnabled',true);expect(store.getSnapshot().value).toBeNull();expect(store.getSnapshot().native).toBe(false);
 });
 it('invalide annuler si un changement externe a remplacé la valeur appliquée',async()=>{
  const adapter=fake(),store=createDesktopSettingsStore(adapter);await store.load();await store.change('autostartEnabled',true);adapter.setExternal(initial);await store.load();expect(store.getSnapshot().lastChange).toBeNull();
 });
});

it('signale une valeur native différente de celle demandée puis relit la vérité',async()=>{
 const store=createDesktopSettingsStore({native:true,read:async()=>initial,change:async()=>initial});
 await store.load();await store.change('closeToTray',false);
 expect(store.getSnapshot().error).toBe('verification_failed');
 expect(store.getSnapshot().value?.closeToTray).toBe(true);expect(store.getSnapshot().lastChange).toBeNull();
});
it('garde un état inconnu si la relecture après échec échoue aussi',async()=>{
 let unreadable=false;
 const store=createDesktopSettingsStore({native:true,read:async()=>{if(unreadable)throw 'read_failed';return initial},change:async()=>{unreadable=true;throw 'autostart_failed'}});
 await store.load();await store.change('autostartEnabled',true);
 expect(store.getSnapshot().error).toBe('autostart_failed');expect(store.getSnapshot().value).toBeNull();
 expect(store.getSnapshot().pending).toBe(false);expect(store.getSnapshot().lastChange).toBeNull();
});
