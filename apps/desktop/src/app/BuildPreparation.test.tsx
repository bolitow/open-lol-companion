import {SettingsProvider} from './SettingsContext';
import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {BuildReport,BuildStats,CatalogRecord} from '@olc/shared';
import catalog from '../../public/game-data/catalog/en_US.json';
import {CommunityBuildPanels} from './BuildPreparation';

it('distingue une séquence d’achats observée vide d’une catégorie inconnue',()=>{
 const variant:BuildStats={patch:'16.19',platform_id:'EUW1',queue_id:420,role:'MIDDLE',rank:'ALL',champion_id:103,category:'purchase_order',selection:[],games:120,wins:60,omitted_variants:null,performance_available:true,population:200,pick_rate:60,win_rate:50,win_rate_lower_bound:null,conditional_rate:null,win_rate_upper_bound:null,win_rate_delta:null,reliability:null,placement_games:0,average_placement:null};
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
 const variant={patch:'16.19',platform_id:'EUW1',queue_id:420,role:'MIDDLE',rank:'ALL',champion_id:103,category:'purchase_order',selection:[3070,3042,2422],games:120,wins:60,omitted_variants:null,performance_available:true,population:200,pick_rate:60,win_rate:50,win_rate_lower_bound:null} as BuildStats;
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

it('garde le champion local et explique les limites du matchup choisi en FR/EN',async()=>{
 const {BuildPreparation}=await import('./BuildPreparation');
 const {PreparationContext}=await import('./PreparationContext');
 const {initialPreparation,initialState}=await import('./state');
 const draft={supported:true,queueId:440,allySide:'blue' as const,allies:[{cellId:0,championId:432,locked:true,local:true,position:'utility' as const,acting:false}],enemies:[],allyBans:[],enemyBans:[],timer:null,localSpells:null};
 const value={...initialPreparation,platform:'NA1',queue:440,matchup:{championId:103,kind:'chosen' as const}};
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<SettingsProvider><PreparationContext.Provider value={{value,session:{...initialState.session,draft},update:()=>{}}}><BuildPreparation draft={draft} equipped={null} catalog={catalog as never} locale={locale} onOpen={()=>{}}/></PreparationContext.Provider></SettingsProvider>);
  expect(html).toContain(locale==='fr'?'Matchup choisi':'Selected matchup');
  expect(html).toContain(locale==='fr'?'Données de matchup indisponibles':'Matchup data unavailable');
  expect(html).toMatch(/aria-label="(?:Champion à consulter|Champion to browse)"[^>]*><span>Bard<\/span>/);
  expect(html).toMatch(/aria-label="(?:Poste|Role)"[^>]*><span>Support<\/span>/);
  expect(html).toMatch(/aria-label="(?:Région|Region)"[^>]*><span>NA1<\/span>/);
 }
});
