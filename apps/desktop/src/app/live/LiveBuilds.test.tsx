import {expect, it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {BuildReport} from '@olc/shared';
import {LiveBuilds} from './LiveBuilds';

const report: BuildReport = {
  request: {patch: '16.19', platform: 'EUW1', queue: 420, role: 'UTILITY', rank: 'ALL', champion_id: 432},
  meta: {min_games: 100, published_at: '2026-10-01T12:00:00Z', source_snapshot_at: '2026-10-01T11:00:00Z'},
  builds: [{patch: '16.19', platform_id: 'EUW1', queue_id: 420, role: 'UTILITY', rank: 'ALL', champion_id: 432, category: 'purchase_order', selection: [], games: 50, wins: 40, performance_available: true, population: 200, pick_rate: 25, win_rate: 80, win_rate_lower_bound: null, conditional_rate: null}],
};

it('présente les statistiques sans import ni réglage des sorts dans les deux langues', () => {
  for (const locale of ['fr', 'en'] as const) {
    const html = renderToStaticMarkup(<LiveBuilds report={report} records={[]} locale={locale} customGame={false} onOpen={() => {}}/>);
    expect(html).toContain('50');
    expect(html).toContain('EUW1');
    expect(html).toContain('Support');
    expect(html).toContain('16.19');
    expect(html).not.toMatch(/Importer|Import items|Import runes|Flash sur|Flash on/);
    expect(html).toContain(locale === 'fr' ? 'Échantillon insuffisant' : 'Insufficient sample');
  }
});

it('nomme la population classée utilisée en partie personnalisée', () => {
  const html = renderToStaticMarkup(<LiveBuilds report={report} records={[]} locale="fr" customGame onOpen={() => {}}/>);
  expect(html).toContain('Partie personnalisée');
  expect(html).toContain('Solo/Duo');
});

it('présente explicitement l’absence de données', () => {
  const html = renderToStaticMarkup(<LiveBuilds report={{...report, builds: []}} records={[]} locale="en" customGame={false} onOpen={() => {}}/>);
  expect(html).toContain('No data yet');
  expect(html).not.toContain('community-build-grid');
});
