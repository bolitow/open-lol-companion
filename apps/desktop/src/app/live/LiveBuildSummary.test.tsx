import {expect, it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {BuildReport, BuildStats, CatalogRecord, LiveSession} from '@olc/shared';
import {LiveBuildSummary, LiveBuildSummaryContent} from './LiveBuildSummary';

const report: BuildReport = {
  request: {patch: '16.19', platform: 'EUW1', queue: 420, role: 'UTILITY', rank: 'ALL', champion_id: 432},
  meta: {min_games: 100, published_at: '2026-10-01T12:00:00Z', source_snapshot_at: '2026-10-01T11:00:00Z'}, builds: [],
};
function variant(category: string, selection: number[], games: number, wins: number): BuildStats {
  return {patch: '16.19', platform_id: 'EUW1', queue_id: 420, role: 'UTILITY', rank: 'ALL', champion_id: 432,
    category, selection, games, wins, population: 500, performance_available: true, pick_rate: games / 5, win_rate: wins / games * 100, win_rate_lower_bound: null, placement_games: 0, average_placement: null};
}
function record(id: number, kind: string, name: string, locale = 'en_US'): CatalogRecord {
  return {id: String(id), kind, name, locale, namespace: 'standard', description: null, icon: null, fields: {}, stats: {}, effects: [], coverage: {source_fields: 0, normalized_fields: 0, unmapped_fields: [], issues: []}};
}
const items = [1, 2, 3, 4, 5, 6, 7].map(id => record(id, 'item', `Local item ${id}`));
const keystone = record(8005, 'rune', 'Local keystone');
keystone.fields = {
  rune_kind: {value: 'rune', status: 'verified', unit: null, sources: []},
  slot: {value: 0, status: 'derived', unit: null, sources: []},
  style_id: {value: '8000', status: 'derived', unit: null, sources: []},
};

it('choisit la variante la plus jouée, limite à six objets sans ordre d’achat ni action', () => {
  const builds = [variant('final_items', [99], 5, 5), variant('final_items', [1, 2, 3, 4, 5, 6, 7], 50, 40)];
  const html = renderToStaticMarkup(<LiveBuildSummaryContent report={{...report, builds}} records={[...items, record(99, 'item', 'Rare item')]} locale="en" customGame={false}/>);
  expect(html).toContain('Local item 1');
  expect(html).toContain('Local item 6');
  expect(html).not.toContain('Local item 7');
  expect(html).not.toContain('Rare item');
  expect(html).toContain('50');
  expect(html).toContain('80');
  expect(html).toContain('Insufficient sample');
  expect(html).toContain('Bard');
  expect(html).toContain('Support');
  expect(html).toContain('16.19');
  expect(html).toContain('EUW1');
  expect(html).not.toMatch(/<(button|input|select|a|form|ol)\b/);
});

it('affiche la fondamentale et son échantillon séparément des objets', () => {
  const builds = [variant('final_items', [1], 50, 40), variant('runes', [8000, 8005, 9111, 9104, 8014, 8200, 8224, 8234, 5005, 5008, 5011], 120, 60)];
  const html = renderToStaticMarkup(<LiveBuildSummaryContent report={{...report, builds}} records={[...items, keystone]} locale="en" customGame={false}/>);
  expect(html).toContain('Local keystone');
  expect(html).toContain('120');
  expect(html).toContain('Independent categories');
});

it('ne publie ni taux masqué ni nom dans une autre langue', () => {
  const build = {...variant('final_items', [1], 3, 2), performance_available: false, win_rate: null};
  const html = renderToStaticMarkup(<LiveBuildSummaryContent report={{...report, builds: [build]}} records={items} locale="fr" customGame/>);
  expect(html).not.toContain('Local item');
  expect(html).toContain('—');
  expect(html).toContain('Échantillon insuffisant');
  expect(html).toContain('Partie personnalisée');
});

it('annonce une catégorie absente au lieu de montrer des objets d’une autre catégorie', () => {
  const html = renderToStaticMarkup(<LiveBuildSummaryContent report={{...report, builds: [variant('item', [1], 200, 100)]}} records={items} locale="en" customGame={false}/>);
  expect(html).toContain('No data for this category');
  expect(html).not.toContain('Local item');
});

it('ne charge pas les builds pour un rôle inconnu même avec un champion live', () => {
  const session: LiveSession = {revision: 1, generation: 1, status: 'ready', context: {championId: 432, role: null, platform: 'EUW1', queue: 420, customGame: false}, game: {gameTime: 1, gameMode: 'CLASSIC', mapNumber: 11, player: {championKey: 'Bard', level: 1, kills: 0, deaths: 0, assists: 0, creepScore: 0, items: [], currentGold: null, wardScore: null, isDead: null, respawnTimer: null, abilityLevels: null, team: null, position: null}, teams: null, events: []}, postgame: null};
  const html = renderToStaticMarkup(<LiveBuildSummary session={session} locale="en"/>);
  expect(html).toContain('Statistics unavailable');
  expect(html).not.toContain('Loading');
});
