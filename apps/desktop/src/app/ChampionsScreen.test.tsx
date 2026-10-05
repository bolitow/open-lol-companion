import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {ChampionsScreen} from './ChampionsScreen';
import {initialChampions} from './state';
import {championsCopy} from './championsCopy';
it('donne la priorité aux cartes et aux filtres sans introduction redondante, en FR et EN',()=>{
 for(const locale of ['fr','en'] as const){
  const t=championsCopy[locale];
  const html=renderToStaticMarkup(<ChampionsScreen locale={locale} state={initialChampions} update={()=>{}}/>);
  expect(html).not.toContain('class="champions-heading"');
  expect(html).toContain(`aria-label="${t.title}"`);
  expect(html).toContain(`aria-label="${t.search}"`);
  expect(html).toContain(`aria-label="${t.sort}"`);
  expect(html).not.toContain('<select');expect(html).toContain('role="combobox"');
  expect(html).toContain('Ahri');expect(html).toContain('aria-pressed="false"');
 }
});
it('conserve le filtre et la sélection du champion dans la bibliothèque compacte',()=>{
 const html=renderToStaticMarkup(<ChampionsScreen locale="fr" state={{...initialChampions,query:'ahri',selected:103}} update={()=>{}}/>);
 expect(html).toContain('aria-pressed="true"');expect(html).toContain('value="ahri"');
 expect(html).toContain('Fermer la fiche');expect(html).not.toContain('Aatrox');
});
