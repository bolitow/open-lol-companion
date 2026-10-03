import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {AutoImportPanel} from './AutoImportPanel';
import {initialState} from './state';

it('présente trois activations séparées, initialement décochées, en FR et EN',()=>{
    for(const locale of ['fr','en'] as const){
        const html=renderToStaticMarkup(<AutoImportPanel session={initialState.session} locale={locale} native={true}/>);
        expect(html.match(/type="checkbox"/g)).toHaveLength(3);
        expect(html).not.toContain('checked=""');
        expect(html).toContain(locale==='fr'?'Importer les sorts':'Import summoner spells');
        expect(html).toContain('min="1"');expect(html).toContain('max="1000"');
    }
});
it('l’aperçu navigateur désactive les commandes et explique le passage au desktop',()=>{
    const html=renderToStaticMarkup(<AutoImportPanel session={initialState.session} locale="fr" native={false}/>);
    expect(html.match(/disabled=""/g)).toHaveLength(5);
    expect(html).toContain('Ouvrez l’application desktop');
});
it('retire les réglages et les annonces du rendu hors de la page paramètres',()=>{
    const html=renderToStaticMarkup(<AutoImportPanel session={initialState.session} locale="fr" native={true} visible={false}/>);
    expect(html).toBe('');
});
