import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {BuildReport} from '@olc/shared';
import {BuildSummaryView,SkillTimings,ItemTimings,BuildOmissions} from './BuildObservations';
const key={champion_id:103,patch:'16.19',platform_id:'EUW1',queue_id:420,role:'MIDDLE' as const,rank:'ALL'};
const report:BuildReport={request:{champion_id:103,patch:'16.19',platform:'EUW1',queue:420,role:'MIDDLE',rank:'ALL'},meta:{min_games:100,source_snapshot_at:'',published_at:''},builds:[],summary:{...key,games:120,wins:60,losses:60,population:200,win_rate:50,pick_rate:60},skill_levels:[{...key,point:1,slot:1,games:120,mean_timestamp_ms:45000}],item_events:[{...key,event:'ITEM_PURCHASED',item_id:1001,minute:3,events:150}],omitted_build_variants:3,omitted_build_variants_by_category:[{category:'runes',omitted:3}],max_item_events:2000,omitted_item_events:7};
it('affiche le résumé publié, temps de point et événements sans taux inventé',()=>{
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<><BuildSummaryView report={report} locale={locale}/><SkillTimings report={report} locale={locale}/><ItemTimings report={report} records={[]} locale={locale}/><BuildOmissions report={report} locale={locale}/></>);
  expect(html).toContain('120');expect(html).toContain('0:45');expect(html).toContain('150');expect(html).toContain('2000');
  expect(html).toContain(locale==='fr'?'pas le niveau du champion':'not the champion level');
  expect(html).toContain(locale==='fr'?'ventes':'sales');
  expect(html).toContain(locale==='fr'?'Runes observées':'Observed runes');
 }
});
it('ne présente pas un ancien compteur global comme propre au champion ni un taux absent',()=>{
 const legacy={...report,summary:{...report.summary!,win_rate:null,pick_rate:null},omitted_build_variants:35339,omitted_build_variants_by_category:undefined};
 expect(renderToStaticMarkup(<BuildOmissions report={legacy} locale="fr"/>)).not.toContain('35339');
 expect(renderToStaticMarkup(<BuildSummaryView report={legacy} locale="fr"/>)).not.toContain('%');
});
it('traduit les catégories enrichies et les catégories futures',()=>{
 const enriched={...report,omitted_build_variants_by_category:[{category:'rune_keystone',omitted:2},{category:'skill_start',omitted:0},{category:'future_category',omitted:1}]};
 const html=renderToStaticMarkup(<BuildOmissions report={enriched} locale="fr"/>);
 expect(html).toContain('Clé de voûte');expect(html).toContain('Trois premiers points');expect(html).toContain('Autre catégorie');
 expect(html).not.toContain('rune_keystone');expect(html).not.toContain('future_category');
});
