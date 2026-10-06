import {it,expect,vi} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {DraftEstimateSummary,DraftEstimate} from './DraftEstimatePanel';
vi.mock('./useBuildPatch',()=>({buildPatchStore:{subscribe:()=>()=>{},getSnapshot:()=>({value:{client:{patch:'16.19',gameVersion:'16.19.123.4567'},manifest:{versions:['16.19']}} ,publicationRevision:0}),load:async()=>{}}}));

it('affiche estimation, couverture, population et limite descriptive en FR et EN',()=>{
 const teams={ally:{champions:3,eligible:2,mean_win_rate:54,estimated_win_rate:54},enemy:{champions:2,eligible:2,mean_win_rate:46,estimated_win_rate:46}};
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<DraftEstimateSummary locale={locale} teams={teams} status="ready" side="red" population="26.19 · EUW1 · Solo/Duo · ALL"/>);
  expect(html).toContain('54');expect(html).toContain('46');expect(html).toContain('2/3');
  expect(html.indexOf('46')).toBeLessThan(html.indexOf('54'));
  expect(html).toContain(locale==='fr'?'Estimation':'Draft estimate');expect(html).toContain('26.19');
  expect(html).toContain(locale==='fr'?'ne prédit pas':'does not predict');
 }
});
it('aucun pourcentage sans échantillon, erreur ou chargement',()=>{
 for(const status of ['loading','error','unavailable','ready'] as const){
  const html=renderToStaticMarkup(<DraftEstimateSummary locale="fr" teams={null} status={status} side="blue" population=""/>);
  expect(html).not.toContain('%');expect(html).not.toContain('50');
 }
});

it('convertit le patch technique du périmètre de draft uniquement au rendu',async()=>{
 const {PreparationContext}=await import('./PreparationContext');
 const {initialPreparation,initialState}=await import('./state');
 const draft={supported:true,queueId:420,allySide:'blue' as const,allies:[{cellId:0,championId:103,locked:true,local:true,position:'middle' as const,acting:false}],enemies:[],allyBans:[],enemyBans:[],timer:null,localSpells:null};
 const session={...initialState.session,connected:true,phase:'ChampSelect' as const,account:{game_name:'Test',tag_line:'EUW',platform:'EUW1',profile_icon_id:null},draft};
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<PreparationContext.Provider value={{value:initialPreparation,session,update:()=>{}}}><DraftEstimate draft={draft} locale={locale}/></PreparationContext.Provider>);
  expect(html).toContain('26.19');expect(html).not.toContain('16.19');
 }
});
