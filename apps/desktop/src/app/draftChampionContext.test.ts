import {describe,it,expect} from 'vitest';
import type {DraftPlayer} from '@olc/shared';
import {draftChampionContext,manualChampionSelection} from './draftChampionContext';
const local:DraftPlayer={cellId:1,championId:103,locked:false,local:true,position:'middle',acting:true};
describe('contexte de préparation',()=>{
 it('ne crée aucun retour sans champion annoncé',()=>{
  expect(draftChampionContext(null,undefined)).toEqual({championId:0,state:'empty',returnId:null,returnLocked:false});
  expect(draftChampionContext(99,{...local,championId:null}).returnId).toBeNull();
 });
 it('suit le prépick puis le verrouillage reçu du client',()=>{
  expect(draftChampionContext(null,local).state).toBe('prepick');
  expect(draftChampionContext(null,{...local,locked:true}).state).toBe('locked');
  expect(draftChampionContext(null,{...local,championId:99}).championId).toBe(99);
 });
 it('garde le champion consulté pendant un changement de notre pick',()=>{
  expect(draftChampionContext(99,{...local,championId:22,locked:true})).toEqual({championId:99,state:'browsing',returnId:22,returnLocked:true});
 });
 it('retire le retour lorsque le client disparait sans effacer la consultation',()=>{
  expect(draftChampionContext(99,undefined)).toEqual({championId:99,state:'browsing',returnId:null,returnLocked:false});
 });
 it('reprend le suivi en choisissant notre champion ou le choix vide',()=>{
  expect(manualChampionSelection(103,local)).toBeNull();
  expect(manualChampionSelection(0,local)).toBeNull();
  expect(manualChampionSelection(99,local)).toBe(99);
  expect(manualChampionSelection(99,undefined)).toBe(99);
 });
});
