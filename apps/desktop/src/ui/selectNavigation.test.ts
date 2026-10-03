import {expect,it} from 'vitest';
import {selectIndex} from './selectNavigation';
it('navigue et boucle au clavier sans sortir des options',()=>{
 expect(selectIndex(2,'ArrowDown',3)).toBe(0);expect(selectIndex(0,'ArrowUp',3)).toBe(2);
 expect(selectIndex(1,'Home',3)).toBe(0);expect(selectIndex(1,'End',3)).toBe(2);
 expect(selectIndex(1,'Escape',3)).toBeNull();expect(selectIndex(0,'ArrowDown',0)).toBeNull();
});
it('saute les options interdites, même aux extrémités ou lorsque tout est interdit',()=>{
 expect(selectIndex(0,'ArrowDown',3,[1])).toBe(2);
 expect(selectIndex(2,'Home',3,[0])).toBe(1);
 expect(selectIndex(0,'End',3,[2])).toBe(1);
 expect(selectIndex(0,'ArrowUp',3,[2])).toBe(1);
 expect(selectIndex(0,'ArrowDown',2,[0,1])).toBeNull();
});

it('ouvre une valeur inconnue à la première ou dernière option selon la flèche',()=>{expect(selectIndex(-1,'ArrowDown',3)).toBe(0);expect(selectIndex(-1,'ArrowUp',3)).toBe(2)});
