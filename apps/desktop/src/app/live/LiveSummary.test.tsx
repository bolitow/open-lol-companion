import {expect, it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {LiveSession} from '@olc/shared';
import {LiveSummary} from './LiveSummary';

const session: LiveSession = {revision: 2, generation: 1, status: 'ready', context: null, game: {
  gameTime: 125.8, gameMode: 'CLASSIC', mapNumber: 11,
  player: {championKey: 'Bard', level: 3, kills: 2, deaths: 1, assists: 4, creepScore: 9, items: []}, events: [],
}};

it('affiche le joueur actuel en lecture seule dans les deux langues', () => {
  for (const locale of ['fr', 'en'] as const) {
    const html = renderToStaticMarkup(<LiveSummary session={session} locale={locale}/>);
    expect(html).toContain('Bard');
    expect(html).toContain('2 / 1 / 4');
    expect(html).toContain('2:05');
    expect(html).toContain('<dd>9</dd>');
    expect(html).toContain(locale === 'fr' ? 'Niveau' : 'Level');
    expect(html).not.toMatch(/<button|<select|<input/);
  }
});

it('retire toutes les statistiques lorsque le flux devient indisponible', () => {
  const html = renderToStaticMarkup(<LiveSummary session={{...session, status: 'unavailable'}} locale="fr"/>);
  expect(html).toContain('role="status"');
  expect(html).not.toContain('Bard');
  expect(html).not.toContain('2:05');
  expect(html).not.toContain('<dd>');
});

it('n’invente pas un champion lorsque sa clé manque au catalogue', () => {
  const html = renderToStaticMarkup(<LiveSummary session={{...session, game: {...session.game!, player: {...session.game!.player, championKey: 'SecretUnknownChampion'}}}} locale="en"/>);
  expect(html).toContain('Unknown champion');
  expect(html).not.toContain('SecretUnknownChampion');
});

it('conserve chaque CS reçu et explique les paliers possibles de la source LoL',()=>{
 for(const locale of ['fr','en'] as const){
  for(const cs of [9,10,11,19,20]){
   const html=renderToStaticMarkup(<LiveSummary session={{...session,game:{...session.game!,player:{...session.game!.player,creepScore:cs}}}} locale={locale}/>);
   expect(html).toContain(`<dd>${cs}</dd>`);
   expect(html).toContain(locale==='fr'?'LoL peut transmettre les CS par paliers de 10.':'League may report CS in increments of 10.');
  }
 }
});
