import {it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {DraftEstimateSummary} from './DraftEstimatePanel';
it('affiche estimation, couverture, population et limite descriptive en FR et EN',()=>{
 const teams={ally:{champions:3,eligible:2,mean_win_rate:54,estimated_win_rate:54},enemy:{champions:2,eligible:2,mean_win_rate:46,estimated_win_rate:46}};
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<DraftEstimateSummary locale={locale} teams={teams} status="ready" side="red" population="16.19 · EUW1 · Solo/Duo · ALL"/>);
  expect(html).toContain('54');expect(html).toContain('46');expect(html).toContain('2/3');
  expect(html.indexOf('46')).toBeLessThan(html.indexOf('54'));
  expect(html).toContain(locale==='fr'?'Estimation':'Draft estimate');expect(html).toContain('16.19');
  expect(html).toContain(locale==='fr'?'ne prédit pas':'does not predict');
 }
});
it('aucun pourcentage sans échantillon, erreur ou chargement',()=>{
 for(const status of ['loading','error','unavailable','ready'] as const){
  const html=renderToStaticMarkup(<DraftEstimateSummary locale="fr" teams={null} status={status} side="blue" population=""/>);
  expect(html).not.toContain('%');expect(html).not.toContain('50');
 }
});
