import {expect,it} from 'vitest';
import type {CatalogRecord} from '@olc/shared';
import {abilitySequence,adjacentAbility} from './abilityDetailModel';
const record=(id:string,namespace='standard',locale='fr_FR')=>({id,kind:'ability',namespace,locale} as CatalogRecord);
it('parcourt uniquement les compétences du champion courant dans l’ordre P A Z E R',()=>{
 const q=record('103:Q'),p=record('103:passive'),w=record('103:W');
 const records=[w,record('99:Q'),record('103:R','classic'),q,p,record('103:E','standard','en_US'),q];
 expect(abilitySequence(q,records)).toEqual([p,q,w]);
 expect(adjacentAbility(q,records,1)).toBe(w);expect(adjacentAbility(q,records,-1)).toBe(p);
 expect(adjacentAbility(w,records,1)).toBe(p);expect(adjacentAbility(p,records,-1)).toBe(w);
});
it('ne propose pas de navigation pour une fiche isolée ou un objet',()=>{
 const q=record('103:Q');expect(adjacentAbility(q,[q],1)).toBeNull();
 expect(abilitySequence({...q,kind:'item'},[q])).toEqual([]);
});
