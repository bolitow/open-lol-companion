import {expect,it} from 'vitest';
import {findChampions,championDirectory} from './championDirectory';
it('retrouve les noms localisés malgré accents, espaces et ponctuation',()=>{
 expect(findChampions('  KOG maw ','ALL','fr').map(c=>c.id)).toEqual([96]);
 expect(findChampions(' vel koz','ALL','en').map(c=>c.id)).toEqual([161]);
 expect(findChampions('Maitre Yi','ALL','fr').map(c=>c.id)).toEqual([11]);
 expect(findChampions('MonkeyKing','ALL','en').map(c=>c.id)).toEqual([62]);
});
it('combine recherche et classe sans confondre classe et poste',()=>{
 expect(findChampions('ahri','Mage','fr').map(c=>c.id)).toEqual([103]);
 expect(findChampions('ahri','Tank','fr')).toEqual([]);
 expect(findChampions('zzzz','ALL','fr')).toEqual([]);
 expect(findChampions('','MIDDLE','fr')).toEqual([]);
});
it('trie les cartes et remonte un nom exact dans les résultats',()=>{
 const list=findChampions('','ALL','en');
 expect(list.length).toBe(championDirectory.champions.length);
 expect(findChampions('','ALL','en',true).map(c=>c.id)).toEqual(list.map(c=>c.id).reverse());
 expect(findChampions('vi','ALL','fr')[0]?.id).toBe(254);
});
