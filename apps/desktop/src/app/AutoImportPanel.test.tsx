import {expect,it,vi,afterEach} from 'vitest';
import {SettingsProvider} from './SettingsContext';
import {renderToStaticMarkup} from 'react-dom/server';
import {AutoImportPanel,Progress} from './AutoImportPanel';
import {initialState} from './state';

it('présente trois activations séparées, initialement décochées, en FR et EN',()=>{
    for(const locale of ['fr','en'] as const){
        const html=renderToStaticMarkup(<SettingsProvider><AutoImportPanel session={initialState.session} locale={locale} native={true}/></SettingsProvider>);
        expect(html.match(/type="checkbox"/g)).toHaveLength(3);
        expect(html).not.toContain('checked=""');
        expect(html).toContain(locale==='fr'?'Importer les sorts':'Import summoner spells');
        expect(html).toContain('min="1"');expect(html).toContain('max="1000"');
    }
});
it('l’aperçu navigateur désactive les commandes et explique le passage au desktop',()=>{
    const html=renderToStaticMarkup(<SettingsProvider><AutoImportPanel session={initialState.session} locale="fr" native={false}/></SettingsProvider>);
    expect(html.match(/disabled=""/g)).toHaveLength(5);
    expect(html).toContain('Ouvrez l’application desktop');
});
it('retire les réglages et les annonces du rendu hors de la page paramètres',()=>{
    const html=renderToStaticMarkup(<SettingsProvider><AutoImportPanel session={initialState.session} locale="fr" native={true} visible={false}/></SettingsProvider>);
    expect(html).toBe('');
});
it('signale en FR et EN les objets remplacés ou retirés du set envoyé à League',()=>{
    const progress={status:'confirmed' as const,adjustments:{converted:2,dropped:1}};
    const fr=renderToStaticMarkup(<Progress progress={progress} locale="fr" label="Importer les objets"/>);
    expect(fr).toContain('2 objets non achetables remplacés');expect(fr).toContain('1 objet sans équivalent achetable retiré du set envoyé à League');
    const en=renderToStaticMarkup(<Progress progress={progress} locale="en" label="Import items"/>);
    expect(en).toContain('2 unpurchasable items replaced');expect(en).toContain('1 item with no purchasable equivalent removed from the set sent to League');
    expect(renderToStaticMarkup(<Progress progress={{status:'confirmed',adjustments:{converted:0,dropped:0}}} locale="fr" label="x"/>)).not.toContain('achetable');
});

afterEach(()=>vi.unstubAllGlobals());
it('reprend la préférence Flash du fournisseur partagé avec les réglages',()=>{
 vi.stubGlobal('localStorage',{getItem:(key:string)=>key==='olc.flash-slot'?'"F"':key==='olc.auto-import.preferences.v1'?'{"spells":true,"minGames":100}':null,setItem:()=>{}});
 const html=renderToStaticMarkup(<SettingsProvider><AutoImportPanel session={initialState.session} locale="fr" native={true}/></SettingsProvider>);
 expect(html).toMatch(/aria-label="Flash sur F" aria-pressed="true"/);
});
