// @vitest-environment jsdom
import {act} from 'react';
import {createRoot,type Root} from 'react-dom/client';
import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import type {CatalogRecord,DraftSession} from '@olc/shared';
import catalog from '../../public/game-data/catalog/en_US.json';
import {SpellWorkbench} from './SpellWorkbench';
import {SettingsProvider} from './SettingsContext';
import {spellCopy} from './spellCopy';
import {buildCopy} from './buildCopy';

const invoke=vi.hoisted(()=>vi.fn(async()=>null));
vi.mock('@tauri-apps/api/core',()=>({invoke,isTauri:()=>true}));
const draft:DraftSession={supported:true,queueId:420,allySide:'blue',allies:[{cellId:0,championId:103,locked:false,local:true,position:'middle',acting:true}],enemies:[],allyBans:[],enemyBans:[],timer:null,localSpells:[14,4]};
let root:Root,container:HTMLDivElement;
let previousMotion:string|undefined;
const previousScroll=Object.getOwnPropertyDescriptor(HTMLElement.prototype,'scrollIntoView');
beforeEach(()=>{
 vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT',true);
 Object.defineProperty(HTMLElement.prototype,'scrollIntoView',{configurable:true,value:vi.fn()});
 previousMotion=document.documentElement.dataset.motion;document.documentElement.dataset.motion='reduced';
 localStorage.clear();invoke.mockClear();
 container=document.createElement('div');document.body.append(container);root=createRoot(container);
});
afterEach(async()=>{
 await act(async()=>root.unmount());container.remove();
 if(previousScroll)Object.defineProperty(HTMLElement.prototype,'scrollIntoView',previousScroll);else delete (HTMLElement.prototype as Partial<HTMLElement>).scrollIntoView;
 if(previousMotion===undefined)delete document.documentElement.dataset.motion;else document.documentElement.dataset.motion=previousMotion;
 vi.unstubAllGlobals();
});
it.each(['fr','en'] as const)('préserve le sort équipé puis respecte une édition D/F explicite (%s)',async locale=>{
 await act(async()=>root.render(<SettingsProvider><SpellWorkbench draft={draft} championId={103} sourceKey="test-124" source={[6,14]} records={catalog.records as unknown as CatalogRecord[]} locale={locale} onOpen={()=>{}}/></SettingsProvider>));
 const t=spellCopy[locale],slots=()=>Array.from(container.querySelectorAll<HTMLElement>('.spell-slot [role="combobox"]'));
 const label=(id:number)=>(catalog.records as unknown as CatalogRecord[]).find(r=>r.kind==='summoner_spell'&&r.id===String(id))!.name;
 expect(slots().map(node=>node.textContent)).toEqual([label(14),label(6)]);
 expect(container.textContent).toContain(buildCopy[locale].spellHint);
 await act(async()=>container.querySelector<HTMLButtonElement>('.spell-import')!.click());
 expect(invoke).toHaveBeenCalledWith('import_draft_spells',{request:{championId:103,preserveEquippedSlot:true,spells:{spellIds:[14,6],flashSlot:'D'}}});
 // Placer Ghost explicitement sur D permute la paire et désactive la conservation automatique.
 await act(async()=>slots()[0]!.click());
 const popup=document.querySelector<HTMLElement>('[role="listbox"][aria-hidden="false"]')!;
 const option=Array.from(popup.querySelectorAll<HTMLElement>('[role="option"]')).find(node=>node.textContent===label(6));
 expect(option).toBeDefined();await act(async()=>option!.click());
 expect(slots().map(node=>node.textContent)).toEqual([label(6),label(14)]);
 await act(async()=>container.querySelector<HTMLButtonElement>('.spell-import')!.click());
 expect(invoke).toHaveBeenLastCalledWith('import_draft_spells',{request:{championId:103,preserveEquippedSlot:false,spells:{spellIds:[6,14],flashSlot:'D'}}});
 expect(container.querySelector('[role="status"]')?.textContent).toBe(t.accepted);
});
