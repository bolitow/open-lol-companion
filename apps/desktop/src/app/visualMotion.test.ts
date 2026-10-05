import {describe,it,expect} from 'vitest';
import {transitionKind,type Scene} from './visualMotion';
const home:Scene={screen:'dashboard',phase:null,connected:false};
describe('transitions de l’application',()=>{
 it('ne rejoue rien au premier rendu ni aux mises à jour de données',()=>{
  expect(transitionKind(null,home)).toBeNull();
  expect(transitionKind(home,{...home,phase:'Lobby',connected:true})).toBeNull();
 });
 it('distingue navigation manuelle et arrivée automatique en draft',()=>{
  expect(transitionKind(home,{...home,screen:'champ-select'})).toBe('page');
  expect(transitionKind(home,{screen:'champ-select',phase:'ChampSelect',connected:true})).toBe('phase');
 });
 it('ne confond pas un retour manuel vers une partie active avec un changement de phase',()=>{
  const before:Scene={...home,connected:true,phase:'ChampSelect'};
  expect(transitionKind(before,{...before,screen:'champ-select'})).toBe('page');
 });
 it('réagit à l’entrée en partie, sans flasher les réglages au changement de phase',()=>{
  const draft:Scene={screen:'champ-select',phase:'ChampSelect',connected:true};
  expect(transitionKind(draft,{screen:'in-game',phase:'GameStart',connected:true})).toBe('phase');
  expect(transitionKind({...draft,screen:'settings'},{screen:'settings',phase:'GameStart',connected:true})).toBeNull();
 });
 it('ne fait pas passer une déconnexion pour une arrivée en partie',()=>{
  expect(transitionKind({...home,phase:'ChampSelect',connected:true},{...home,screen:'champ-select'})).toBe('page');
 });
});
