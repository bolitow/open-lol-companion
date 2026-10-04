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
  expect(html.match(/class="setting-system-note">Disponible dans l’application desktop/g)).toHaveLength(2);
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

it('rend les commandes overlay dans les résultats recherchables avec leur état desktop',()=>{
 const html=renderToStaticMarkup(<SettingsProvider><SettingsScreen reduced={false} view={{query:'overlay',category:'league'}} update={()=>{}}/></SettingsProvider>);
 expect(html).toContain('overlay-settings-title');
 expect(html).not.toContain('Aucun réglage trouvé');
 expect(html).toContain('fieldset disabled');
 expect(html).not.toContain('setting-theme');
});

it('trouve l’accès à l’API par la recherche et masque la saisie du jeton',()=>{
 for(const query of ['jeton','token','trousseau']){
  const html=renderToStaticMarkup(<SettingsProvider><SettingsScreen reduced={false} view={{query,category:'app'}} update={()=>{}}/></SettingsProvider>);
  expect(html).toContain('api-access-title');expect(html).not.toContain('Aucun réglage trouvé.');
 }
 const html=renderToStaticMarkup(<SettingsProvider><SettingsScreen reduced={false} view={{query:'jeton',category:'all'}} update={()=>{}}/></SettingsProvider>);
 expect(html).toContain('type="password"');expect(html).toContain('autoComplete="off"');
 expect(html).toContain('L’accès à l’API se configure dans l’application desktop.');
});


it('affiche le réglage et le périmètre du taux de draft',()=>{
 const html=renderToStaticMarkup(<SettingsProvider><SettingsScreen reduced={false} view={{query:'victoire draft',category:'league'}} update={()=>{}}/></SettingsProvider>);
 expect(html).toContain('aria-label="% de victoire estimé de la draft"');
 expect(html).toContain('role="switch"');
 expect(html).toContain('checked');
 expect(html).toContain('estimation descriptive en draft classée');
});
