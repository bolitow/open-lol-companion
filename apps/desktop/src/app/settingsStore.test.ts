import {describe,it,expect} from 'vitest';
import {createSettingsStore,searchSettings} from './settingsStore';
const memory=(initial:Record<string,string>={})=>{const data=new Map(Object.entries(initial));return {getItem:(key:string)=>data.get(key)??null,setItem:(key:string,value:string)=>{data.set(key,value)},data}};
describe('réglages partagés',()=>{
 it('reprend les clés existantes et garde Flash non défini sans choix explicite',()=>{
  const storage=memory({'olc.app.preferences':'{"theme":"light","locale":"en","motion":false}','olc.flash-slot':'"F"'});
  expect(createSettingsStore(storage).getSnapshot().values).toEqual({theme:'light',locale:'en',motion:false,flashSlot:'F'});
  expect(createSettingsStore(memory()).getSnapshot().values).toEqual({theme:'dark',locale:'fr',motion:true,flashSlot:null});
  expect(createSettingsStore(memory({'olc.app.preferences':'broken','olc.flash-slot':'"Z"'})).getSnapshot().values.flashSlot).toBeNull();
 });
 it('propage un changement aux consommateurs et le restaure au prochain lancement',()=>{
  const storage=memory(),store=createSettingsStore(storage),seen:string[]=[];
  const stop=store.subscribe(()=>seen.push(store.getSnapshot().values.flashSlot??'unset'));
  store.change('flashSlot','F');expect(seen).toEqual(['F']);
  expect(createSettingsStore(storage).getSnapshot().values.flashSlot).toBe('F');
  stop();store.change('flashSlot','D');expect(seen).toEqual(['F']);
  expect(store.getSnapshot().recent).toEqual(['flashSlot']);
 });
 it('annule seulement la dernière modification et persiste le retour y compris Flash non choisi',()=>{
  const storage=memory(),store=createSettingsStore(storage);
  store.change('theme','light');store.change('flashSlot','F');store.undo();
  expect(store.getSnapshot().values).toEqual({theme:'light',locale:'fr',motion:true,flashSlot:null});
  expect(createSettingsStore(storage).getSnapshot().values).toEqual(store.getSnapshot().values);
  expect(store.getSnapshot().lastChange).toBeNull();store.undo();expect(store.getSnapshot().values.theme).toBe('light');
 });
 it('un clic sur la valeur active ne remplace pas l’annulation',()=>{
  const store=createSettingsStore(memory());store.change('theme','light');const before=store.getSnapshot();store.change('theme','light');
  expect(store.getSnapshot()).toBe(before);store.undo();expect(store.getSnapshot().values.theme).toBe('dark');
 });
 it('garde une erreur de stockage tant que toutes les préférences non sauvées ne le sont pas',()=>{
  const storage=memory();let fail=true;
  const store=createSettingsStore({...storage,setItem:(key,value)=>{if(fail&&key==='olc.flash-slot')throw new Error('fixture');storage.setItem(key,value)}});
  store.change('flashSlot','F');store.change('theme','light');
  expect(store.getSnapshot().storageFailed).toBe(true);expect(store.getSnapshot().values.flashSlot).toBe('F');
  fail=false;store.retrySave();expect(store.getSnapshot().storageFailed).toBe(false);expect(createSettingsStore(storage).getSnapshot().values.flashSlot).toBe('F');
 });
 it('reste utilisable quand la lecture et l’écriture du stockage sont refusées',()=>{
  const store=createSettingsStore({getItem:()=>{throw new Error()},setItem:()=>{throw new Error()}});
  expect(store.getSnapshot().storageFailed).toBe(true);store.change('locale','en');store.undo();expect(store.getSnapshot().values.locale).toBe('fr');
 });
});
describe('recherche des réglages sans effet de bord',()=>{
 it('trouve des intentions FR/EN, accents et casse sans sélectionner une valeur',()=>{
  expect(searchSettings('moins d’animations','fr','all')).toEqual(['motion']);
  expect(searchSettings('RÉDUIRE les effets','fr','all')).toEqual(['motion']);
  expect(searchSettings('Flash sur F','fr','all')).toEqual(['flashSlot']);
  expect(searchSettings('dark mode','en','all')).toEqual(['theme']);
  expect(searchSettings('fewer animations','en','all')).toEqual(['motion']);
  expect(searchSettings('language','fr','all')).toEqual(['locale']);
 });
 it('combine les termes, garde les catégories et accepte un résultat vide',()=>{
  expect(searchSettings('flash couleur','fr','all')).toEqual([]);
  expect(searchSettings('micro','fr','all')).toEqual([]);
  expect(searchSettings('flash','fr','app')).toEqual([]);
  expect(searchSettings('','en','league')).toEqual(['flashSlot']);
  expect(searchSettings('','fr','all')).toEqual(['theme','locale','motion','flashSlot']);
 });
});
it('distingue une panne de sauvegarde Flash d’une panne des préférences de l’application',()=>{
 for(const failedKey of ['olc.app.preferences','olc.flash-slot']){
  const storage=memory();const store=createSettingsStore({...storage,setItem:(key,value)=>{if(key===failedKey)throw new Error();storage.setItem(key,value)}});
  store.change('theme','light');store.change('flashSlot','F');
  expect(store.getSnapshot().storageFailed).toBe(true);
  expect(store.getSnapshot().flashStorageFailed).toBe(failedKey==='olc.flash-slot');
 }
});
