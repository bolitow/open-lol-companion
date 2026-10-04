import {expect,it} from 'vitest';
import type {CatalogRecord} from '@olc/shared';
import catalog from '../../public/game-data/catalog/en_US.json';
import {isImportableCategory,itemSetPlan,itemSetRequest} from './itemImport';
import {createImportController} from './runeImport';
const field=(value:unknown)=>({value,unit:null,status:'verified',sources:[]});
/** Objet minimal du catalogue : seuls les champs lus par la résolution sont renseignés. */
const item=(id:number,shop=true,extra:Record<string,unknown>={})=>({kind:'item',id:String(id),fields:{purchasable:field(shop),in_store:field(shop),...Object.fromEntries(Object.entries(extra).map(([k,v])=>[k,field(v)]))}} as unknown as CatalogRecord);
const records=[item(2003),item(3006),item(3004),item(3042,false,{special_recipe:3004})];
const real=(catalog as unknown as {records:CatalogRecord[]}).records;
it('importe uniquement la variante visible et conserve l’ordre et les consommables répétés',()=>{
 expect(itemSetRequest(103,'Ahri','Ordre des achats',[2003,3006,2003],records)).toEqual({championId:103,championName:'Ahri',mapId:11,blocks:[{label:'Ordre des achats',items:[{id:2003,count:1},{id:3006,count:1},{id:2003,count:1}]}]});
});
it('refuse une variante vide, inconnue ou mal formée sans envoyer un set partiel',()=>{
 for(const ids of [[],[2003,9999],[NaN],[0],[-1],[2003.5]])expect(itemSetRequest(103,'Ahri','Objets',ids,records)).toBeNull();
 expect(itemSetRequest(0,'Ahri','Objets',[2003],records)).toBeNull();
 expect(itemSetRequest(103,'','Objets',[2003],records)).toBeNull();
});
it('convertit les formes évoluées de Larme vers leur forme achetable avec le catalogue',()=>{
 expect(itemSetRequest(103,'Ahri','Inventaire final',[3042],records)?.blocks[0]?.items).toEqual([{id:3004,count:1}]);
 expect(itemSetRequest(103,'Ahri','Inventaire final',[3042,3040,3121],real)?.blocks[0]?.items).toEqual([{id:3004,count:1},{id:3003,count:1},{id:3119,count:1}]);
});
it('résout les objets non achetables du catalogue 16.19.1 vers leur ancêtre achetable',()=>{
 const plan=itemSetPlan(103,'Ahri','Inventaire final',[3866,3867,3176,2033,3070],real);
 expect(plan?.request.blocks[0]?.items).toEqual([{id:3865,count:1},{id:1001,count:1},{id:2031,count:1},{id:3070,count:1}]);
 expect(plan?.converted).toEqual([{from:3866,to:3865},{from:3867,to:3865},{from:3176,to:1001},{from:2033,to:2031}]);
 expect(plan?.dropped).toEqual([]);
});
it('retire sans rejeter la variante un objet sans ancêtre achetable unique, et le signale',()=>{
 const plan=itemSetPlan(103,'Ahri','Inventaire final',[3070,2422,3002,3006],real);
 expect(plan?.request.blocks[0]?.items).toEqual([{id:3070,count:1},{id:3006,count:1}]);
 expect(plan?.dropped).toEqual([2422,3002]);
 expect(plan?.converted).toEqual([]);
});
it('refuse la variante quand plus aucun objet achetable ne subsiste',()=>{
 expect(itemSetPlan(103,'Ahri','Inventaire final',[2422],real)).toBeNull();
 expect(itemSetRequest(103,'Ahri','Inventaire final',[2422,1501],real)).toBeNull();
});
/** Objet dont un champ de boutique est présent mais non vérifié (`unmapped`) : sa valeur n'est pas lisible. */
const unreadable=(id:number,which:'purchasable'|'in_store',value=true)=>{
 const other=which==='purchasable'?'in_store':'purchasable';
 return {kind:'item',id:String(id),fields:{[which]:{value,unit:null,status:'unmapped',sources:[]},[other]:field(true)}} as unknown as CatalogRecord;
};
it('ne convertit jamais un objet absent du catalogue ou sans champs lisibles',()=>{
 expect(itemSetRequest(103,'Ahri','Objets',[9999],records)).toBeNull();
 expect(itemSetRequest(103,'Ahri','Objets',[4000],[unreadable(4000,'purchasable')])).toBeNull();
 expect(itemSetRequest(103,'Ahri','Objets',[2003],[{kind:'item',id:'2003',fields:{}} as unknown as CatalogRecord])).toBeNull();
});
it('rejette la variante entière, sans retirer l’objet en silence, quand purchasable ou in_store est non vérifié',()=>{
 // Un objet achetable accompagne l'objet douteux : « retiré » donnerait un plan, « rejeté » donne null.
 for(const which of ['purchasable','in_store'] as const)for(const value of [true,false]){
  const mixed=[unreadable(4000,which,value),item(3070)];
  expect(itemSetPlan(103,'Ahri','Objets',[4000,3070],mixed)).toBeNull();
  expect(itemSetPlan(103,'Ahri','Objets',[3070,4000],mixed)).toBeNull();
 }
});
it('rejette la variante quand un maillon de la chaîne vers l’ancêtre est non vérifié, même si un ancêtre achetable existe plus loin',()=>{
 const broken=[item(3004),{kind:'item',id:'3042',fields:{purchasable:{value:false,unit:null,status:'unmapped',sources:[]},in_store:field(false),special_recipe:field(3004)}} as unknown as CatalogRecord,item(3070)];
 expect(itemSetPlan(103,'Ahri','Objets',[3042,3070],broken)).toBeNull();
 const middle=[item(1001),unreadable(3010,'in_store',false),item(3013,false,{builds_from:['3010']}),item(3070)];
 expect(itemSetPlan(103,'Ahri','Objets',[3013,3070],middle)).toBeNull();
});
it('distingue un false lisible (retiré ou converti) d’un champ illisible (variante rejetée)',()=>{
 const readableFalse=[item(4002,false),item(3070)];
 expect(itemSetPlan(103,'Ahri','Objets',[4002,3070],readableFalse)?.dropped).toEqual([4002]);
});
it('exige purchasable et in_store à la fois',()=>{
 const hidden={kind:'item',id:'4001',fields:{purchasable:field(true),in_store:field(false)}} as unknown as CatalogRecord;
 expect(itemSetRequest(103,'Ahri','Objets',[4001],[hidden])).toBeNull();
});
it('ne duplique pas un ancêtre issu de conversions mais conserve les répétitions réelles',()=>{
 const boots=[item(1001),item(3010,false,{builds_from:['1001']}),item(3013,false,{builds_from:['3010']}),item(2003)];
 expect(itemSetRequest(103,'Ahri','Ordre',[3010,3013,2003,2003],boots)?.blocks[0]?.items).toEqual([{id:1001,count:1},{id:2003,count:1},{id:2003,count:1}]);
});
it('ne boucle pas sur une chaîne cyclique du catalogue',()=>{
 const loop=[item(5001,false,{special_recipe:5002}),item(5002,false,{special_recipe:5001}),item(3070)];
 const plan=itemSetPlan(103,'Ahri','Objets',[5001,3070],loop);
 expect(plan?.dropped).toEqual([5001]);
 expect(plan?.request.blocks[0]?.items).toEqual([{id:3070,count:1}]);
});
it('conserve les erreurs stables des objets, sans afficher la réponse brute',async()=>{
 for(const error of ['invalidItems','itemSetPriorityUnavailable']){
  const controller=createImportController(async()=>{throw error});controller.activate('items');await controller.submit('items',{});
  expect(controller.getSnapshot().result).toMatchObject({status:'error',error});
 }
});
it('limite l’import aux catégories de parcours : ni objets fréquents ni relique seule',()=>{
 expect(['purchase_order','final_items'].every(isImportableCategory)).toBe(true);
 for(const category of ['item','trinket','runes','skill_order',''])expect(isImportableCategory(category)).toBe(false);
});
