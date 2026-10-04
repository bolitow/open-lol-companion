import {SettingsProvider} from './SettingsContext';
import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {BuildReport,BuildStats,CatalogRecord} from '@olc/shared';
import catalog from '../../public/game-data/catalog/en_US.json';
import {CommunityBuildPanels} from './BuildPreparation';

it('distingue une séquence d’achats observée vide d’une catégorie inconnue',()=>{
 const variant:BuildStats={patch:'16.19',platform_id:'EUW1',queue_id:420,role:'MIDDLE',rank:'ALL',champion_id:103,category:'purchase_order',selection:[],games:120,wins:60,performance_available:true,population:200,pick_rate:60,win_rate:50,win_rate_lower_bound:null,win_rate_upper_bound:null,reliability:null,placement_games:0,average_placement:null};
 const report:BuildReport={request:{patch:'16.19',platform:'EUW1',queue:420,role:'MIDDLE',rank:'ALL',champion_id:103},meta:{min_games:100,published_at:'2026-10-01T12:00:00Z',source_snapshot_at:'2026-10-01T11:00:00Z'},builds:[variant]};
 const present=renderToStaticMarkup(<SettingsProvider><CommunityBuildPanels report={report} records={[]} locale="fr" onOpen={()=>{}}/></SettingsProvider>);
 expect(present).toContain('Aucun objet observé dans cette variante.');
 expect(present).toContain('120');
 const absent=renderToStaticMarkup(<SettingsProvider><CommunityBuildPanels report={{...report,builds:[]}} records={[]} locale="fr" onOpen={()=>{}}/></SettingsProvider>);
 expect(absent).not.toContain('Aucun objet observé dans cette variante.');
});

it('une fiche consultative ne monte ni import ni préférence Flash',()=>{
 const report:BuildReport={request:{patch:'16.19',platform:'EUW1',queue:420,role:'MIDDLE',rank:'ALL',champion_id:103},meta:{min_games:100,published_at:'2026-10-01T12:00:00Z',source_snapshot_at:'2026-10-01T11:00:00Z'},builds:[]};
 const html=renderToStaticMarkup(<CommunityBuildPanels readOnly report={report} records={[]} locale="fr" onOpen={()=>{}}/>);
 expect(html).not.toContain('Importer');
 expect(html).not.toContain('Flash sur');
 expect(html).toContain('Sorts d’invocateur');
});

it('propose l’import d’une variante d’achats et signale les objets remplacés ou retirés',()=>{
 const records=(catalog as unknown as {records:CatalogRecord[]}).records;
 // Fixture partielle : seuls la catégorie et les objets comptent ici ; les autres champs de
 // `BuildStats` (bornes, placement Arena, écarts…) sont hors sujet pour l'import d'achats.
 const variant={patch:'16.19',platform_id:'EUW1',queue_id:420,role:'MIDDLE',rank:'ALL',champion_id:103,category:'purchase_order',selection:[3070,3042,2422],games:120,wins:60,performance_available:true,population:200,pick_rate:60,win_rate:50,win_rate_lower_bound:null} as BuildStats;
 const report:BuildReport={request:{patch:'16.19',platform:'EUW1',queue:420,role:'MIDDLE',rank:'ALL',champion_id:103},meta:{min_games:100,published_at:'2026-10-01T12:00:00Z',source_snapshot_at:'2026-10-01T11:00:00Z'},builds:[variant]};
 const importContext={draft:null,equipped:null,championId:103,championName:'Ahri',sourceKey:'k'};
 const html=renderToStaticMarkup(<SettingsProvider><CommunityBuildPanels report={report} records={records} locale="fr" onOpen={()=>{}} importContext={importContext}/></SettingsProvider>);
 expect(html).toContain('Importer cette variante');
 expect(html).toContain('1 objet non achetable remplacé');
 expect(html).toContain('1 objet sans équivalent achetable retiré');
 const en=renderToStaticMarkup(<SettingsProvider><CommunityBuildPanels report={report} records={records} locale="en" onOpen={()=>{}} importContext={importContext}/></SettingsProvider>);
 expect(en).toContain('1 unpurchasable item replaced');
 expect(en).toContain('1 item with no purchasable equivalent removed');
});
