import {renderToStaticMarkup} from 'react-dom/server';
import {describe,expect,it} from 'vitest';
import {SkinImage} from './SkinImage';
describe('visuels de collection progressifs',()=>{
 it('réserve le cadre et montre un squelette pendant le chargement avec décodage asynchrone',()=>{
  const html=renderToStaticMarkup(<SkinImage url="https://assets.example/tile.jpg" label="Visuel indisponible"/>);
  expect(html).toContain('collection-skin-media');expect(html).toContain('collection-skin-skeleton');expect(html).toContain('aria-busy="true"');expect(html).toContain('loading="lazy"');expect(html).toContain('decoding="async"');expect(html).toContain('fetchPriority="auto"');
 });
 it('donne la priorité à la fiche et aux premières cartes sans image lazy',()=>{
  const html=renderToStaticMarkup(<SkinImage url="https://assets.example/splash.jpg" placeholderUrl="https://assets.example/tile.jpg" priority label="Artwork unavailable"/>);
  expect(html).toContain('loading="eager"');expect(html).toContain('fetchPriority="high"');expect(html).toContain('collection-skin-placeholder');expect(html).toContain('https://assets.example/tile.jpg');
 });
 it('ne lance aucune requête invalide et expose le repli accessible',()=>{
  const html=renderToStaticMarkup(<SkinImage url="javascript:alert(1)" label="Visuel indisponible"/>);
  expect(html).not.toContain('<img');expect(html).not.toContain('collection-skin-skeleton');expect(html).toContain('Visuel indisponible');expect(html).toContain('aria-busy="false"');
 });
});
