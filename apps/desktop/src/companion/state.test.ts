import {describe,it,expect} from 'vitest';
import {CompanionState,poseFor,WelcomeSession} from './state';
describe('compagnon intégré',()=>{
 it('accueille puis retrouve le repos après une interaction complète',()=>{const c=new CompanionState();c.advance(4);expect(c.phase).toBe('idle');c.interact();c.advance(2);expect(c.phase).toBe('evil');c.advance(5);expect(c.phase).toBe('idle');});
 it('ignore les clics répétés pendant le retournement',()=>{const c=new CompanionState();c.skip();c.interact();c.advance(.1);c.interact();expect(c.elapsed).toBe(.1);});
 it('ne consomme pas une animation pendant que le composant est masqué',()=>{const c=new CompanionState();c.advance(10,false);expect(c.elapsed).toBe(0);});
 it('supprime rotation et feu mouvant avec la préférence réduite',()=>{const c=new CompanionState();c.setMotion(false);c.interact();expect(c.phase).toBe('evil');const a=poseFor(c);expect(a.energy).toBe(0);expect(a.clip).toBe('IdleEvil');c.advance(6);expect(poseFor(c).energy).toBe(0);expect(poseFor(c).clip).toBe('IdleFriendly');});
 it('maintient une transition progressive de couleur et revient au calme',()=>{const c=new CompanionState();c.skip();c.interact();c.advance(1);expect(poseFor(c).heat).toBeGreaterThan(0);expect(poseFor(c).heat).toBeLessThan(1);c.advance(10);expect(poseFor(c).heat).toBe(0);});
});

describe('rythme et accueil de session',()=>{
 it('ne rejoue le salut que dans une nouvelle session',()=>{
  const session=new WelcomeSession();
  expect(session.claim()).toBe(true);
  expect(session.claim()).toBe(false);
  expect(new WelcomeSession().claim()).toBe(true);
 });
 it('anticipe avant la rotation et garde une couleur continue',()=>{
  const c=new CompanionState();c.skip();c.interact();
  expect(c.phase).toBe('anticipate');
  c.advance(.12);expect(poseFor(c).squash).toBeLessThan(1);
  expect(poseFor(c).heat).toBe(0);
  c.advance(.12);expect(c.phase).toBe('turn');
  expect(poseFor(c).heat).toBe(0);
  c.advance(1.25);expect(c.phase).toBe('evil');
  expect(poseFor(c).heat).toBe(1);
 });
});
