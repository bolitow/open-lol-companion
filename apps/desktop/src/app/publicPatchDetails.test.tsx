import {afterEach,expect,it,vi} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {ReactNode} from 'react';
import type {CatalogRecord,BuildReport} from '@olc/shared';
import catalog from '../../public/game-data/catalog/en_US.json';
import {GameDetails} from './GameDetails';
import {LiveBuilds} from './live/LiveBuilds';
import {LiveBuildSummaryContent} from './live/LiveBuildSummary';
vi.mock('react-dom',async importOriginal=>({...await importOriginal<typeof import('react-dom')>(),createPortal:(children:ReactNode)=>children}));
afterEach(()=>vi.unstubAllGlobals());
it('affiche le patch public dans les détails objet et compétence en FR/EN',()=>{
 vi.stubGlobal('document',{activeElement:null,body:{}});
 vi.stubGlobal('HTMLElement',class {});
 const item=(catalog.records as unknown as CatalogRecord[]).find(record=>record.kind==='item')!;
 const ability={...item,kind:'ability',id:'103:Q',name:'Orb'} as CatalogRecord;
 for(const locale of ['fr','en'] as const)for(const record of [item,ability]){
  const html=renderToStaticMarkup(<GameDetails record={record} records={[record]} locale={locale} version="16.19.1" onClose={()=>{}} onOpen={()=>{}}/>);
  expect(html).toContain('26.19');expect(html).not.toContain('16.19.1');
  const invalid=renderToStaticMarkup(<GameDetails record={record} records={[record]} locale={locale} version="invalid-version" onClose={()=>{}} onOpen={()=>{}}/>);
  expect(invalid).toContain('—');expect(invalid).not.toContain('invalid-version');
 }
});

it('ne montre pas un patch invalide dans les deux vues en partie en FR/EN',()=>{
 const report:BuildReport={request:{patch:'invalid-version',platform:'EUW1',queue:420,role:'MIDDLE',rank:'ALL',champion_id:103},meta:{min_games:100,published_at:'',source_snapshot_at:''},builds:[]};
 for(const locale of ['fr','en'] as const)for(const view of [<LiveBuilds report={report} records={[]} locale={locale} customGame={false} onOpen={()=>{}}/>,<LiveBuildSummaryContent report={report} records={[]} locale={locale} customGame={false}/>]){
  const html=renderToStaticMarkup(view);expect(html).not.toContain('invalid-version');expect(html).toContain('—');
 }
});
