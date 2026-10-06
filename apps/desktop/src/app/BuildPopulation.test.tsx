import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {BuildReport} from '@olc/shared';
import {BuildPopulation} from './BuildPopulation';
const report:BuildReport={request:{champion_id:103,patch:'16.19',platform:'EUW1',queue:420,role:'MIDDLE',rank:'ALL'},meta:{min_games:100,source_snapshot_at:'',published_at:'',population_label:'collected_sample',coverage:[{patch:'16.19',platform_id:'EUW1',queue_id:420,ranked_participations:100,tier_participations:{GOLD:8,MASTER:92},apex_share:.92,high_elo_biased:true}]},builds:[]};
it('montre parts et effectifs de tous les rôles, badge seulement sur drapeau serveur',()=>{
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<BuildPopulation report={report} locale={locale}/>);
  expect(html).toContain(locale==='fr'?'Échantillon collecté':'Collected sample');
  expect(html).toContain(locale==='fr'?'Échantillon surtout Master+':'Mostly Master+ sample');
  expect(html).toContain(locale==='fr'?'tous rôles':'all roles');
  expect(html).toContain('26.19');expect(html).not.toContain('16.19');
  expect(report.request.patch).toBe('16.19');
  expect(html).toContain('92');expect(html).toContain('100');
 }
 const quiet={...report,meta:{...report.meta,coverage:[{...report.meta.coverage![0]!,high_elo_biased:false}]}};
 expect(renderToStaticMarkup(<BuildPopulation report={quiet} locale="en"/>)).not.toContain('Mostly Master+ sample');
});
it('tolère anciens instantanés, répartition vide et libellés inconnus',()=>{
 const legacy={...report,meta:{min_games:100,source_snapshot_at:'',published_at:''}};
 expect(renderToStaticMarkup(<BuildPopulation report={legacy} locale="en"/>)).toContain('Collected sample');
 const unknown={...legacy,meta:{...legacy.meta,population_label:'unknown_match_tier'}};
 const html=renderToStaticMarkup(<BuildPopulation report={unknown} locale="en"/>);
 expect(html).toContain('Statistical population');expect(html).not.toContain('Mostly Master+ sample');
});
