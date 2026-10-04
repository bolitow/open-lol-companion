import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {ItemImport} from './ItemImportAction';

const adjustments={converted:1,dropped:1};
const render=(importable:boolean,locale:'fr'|'en')=>renderToStaticMarkup(<ItemImport importable={importable} request={null} adjustments={adjustments} sourceKey="k" connected={true} locale={locale}/>);

it('désactive l’import des catégories item et trinket avec un message FR/EN et masque les signalements',()=>{
 const fr=render(false,'fr'),en=render(false,'en');
 expect(fr).toContain('Seuls l’ordre des achats et l’inventaire final sont importables.');
 expect(en).toContain('Only the purchase order and the final inventory can be imported.');
 for(const html of [fr,en]){
  expect(html).toMatch(/<button[^>]*disabled=""/);
  expect(html).not.toMatch(/remplacé|retiré|replaced|removed/);
 }
});
it('affiche les signalements remplacés/retirés quand la catégorie est importable',()=>{
 expect(render(true,'fr')).toContain('1 objet non achetable remplacé');
 expect(render(true,'fr')).toContain('1 objet sans équivalent achetable retiré');
 expect(render(true,'en')).toContain('1 unpurchasable item replaced');
 expect(render(true,'en')).toContain('1 item with no purchasable equivalent removed');
 expect(render(true,'fr')).not.toContain('Seuls l’ordre des achats');
});
