import {expect,it,vi,afterEach} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {AutoImportProvider} from './AutoImportPanel';
import {SettingsProvider} from './SettingsContext';
import {SettingsScreen} from './SettingsScreen';
import {initialState} from './state';

const render=(query:string,native=true)=>renderToStaticMarkup(<SettingsProvider><AutoImportProvider session={initialState.session} locale="fr" native={native}><SettingsScreen reduced={false} view={{query,category:'league'}} update={()=>{}}/></AutoImportProvider></SettingsProvider>);
it('affiche uniquement le réglage de runes demandé et son statut sans panneau replié',()=>{
 const html=render('runes');
 expect(html).toContain('aria-label="Importer les runes"');expect(html).not.toContain('checked=""');
 expect(html).not.toContain('aria-label="Importer les objets"');expect(html.indexOf('aria-label="Importer les runes"')).toBeLessThan(html.indexOf('<details'));
 expect(html).toContain('Vos autres pages sont conservées.');
});
it('la recherche imports présente les cinq commandes modifiables',()=>{
 const html=render('imports');expect(html.match(/class="surface setting-card"/g)).toHaveLength(5);
 expect(html).toContain('min="1"');expect(html).toContain('max="1000"');expect(html).toContain('Poste en personnalisée');
});
it('désactive les commandes des imports dans le navigateur avec une explication',()=>{
 const html=render('imports',false);
 expect(html).toContain('Ouvrez l’application desktop');
 expect(html).toMatch(/aria-label="Importer les runes"[^>]*disabled/);
});
it('le moteur sans réglages rend uniquement la page active, sans annonce cachée',()=>{
 const html=renderToStaticMarkup(<SettingsProvider><AutoImportProvider session={initialState.session} locale="fr" native><p>Draft</p></AutoImportProvider></SettingsProvider>);
 expect(html).toBe('<p>Draft</p>');
});

afterEach(()=>vi.unstubAllGlobals());
it('reprend la préférence Flash du fournisseur partagé avec les réglages',()=>{
 vi.stubGlobal('localStorage',{getItem:(key:string)=>key==='olc.flash-slot'?'"F"':key==='olc.auto-import.preferences.v1'?'{"spells":true,"minGames":100}':null,setItem:()=>{}});
 const html=renderToStaticMarkup(<SettingsProvider><AutoImportProvider session={initialState.session} locale="fr" native><SettingsScreen reduced={false} view={{query:"imports sorts",category:"league"}} update={()=>{}}/></AutoImportProvider></SettingsProvider>);
 expect(html).toMatch(/aria-label="Flash sur F" aria-pressed="true"/);
});

it('présente les trois activations séparées décochées en FR et EN',()=>{
 for(const locale of ['fr','en'] as const){
  vi.stubGlobal('localStorage',{getItem:(key:string)=>key==='olc.app.preferences'?JSON.stringify({locale}):null,setItem:()=>{}});
  const html=renderToStaticMarkup(<SettingsProvider><AutoImportProvider session={initialState.session} locale={locale} native><SettingsScreen reduced={false} view={{query:'imports',category:'league'}} update={()=>{}}/></AutoImportProvider></SettingsProvider>);
  expect(html.match(/role="switch"/g)).toHaveLength(3);
  expect(html).not.toContain('checked=""');
  expect(html).toContain(locale==='fr'?'Importer les sorts':'Import summoner spells');
 }
});
