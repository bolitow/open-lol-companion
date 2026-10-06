// @vitest-environment jsdom
import {act} from 'react';
import {createRoot,type Root} from 'react-dom/client';
import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import type {BuildReport,BuildStats,CatalogRecord} from '@olc/shared';
import catalog from '../../public/game-data/catalog/en_US.json';
import {CommunityBuildPanels} from './BuildPreparation';
import {SettingsProvider} from './SettingsContext';
import {buildCopy} from './buildCopy';

const request:BuildReport['request']={patch:'16.19',platform:'EUW1',queue:420,role:'MIDDLE',rank:'ALL',champion_id:103};
const variant=(category:BuildStats['category'],selection:number[]):BuildStats=>({patch:request.patch,platform_id:request.platform,queue_id:request.queue,role:request.role,rank:request.rank,champion_id:request.champion_id,category,selection,games:120,wins:60,omitted_variants:null,performance_available:true,population:200,pick_rate:60,win_rate:50,win_rate_lower_bound:null,conditional_rate:null,win_rate_upper_bound:null,win_rate_delta:null,reliability:null,placement_games:0,average_placement:null});
const report:BuildReport={request,meta:{min_games:100,published_at:'2026-10-01T12:00:00Z',source_snapshot_at:'2026-10-01T11:00:00Z'},builds:[variant('purchase_order',[3042,3004]),variant('final_items',[3004,3042]),variant('item',[3004]),variant('trinket',[3340])]};
let root:Root,container:HTMLDivElement;
const previousScroll=Object.getOwnPropertyDescriptor(HTMLElement.prototype,'scrollIntoView');
let previousMotion:string|undefined;
beforeEach(()=>{
 vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT',true);
 // jsdom n'effectue pas de mise en page ; le dropdown peut néanmoins défiler vers l'option active.
 Object.defineProperty(HTMLElement.prototype,'scrollIntoView',{configurable:true,value:vi.fn()});
 previousMotion=document.documentElement.dataset.motion;document.documentElement.dataset.motion='reduced';
 localStorage.clear();
 container=document.createElement('div');document.body.append(container);root=createRoot(container);
});
afterEach(async()=>{
 await act(async()=>root.unmount());container.remove();
 if(previousScroll)Object.defineProperty(HTMLElement.prototype,'scrollIntoView',previousScroll);else delete (HTMLElement.prototype as Partial<HTMLElement>).scrollIntoView;
 if(previousMotion===undefined)delete document.documentElement.dataset.motion;else document.documentElement.dataset.motion=previousMotion;
 vi.unstubAllGlobals();
});
it.each(['fr','en'] as const)('bloque les objets isolés et les reliques via le vrai dropdown (%s)',async locale=>{
 await act(async()=>root.render(<SettingsProvider><CommunityBuildPanels report={report} records={catalog.records as unknown as CatalogRecord[]} locale={locale} onOpen={()=>{}} connected importContext={{draft:null,equipped:null,championId:103,championName:'Ahri',sourceKey:'test-88'}}/></SettingsProvider>));
 const t=buildCopy[locale],blocked=locale==='fr'?'Seuls l’ordre des achats et l’inventaire final sont importables.':'Only the purchase order and the final inventory can be imported.';
 const imports=()=>container.querySelector<HTMLElement>('.item-import')!;
 const choose=async(category:'purchase_order'|'final_items'|'item'|'trinket')=>{
  const trigger=container.querySelector<HTMLButtonElement>('.build-item-panel [role="combobox"]')!;
  await act(async()=>trigger.click());
  const popup=document.querySelector<HTMLElement>('[role="listbox"][aria-hidden="false"]')!;
  const option=Array.from(popup.querySelectorAll<HTMLElement>('[role="option"]')).find(node=>node.textContent===t[category]);
  expect(option).toBeDefined();await act(async()=>option!.click());
  expect(trigger.textContent).toBe(t[category]);
 };
 const converted=locale==='fr'?'1 objet non achetable remplacé':'1 unpurchasable item replaced';
 // En navigateur, les écritures natives restent indisponibles, mais le plan et ses ajustements sont visibles.
 expect(imports().textContent).toContain(converted);
 for(const category of ['item','trinket'] as const){
  await choose(category);
  expect(imports().querySelector<HTMLButtonElement>('button')?.disabled).toBe(true);
  expect(imports().querySelector('[role="status"]')?.textContent).toBe(blocked);
  expect(imports().textContent).not.toContain(converted);
 }
 for(const category of ['final_items','purchase_order'] as const){
  await choose(category);
  expect(imports().querySelector('[role="status"]')?.textContent).not.toBe(blocked);
  expect(imports().textContent).toContain(converted);
 }
});
