import {expect, it} from 'vitest';
import type {CatalogRecord} from '@olc/shared';
import type {PreparationCatalog} from '../catalog';
import {connectLiveCatalog, type LiveCatalogState} from './liveCatalog';

const catalog: PreparationCatalog = {version: '16.19.1', records: []};
const ability: CatalogRecord = {kind: 'ability', id: '432:Q', namespace: 'standard', locale: 'en_US',
  name: 'Cosmic Binding', description: null, icon: null, fields: {}, stats: {}, effects: [],
  coverage: {source_fields: 0, normalized_fields: 0, unmapped_fields: [], issues: []}};
const flush = async () => { await Promise.resolve(); await Promise.resolve(); await Promise.resolve(); };

it('publie le catalogue puis les compétences du seul champion actuel', async () => {
  const states: LiveCatalogState[] = [];
  let finish = (_: CatalogRecord[]) => {};
  const reads: unknown[][] = [];
  connectLiveCatalog('fr', 432, state => states.push(state), {
    catalog: async locale => { reads.push([locale]); return catalog; },
    abilities: (locale, id, version) => { reads.push([locale, id, version]); return new Promise(resolve => { finish = resolve; }); },
  });
  await flush();
  expect(states).toEqual([{catalog, abilities: [], error: false}]);
  finish([ability]); await flush();
  expect(states.at(-1)).toEqual({catalog, abilities: [ability], error: false});
  expect(reads).toEqual([['fr'], ['fr', 432, '16.19.1']]);
});

it('ignore le catalogue tardif quand le contexte a disparu', async () => {
  const states: LiveCatalogState[] = [];
  let finish = (_: PreparationCatalog) => {};
  let abilityReads = 0;
  const stop = connectLiveCatalog('en', 432, state => states.push(state), {
    catalog: () => new Promise(resolve => { finish = resolve; }),
    abilities: async () => { abilityReads++; return [ability]; },
  });
  stop(); finish(catalog); await flush();
  expect(states).toEqual([]);
  expect(abilityReads).toBe(0);
});

it('ignore les compétences tardives du champion précédent', async () => {
  const states: LiveCatalogState[] = [];
  let finish = (_: CatalogRecord[]) => {};
  const stop = connectLiveCatalog('en', 432, state => states.push(state), {
    catalog: async () => catalog,
    abilities: () => new Promise(resolve => { finish = resolve; }),
  });
  await flush(); stop(); finish([ability]); await flush();
  expect(states).toEqual([{catalog, abilities: [], error: false}]);
});

it('garde les runes et objets lisibles si la fiche de compétences échoue', async () => {
  const states: LiveCatalogState[] = [];
  connectLiveCatalog('en', 432, state => states.push(state), {
    catalog: async () => catalog, abilities: async () => { throw new Error('asset'); },
  });
  await flush();
  expect(states.at(-1)).toEqual({catalog, abilities: [], error: false});
});

it('signale un catalogue indisponible sans données précédentes', async () => {
  const states: LiveCatalogState[] = [];
  connectLiveCatalog('en', 432, state => states.push(state), {
    catalog: async () => { throw new Error('asset'); }, abilities: async () => [],
  });
  await flush();
  expect(states).toEqual([{catalog: null, abilities: [], error: true}]);
});
