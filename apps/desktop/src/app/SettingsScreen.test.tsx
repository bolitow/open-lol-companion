import {describe,it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {NativeControl,SettingsScreen} from './SettingsScreen';
import {SettingsProvider} from './SettingsContext';
import {createSettingsStore} from './settingsStore';
import {createDesktopSettingsStore} from './desktopSettings';

describe('réglages natifs visibles',()=>{
 it('distingue un état inconnu d’un réglage désactivé',()=>{
  const props={name:'Lancement',disabled:false,unknown:'État inconnu',enabled:'Activé',off:'Désactivé',change:()=>{}};
  const unknown=renderToStaticMarkup(<NativeControl {...props} value={null}/>);
  expect(unknown).toContain('État inconnu');expect(unknown).not.toContain('type="checkbox"');
  const off=renderToStaticMarkup(<NativeControl {...props} value={false}/>);
  expect(off).toContain('type="checkbox"');expect(off).toContain('Désactivé');expect(off).not.toContain('checked');
  const disabled=renderToStaticMarkup(<NativeControl {...props} value={true} disabled/>);
  expect(disabled).toContain('checked');expect(disabled).toContain('disabled');
 });
 it('présente les deux réglages natifs sans faux commutateur dans le navigateur',()=>{
  const html=renderToStaticMarkup(<SettingsProvider><SettingsScreen reduced={false} view={{query:'',category:'all'}} update={()=>{}}/></SettingsProvider>);
  expect(html).toContain('Fermer en arrière-plan');expect(html).toContain('Lancer à la connexion');
  expect(html.match(/Disponible dans l’application desktop/g)).toHaveLength(2);
  expect(html).not.toContain('aria-label="Lancer à la connexion"');
 });
 it('annule seulement la dernière mutation entre stockage local et système',async()=>{
  const storage={getItem:()=>null,setItem:()=>{}};
  let value={closeToTray:true,autostartEnabled:false,trayAvailable:true,storageError:false};
  const local=createSettingsStore(storage,()=>native.clearUndo());
  const native=createDesktopSettingsStore({native:true,read:async()=>value,change:async(key,next)=>{value={...value,[key]:next};return value}},()=>local.clearUndo());
  await native.load();local.change('theme','light');await native.change('closeToTray',false);
  expect(local.getSnapshot().lastChange).toBeNull();expect(native.getSnapshot().lastChange?.key).toBe('closeToTray');
  local.change('motion',false);expect(native.getSnapshot().lastChange).toBeNull();local.undo();
  expect(local.getSnapshot().values.motion).toBe(true);expect(value.closeToTray).toBe(false);
 });
});
