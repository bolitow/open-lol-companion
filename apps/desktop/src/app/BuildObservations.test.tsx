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

it('explique le pick rate par parties publié et distingue la fréquence des builds en FR/EN',()=>{
 for(const locale of ['fr','en'] as const){
  const current={...report,meta:{...report.meta,pick_rate_definition:'champion_matches / bucket_matches * 100'}};
  const html=renderToStaticMarkup(<BuildSummaryView report={current} locale={locale}/>);
  expect(html).toContain(locale==='fr'?'Présence dans les parties':'Match presence');
  expect(html).toContain(locale==='fr'?'Parties avec ce champion / parties du compartiment':'Matches with this champion / matches in this group');
  expect(html).toContain('champion_matches / bucket_matches * 100');
  expect(html).toContain(locale==='fr'?'Fréquence des builds':'Build frequency');
  expect(html).toContain(locale==='fr'?'participations éligibles de cette catégorie':'eligible participations in this category');
  expect(html).toContain('60 %');
 }
});

it('garde la formule historique par participations sans la présenter comme une présence en partie',()=>{
 for(const locale of ['fr','en'] as const){
  const legacy={...report,meta:{...report.meta,pick_rate_definition:'champion_participations / bucket_participations * 100'}};
  const html=renderToStaticMarkup(<BuildSummaryView report={legacy} locale={locale}/>);
  expect(html).toContain(locale==='fr'?'Part des sélections':'Selection share');
  expect(html).toContain(locale==='fr'?'Participations du champion / participations du compartiment':'Champion participations / participations in this group');
  expect(html).toContain('champion_participations / bucket_participations * 100');
  expect(html).not.toContain(locale==='fr'?'Présence dans les parties':'Match presence');
  expect(html).not.toContain('champion_matches / bucket_matches');
 }
});

it('signale la définition absente ou inconnue sans déduire la méthode du patch',()=>{
 for(const locale of ['fr','en'] as const){
  for(const patch of ['16.19','16.20']){
   for(const definition of [undefined,'','   ','future_rate / future_population * 100']){
    const unknown={...report,request:{...report.request,patch},meta:{...report.meta,pick_rate_definition:definition}};
    const html=renderToStaticMarkup(<BuildSummaryView report={unknown} locale={locale}/>);
    expect(html).toContain(locale==='fr'?'Taux de sélection':'Pick rate');
    expect(html).toContain(definition?.trim()?(locale==='fr'?'Définition non reconnue':'Unrecognized definition'):(locale==='fr'?'Définition non fournie':'Definition not provided'));
    expect(html).not.toContain(locale==='fr'?'Présence dans les parties':'Match presence');
    expect(html).not.toContain('champion_matches / bucket_matches');
    if(definition?.trim())expect(html).toContain(definition);
   }
  }
 }
});

it('explique la méthode sans reconstituer un taux masqué',()=>{
 const masked={...report,meta:{...report.meta,pick_rate_definition:'champion_matches / bucket_matches * 100'},summary:{...report.summary!,games:1,population:3,win_rate:null,pick_rate:null}};
 const html=renderToStaticMarkup(<BuildSummaryView report={masked} locale="fr"/>);
 expect(html).toContain('champion_matches / bucket_matches * 100');
 expect(html).not.toContain('%');
});
