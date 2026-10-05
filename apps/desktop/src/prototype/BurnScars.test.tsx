import {describe,it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {BurnScars} from './BurnScars';
describe('matière des panneaux sur WebKit',()=>{
 it('porte le flou dans le SVG et isole les filtres entre deux cartes',()=>{
  const html=renderToStaticMarkup(<><BurnScars/><BurnScars/></>);
  const ids=[...html.matchAll(/<filter id="([^"]+)"/g)].map(m=>m[1]);
  expect(ids).toHaveLength(2);expect(new Set(ids).size).toBe(2);
  for(const id of ids)expect(html).toContain(`filter="url(#${id})"`);
  expect(html).toContain('feGaussianBlur');
 });
});
