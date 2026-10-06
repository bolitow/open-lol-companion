import {expect,it} from 'vitest';
import type {CatalogRecord} from '@olc/shared';
import {parseFlashSlot,placeFlash,placeRecommendedSpells,chooseSpell,editSpellSelection,validSpellPair,spellsMatch,spellChoices} from './spellEditing';
const records=[1,4,7,14].map(id=>({kind:'summoner_spell',id:String(id),fields:{modes:{value:['CLASSIC']}}} as unknown as CatalogRecord));
it('n’invente pas une préférence D/F absente ou corrompue',()=>{
 expect(parseFlashSlot(null)).toBe(null);expect(parseFlashSlot('undefined')).toBe(null);expect(parseFlashSlot('"D"')).toBe('D');expect(parseFlashSlot('"F"')).toBe('F');expect(parseFlashSlot('{}')).toBe(null);
});
it('place Flash selon le choix explicite mais préserve une paire sans Flash',()=>{
 expect(placeFlash([14,4],'D')).toEqual([4,14]);expect(placeFlash([4,14],'F')).toEqual([14,4]);expect(placeFlash([7,14],'F')).toEqual([7,14]);expect(placeFlash([4,14],null)).toEqual([4,14]);
});
it('permute le sort déjà présent au lieu de créer un doublon',()=>{
 expect(chooseSpell([4,14],0,14)).toEqual([14,4]);expect(chooseSpell([4,14],1,7)).toEqual([4,7]);
});
it('oriente une recommandation sans Flash sur l’habitude locale valide, sans inventer de préférence',()=>{
 for(const [equipped,expected] of [
  [[14,4],[14,6]],[[4,14],[6,14]],[[14,6],[14,6]],[[6,14],[6,14]],
  [[7,21],[6,14]],[[0,14],[6,14]],[[14,14],[6,14]],[[14],[6,14]],
  [[14,4,7],[6,14]],[[14.5,4],[6,14]],[null,[6,14]],
 ] as const)expect(placeRecommendedSpells([6,14],equipped,'F')).toEqual(expected);
 expect(placeRecommendedSpells([14,6],null,null)).toEqual([14,6]);
 expect(placeRecommendedSpells([4,14],[4,14],'F')).toEqual([14,4]);
 expect(placeRecommendedSpells([14,4],[14,4],'D')).toEqual([4,14]);
});
it('refuse paire incomplète, dupliquée ou mode hors Faille',()=>{
 expect(validSpellPair([4,14],records)).toBe(true);
 for(const pair of [[4,4],[0,14],[4,32]])expect(validSpellPair(pair,records)).toBe(false);
 const aram={kind:'summoner_spell',id:'32',fields:{modes:{value:['ARAM']}}} as unknown as CatalogRecord;
 expect(spellChoices([...records,aram])).toEqual(records);
});
it('ne confirme que les deux emplacements remontés par le client',()=>{
 expect(spellsMatch([4,14],[4,14])).toBe(true);expect(spellsMatch([14,4],[4,14])).toBe(false);expect(spellsMatch(null,[4,14])).toBe(false);expect(spellsMatch([0,0],[0,0])).toBe(false);
});
it('changer seulement l’autre sort ne choisit pas implicitement la position de Flash',()=>{
 expect(editSpellSelection([4,14],1,21,null)).toEqual({pair:[4,21],flashSlot:null});
 expect(editSpellSelection([4,14],1,4,null)).toEqual({pair:[14,4],flashSlot:'F'});
 expect(editSpellSelection([4,14],0,14,'D')).toEqual({pair:[14,4],flashSlot:'F'});
});
