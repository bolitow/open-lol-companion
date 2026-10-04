import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {CatalogRecord} from '@olc/shared';
import {AbilityDescription} from './AbilityInfo';
const text='Inflige des dégâts magiques. '.repeat(15)+'Fin du fonctionnement.';
const record={kind:'ability',id:'103:passive',description:text,fields:{},stats:{},effects:[]} as unknown as CatalogRecord;
it('affiche le fonctionnement entier sans dépliant dans la vue agrandie',()=>{
 const html=renderToStaticMarkup(<AbilityDescription record={record} locale="fr" expanded/>);
 expect(html).toContain('Fin du fonctionnement.');expect(html).not.toContain('aria-expanded');expect(html).not.toContain('inert');expect(html).not.toContain('<button');
});
it('conserve le fonctionnement dépliable dans la fiche compacte',()=>{
 expect(renderToStaticMarkup(<AbilityDescription record={record} locale="en"/>)).toContain('aria-expanded="false"');
});
