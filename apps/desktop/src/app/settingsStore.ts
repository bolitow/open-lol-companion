import type {FlashSlot} from '@olc/shared';
import {parsePreferences,type Preferences,type Locale} from './state';
import {parseFlashSlot} from './spellEditing';
export type SettingValues=Preferences&{flashSlot:FlashSlot|null};
export type SettingKey=keyof SettingValues;
export type SettingCategory='all'|'app'|'league';
export interface SettingsStorage {getItem:(key:string)=>string|null;setItem:(key:string,value:string)=>void}
export interface SettingsSnapshot {values:SettingValues;recent:SettingKey[];lastChange:{key:SettingKey;previous:SettingValues[SettingKey]}|null;storageFailed:boolean;flashStorageFailed:boolean}
const appKey='olc.app.preferences',flashKey='olc.flash-slot';

/** Une seule source en mémoire pour l’en-tête, les réglages et la préparation. Aucun appel au jeu. */
export function createSettingsStore(storage:SettingsStorage){
 const failed=new Set<string>(),listeners=new Set<()=>void>();
 const read=(key:string)=>{try{return storage.getItem(key)}catch{failed.add(key);return null}};
 let state:SettingsSnapshot={values:{...parsePreferences(read(appKey)),flashSlot:parseFlashSlot(read(flashKey))},recent:[],lastChange:null,storageFailed:failed.size>0,flashStorageFailed:failed.has(flashKey)};
 const publish=(patch:Partial<SettingsSnapshot>)=>{state={...state,...patch};listeners.forEach(listener=>listener())};
 const save=(key:string,values:SettingValues)=>{
  try{
   const {theme,locale,motion}=values;
   storage.setItem(key,JSON.stringify(key===appKey?{theme,locale,motion}:values.flashSlot));failed.delete(key);
  }catch{failed.add(key)}
 };
 const apply=(key:SettingKey,value:SettingValues[SettingKey],undo=false)=>{
  if(state.values[key]===value)return;
  const values={...state.values,[key]:value};
  save(key==='flashSlot'?flashKey:appKey,values);
  publish({values,storageFailed:failed.size>0,flashStorageFailed:failed.has(flashKey),lastChange:undo?null:{key,previous:state.values[key]},recent:[key,...state.recent.filter(id=>id!==key)]});
 };
 return {
  getSnapshot:()=>state,
  subscribe:(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener)}},
  change:<K extends SettingKey>(key:K,value:SettingValues[K])=>apply(key,value),
  undo:()=>{if(state.lastChange)apply(state.lastChange.key,state.lastChange.previous,true)},
  retrySave:()=>{save(appKey,state.values);save(flashKey,state.values);publish({storageFailed:failed.size>0,flashStorageFailed:failed.has(flashKey)})},
 };
}
export type SettingsStore=ReturnType<typeof createSettingsStore>;
export const settingDefinitions:readonly {key:SettingKey;category:Exclude<SettingCategory,'all'>;terms:string}[]=[
 {key:'theme',category:'app',terms:'theme themes apparence appearance couleur couleurs color colors mode sombre dark nuit night clair light luminosite brightness'},
 {key:'locale',category:'app',terms:'langue langues language languages anglais english francais french traduction translation'},
 {key:'motion',category:'app',terms:'animations animation effets effects mouvement mouvements motion transition transitions moins less fewer reduce reduire ralentir desactiver disable couper arreter stop'},
 {key:'flashSlot',category:'league',terms:'flash saut eclair sort sorts spell spells summoner invocateur touche touches key keys position emplacement slot d f'},
];
const normalize=(value:string)=>value.normalize('NFD').replace(/\p{M}/gu,'').toLowerCase().replace(/[^a-z0-9]+/g,' ').trim();
const stopWords=new Set('je j veux voudrais mon ma mes les le la l de d des du un une sur en pour avec mettre changer the my i want to on in a an of with change'.split(' '));
/** Tous les termes significatifs doivent correspondre ; une recherche ne change aucune préférence. */
export function searchSettings(query:string,_locale:Locale,category:SettingCategory):SettingKey[]{
 const words=normalize(query).split(' ').filter(word=>word&&!stopWords.has(word));
 return settingDefinitions.filter(definition=>(category==='all'||definition.category===category)&&words.every(word=>normalize(definition.terms).split(' ').some(term=>term.startsWith(word)))).map(definition=>definition.key);
}
