import {beforeEach, expect, it, vi} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {LiveSession} from '@olc/shared';
import {InGameScreen} from './InGameScreen';

const connection = vi.hoisted(() => ({session: null as LiveSession | null, native: true, error: false, retry: () => {}}));
vi.mock('./useLiveSession', () => ({useLiveSession: () => connection}));

const ready: LiveSession = {revision: 2, generation: 1, status: 'ready',
  context: {championId: 432, role: 'UTILITY', platform: 'EUW1', queue: 420, customGame: false},
  game: {gameTime: 125.8, gameMode: 'CLASSIC', mapNumber: 11,
    player: {championKey: 'Bard', level: 3, kills: 2, deaths: 1, assists: 4, creepScore: 9, items: []}, events: []},
};
beforeEach(() => { connection.session = ready; connection.native = true; connection.error = false; });

it('ouvre les données du champion réel quand le contexte est complet', () => {
  const html = renderToStaticMarkup(<InGameScreen locale="fr"/>);
  expect(html).toContain('Bard');
  expect(html).toContain('Chargement des données du champion');
});

it('garde le résumé mais aucun chargement de build si le poste manque', () => {
  connection.session = {...ready, context: {...ready.context!, role: null}};
  const html = renderToStaticMarkup(<InGameScreen locale="en"/>);
  expect(html).toContain('Bard');
  expect(html).toContain('Statistics unavailable');
  expect(html).not.toContain('Loading champion data');
});

it('ne montre plus la partie lors d’une erreur de transport', () => {
  connection.error = true;
  const html = renderToStaticMarkup(<InGameScreen locale="en"/>);
  expect(html).toContain('connection');
  expect(html).not.toContain('Bard');
  expect(html).not.toContain('Loading champion data');
});
