import {expect,it} from 'vitest';
import type {CatalogRecord} from '@olc/shared';
import {itemSetRequest} from './itemImport';
import {createImportController} from './runeImport';
const records=[2003,3006,3042].map(id=>({kind:'item',id:String(id)} as CatalogRecord));
it('importe uniquement la variante visible et conserve l’ordre et les consommables répétés',()=>{
 expect(itemSetRequest(103,'Ahri','Ordre des achats',[2003,3006,2003],records)).toEqual({championId:103,championName:'Ahri',mapId:11,blocks:[{label:'Ordre des achats',items:[{id:2003,count:1},{id:3006,count:1},{id:2003,count:1}]}]});
});
it('refuse une variante vide, inconnue ou mal formée sans envoyer un set partiel',()=>{
 for(const ids of [[],[2003,9999],[NaN],[0],[-1],[2003.5]])expect(itemSetRequest(103,'Ahri','Objets',ids,records)).toBeNull();
 expect(itemSetRequest(0,'Ahri','Objets',[2003],records)).toBeNull();
 expect(itemSetRequest(103,'','Objets',[2003],records)).toBeNull();
});
it('conserve les formes évoluées de Larme pour la conversion par le moteur Rust',()=>{
 expect(itemSetRequest(103,'Ahri','Inventaire final',[3042],records)?.blocks[0]?.items).toEqual([{id:3042,count:1}]);
});
it('conserve les erreurs stables des objets, sans afficher la réponse brute',async()=>{
 for(const error of ['invalidItems','itemSetPriorityUnavailable']){
  const controller=createImportController(async()=>{throw error});controller.activate('items');await controller.submit('items',{});
  expect(controller.getSnapshot().result).toMatchObject({status:'error',error});
 }
});
