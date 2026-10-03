import type {FlashSlot,DesktopSettingKey,Role} from '@olc/shared';
import {parsePreferences,type Preferences,type Locale} from './state';
import {parseFlashSlot} from './spellEditing';
import {parseAutoImportPreferences,type AutoImportPreferences} from './autoImport';
export type AutoImportSettingKey='autoRunes'|'autoItems'|'autoSpells'|'autoMinGames'|'autoCustomRole';
export type SettingValues=Preferences&{flashSlot:FlashSlot|null;autoRunes:boolean;autoItems:boolean;autoSpells:boolean;autoMinGames:number;autoCustomRole:Role|undefined};
export const autoImportStorageKey='olc.auto-import.preferences.v1';
export function autoImportPreferences(values:SettingValues):AutoImportPreferences {
 return {runes:values.autoRunes,items:values.autoItems,spells:values.autoSpells,minGames:values.autoMinGames,...(values.autoCustomRole?{customRole:values.autoCustomRole}:{})};
}
export function isAutoImportSetting(key:SettingsSearchKey):key is AutoImportSettingKey {
 return key==='autoRunes'||key==='autoItems'||key==='autoSpells'||key==='autoMinGames'||key==='autoCustomRole';
}
export type SettingKey=keyof SettingValues;
export type SettingsSearchKey=SettingKey|DesktopSettingKey|'overlay';
export type SettingCategory='all'|'app'|'league';
export interface SettingsStorage {getItem:(key:string)=>string|null;setItem:(key:string,value:string)=>void}
export interface SettingsSnapshot {values:SettingValues;recent:SettingKey[];lastChange:{key:SettingKey;previous:SettingValues[SettingKey]}|null;storageFailed:boolean;flashStorageFailed:boolean}
const appKey='olc.app.preferences',flashKey='olc.flash-slot';

/** Une seule source en mémoire pour l’en-tête, les réglages et la préparation. Aucun appel au jeu. */
export function createSettingsStore(storage:SettingsStorage,onChange=()=>{}){
 const failed=new Set<string>(),listeners=new Set<()=>void>();
 const read=(key:string)=>{try{return storage.getItem(key)}catch{failed.add(key);return null}};
 const imports=parseAutoImportPreferences(read(autoImportStorageKey),import.meta.env.DEV);
 let state:SettingsSnapshot={values:{...parsePreferences(read(appKey)),flashSlot:parseFlashSlot(read(flashKey)),autoRunes:imports.runes,autoItems:imports.items,autoSpells:imports.spells,autoMinGames:imports.minGames,autoCustomRole:imports.customRole},recent:[],lastChange:null,storageFailed:failed.size>0,flashStorageFailed:failed.has(flashKey)};
 const publish=(patch:Partial<SettingsSnapshot>)=>{state={...state,...patch};listeners.forEach(listener=>listener())};
 const save=(key:string,values:SettingValues)=>{
  try{
   const {theme,locale,motion}=values;
   storage.setItem(key,JSON.stringify(key===appKey?{theme,locale,motion}:key===autoImportStorageKey?autoImportPreferences(values):values.flashSlot));failed.delete(key);
  }catch{failed.add(key)}
 };
 const apply=(key:SettingKey,value:SettingValues[SettingKey],undo=false)=>{
  if(state.values[key]===value)return;
  if(key==='autoMinGames'&&(typeof value!=='number'||!Number.isInteger(value)||value<1||value>1000))return;
  const values={...state.values,[key]:value};
  save(key==='flashSlot'?flashKey:isAutoImportSetting(key)?autoImportStorageKey:appKey,values);
  publish({values,storageFailed:failed.size>0,flashStorageFailed:failed.has(flashKey),lastChange:undo?null:{key,previous:state.values[key]},recent:[key,...state.recent.filter(id=>id!==key)]});
  onChange();
 };
 return {
  getSnapshot:()=>state,
  subscribe:(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener)}},
  change:<K extends SettingKey>(key:K,value:SettingValues[K])=>apply(key,value),
  undo:()=>{if(state.lastChange)apply(state.lastChange.key,state.lastChange.previous,true)},
  clearUndo:()=>{if(state.lastChange)publish({lastChange:null})},
  retrySave:()=>{save(appKey,state.values);save(flashKey,state.values);save(autoImportStorageKey,state.values);publish({storageFailed:failed.size>0,flashStorageFailed:failed.has(flashKey)})},
 };
}
export type SettingsStore=ReturnType<typeof createSettingsStore>;
export const settingDefinitions:readonly {key:SettingsSearchKey;category:Exclude<SettingCategory,'all'>;terms:string}[]=[
 {key:'theme',category:'app',terms:'theme themes apparence appearance couleur couleurs color colors mode sombre dark nuit night clair light luminosite brightness'},
 {key:'locale',category:'app',terms:'langue langues language languages anglais english francais french traduction translation'},
 {key:'motion',category:'app',terms:'animations animation effets effects mouvement mouvements motion transition transitions moins less fewer reduce reduire ralentir desactiver disable couper arreter stop'},
 {key:'flashSlot',category:'league',terms:'flash saut eclair sort sorts spell spells summoner invocateur touche touches key keys position emplacement slot d f'},
 {key:'closeToTray',category:'app',terms:'fermer fermeture fenetre close closing window tray barre systeme arriere plan background garder actif'},
 {key:'autostartEnabled',category:'app',terms:'lancement demarrage automatique ouvrir connexion session ordinateur startup start launch login autostart automatically computer'},
 {key:'autoRunes',category:'league',terms:'imports importer import runes rune page pages automatique automatiques automatic auto prepick preselection activation activer desactiver disable enable'},
 {key:'autoItems',category:'league',terms:'imports importer import objets objet items item equipement equipment build builds automatique automatiques automatic auto prepick preselection activation activer desactiver disable enable'},
 {key:'autoSpells',category:'league',terms:'imports importer import sorts sort summoner spells spell invocateur automatique automatiques automatic auto prepick preselection activation activer desactiver disable enable'},
 {key:'autoMinGames',category:'league',terms:'imports importer import minimum min seuil threshold parties games echantillon sample statistiques statistics variantes variants'},
 {key:'overlay',category:'league',terms:'overlay partie en jeu game affichage display panneau panel ecran screen monitor position emplacement taille size largeur width opacite opacity transparence transparency raccourci shortcut apercu preview'},
 {key:'autoCustomRole',category:'league',terms:'imports importer import poste role lane personnalisee personnalise custom game games partie top jungle mid support bot'},
];
const normalize=(value:string)=>value.normalize('NFD').replace(/\p{M}/gu,'').toLowerCase().replace(/[^a-z0-9]+/g,' ').trim();
const stopWords=new Set('je j veux voudrais mon ma mes les le la l de d des du un une sur en pour avec mettre changer the my i want to on in a an of with change'.split(' '));
/** Tous les termes significatifs doivent correspondre ; une recherche ne change aucune préférence. */
export function searchSettings(query:string,_locale:Locale,category:SettingCategory):SettingsSearchKey[]{
 const words=normalize(query).split(' ').filter(word=>word&&!stopWords.has(word));
 return settingDefinitions.filter(definition=>(category==='all'||definition.category===category)&&words.every(word=>normalize(definition.terms).split(' ').some(term=>term.startsWith(word)))).map(definition=>definition.key);
}
