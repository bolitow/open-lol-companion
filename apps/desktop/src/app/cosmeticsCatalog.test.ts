import {afterEach,expect,it,vi} from 'vitest';
import {createCatalogRuntime,catalogRuntime} from './catalogRuntime';
import {profileIconUrl} from './AccountControl';
import {skinImageUrl,skinLineEntries} from './cosmeticsAssets';
import {collectionImageUrl,filterCollection,initialCollectionState,initialCollectionView} from './collection/collectionModel';
const id='a'.repeat(64),other='b'.repeat(64);
const state=(snapshotId=id)=>({status:'ready' as const,version:'16.20.1',snapshotId,assetBase:`catalog://localhost/${snapshotId}/`,error:null});
const directory={version:'16.20.1',champions:[{id:999,key:'New',names:{fr:'Nouveau',en:'New'},titles:{fr:'Titre',en:'Title'},categories:['Mage'],image:'champions/999.png'}]};
const cosmetics={schema_version:1,version:'16.20.1',profile_icons:[0,99999],skin_lines:{fr:{'0':'','9':'Nouvelle série'},en:{'0':'','9':'New series'}},skins:{'999001':{tile:'assets/characters/new/tile.jpg',splash:'assets/characters/new/splash.jpg'}}};
const read=async(_id:string,path:string):Promise<unknown>=>path==='champions.json'?{'999':{key:'New',fr:'Nouveau',en:'New'}}:path==='cosmetics.json'?cosmetics:directory;
afterEach(()=>vi.restoreAllMocks());
it('active les métadonnées ensemble et résout seulement les nouveaux identifiants déclarés',async()=>{
 const runtime=createCatalogRuntime({read});await runtime.accept(state());vi.spyOn(catalogRuntime,'getSnapshot').mockImplementation(runtime.getSnapshot);
 expect(profileIconUrl(99999)).toBe(`cosmetic://localhost/${id}/profile/99999`);expect(profileIconUrl(88)).toBeNull();
 expect(skinImageUrl(999001,'tile','https://raw.communitydragon.org/latest/old.jpg')).toBe(`cosmetic://localhost/${id}/tile/999001`);
 expect(skinImageUrl(999002,'splash','https://raw.communitydragon.org/latest/old.jpg')).toBeNull();
 expect(skinLineEntries()).toEqual([{id:9,names:{fr:'Nouvelle série',en:'New series'}}]);
 const skin={id:999001,champion_id:999,name:'New',ownership:'owned' as const,tile_url:null,splash_url:null,obtainable:null,rarity:null,series_ids:[9]};
 expect(filterCollection({...initialCollectionState(false),skins:[skin]},{...initialCollectionView,query:'nouvelle série'},'fr')).toEqual([skin]);
});
it('préserve le repli d’un ancien paquet sans latest et distingue corruption ou erreur',async()=>{
 const legacy=createCatalogRuntime({read:async(i,p)=>p==='cosmetics.json'?null:read(i,p)});await legacy.accept(state());
 vi.spyOn(catalogRuntime,'getSnapshot').mockImplementation(legacy.getSnapshot);
 expect(legacy.getSnapshot().status).toBe('ready');expect(profileIconUrl(0)).toMatch(/\/cdn\/16\.19\.1\/img\/profileicon\/0\.png$/);
 expect(skinImageUrl(1,'tile','https://raw.communitydragon.org/latest/old.jpg')).toBeNull();
 expect(skinImageUrl(1,'tile','https://raw.communitydragon.org/16.20/old.jpg')).toBe('https://raw.communitydragon.org/16.20/old.jpg');
 for(const invalid of [{...cosmetics,version:'16.19.1'},{...cosmetics,profile_icons:[0,0]},{...cosmetics,skin_lines:{fr:{'9':'A'},en:{}}},{...cosmetics,skins:{'999001':{tile:'../escape.png',splash:null}}}]){
  const runtime=createCatalogRuntime({read:async(i,p)=>p==='cosmetics.json'?invalid:read(i,p)});await runtime.accept(state());expect(runtime.getSnapshot().active).toBeNull();
 }
 const failure=createCatalogRuntime({read:async(i,p)=>{if(p==='cosmetics.json')throw Error('disk');return read(i,p)}});await failure.accept(state());expect(failure.getSnapshot().status).toBe('error');
});
it('une réponse cosmétique tardive ne remplace pas la nouvelle publication',async()=>{
 let release!:()=>void;const blocked=new Promise<void>(r=>release=r);
 const runtime=createCatalogRuntime({read:async(i,p)=>{if(i===id&&p==='cosmetics.json')await blocked;return read(i,p)}});
 const first=runtime.accept(state());await runtime.accept(state(other));release();await first;
 expect(runtime.getSnapshot().active?.snapshotId).toBe(other);expect(runtime.getSnapshot().generation).toBe(1);
 vi.spyOn(catalogRuntime,'getSnapshot').mockImplementation(runtime.getSnapshot);expect(profileIconUrl(0)).toContain(other);
});
it('génère le protocole Windows et refuse les URLs locales non déclarées ou détournées',async()=>{
 const runtime=createCatalogRuntime({read});await runtime.accept({...state(),assetBase:`http://catalog.localhost/${id}/`});vi.spyOn(catalogRuntime,'getSnapshot').mockImplementation(runtime.getSnapshot);
 const url=`http://cosmetic.localhost/${id}/tile/999001`;expect(skinImageUrl(999001,'tile',null)).toBe(url);expect(collectionImageUrl(url)).toBe(url);
 for(const invalid of [`http://cosmetic.localhost/${id}/tile/999002`,`http://cosmetic.localhost.evil/${id}/tile/999001`,`cosmetic://localhost/${id}/tile/999001?x=1`,`cosmetic://localhost/${id}/tile/../999001`,`http://cosmetic.localhost/${id}/profile/0`])expect(collectionImageUrl(invalid)).toBeNull();
});
it('raccorde réellement cartes, aperçu et avatar sans modifier les données de possession',async()=>{
 const {renderToStaticMarkup}=await import('react-dom/server');
 const {createElement}=await import('react');
 const {CollectionScreen}=await import('./collection/CollectionScreen');
 const {AccountControl}=await import('./AccountControl');const {copy}=await import('./copy');
 const runtime=createCatalogRuntime({read});await runtime.accept(state());vi.spyOn(catalogRuntime,'getSnapshot').mockImplementation(runtime.getSnapshot);
 const skin={id:999001,champion_id:999,name:'Nouvelle apparence',ownership:'owned' as const,tile_url:'https://raw.communitydragon.org/latest/old.jpg',splash_url:'https://raw.communitydragon.org/latest/old-splash.jpg',obtainable:null,rarity:null,series_ids:[9]};
 const collection={state:{...initialCollectionState(false),status:'ready' as const,skins:[skin]},pending:false,error:null,refresh:vi.fn(async()=>{}),setWish:vi.fn(async()=>{})};
 const html=renderToStaticMarkup(createElement(CollectionScreen,{locale:'fr',collection,view:{...initialCollectionView,selectedId:999001},update:()=>{}}));
 expect(html).toContain(`cosmetic://localhost/${id}/tile/999001`);expect(html).toContain(`cosmetic://localhost/${id}/splash/999001`);expect(html).toContain('Nouvelle série');expect(html).not.toContain('/latest/');expect(html).toContain('Possédé');
 const avatar=renderToStaticMarkup(createElement(AccountControl,{t:copy.fr,account:{platform:'EUW1',game_name:'Test',tag_line:'EUW',profile_icon_id:99999},status:'connected',phase:null,onProfile:()=>{},onSession:()=>{},onRetry:()=>{}}));
 expect(avatar).toContain(`cosmetic://localhost/${id}/profile/99999`);
});
