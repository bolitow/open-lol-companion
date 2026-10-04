import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {StatLabel,StatAmount} from './StatVisual';
import {abilitySegments} from './abilityPresentation';
it('distingue AD, AP, armure et résistance magique avec un visuel League et un libellé',()=>{
 for(const key of ['attack_damage','ability_power','armor','magic_resistance']){
  const html=renderToStaticMarkup(<><StatLabel statKey={key} locale="fr"/><StatAmount statKey={key}>42</StatAmount></>);
  expect(html).toContain('/game-data/stats/tooltip-stats-atlas.png');expect(html).toContain(`data-stat="${key}"`);expect(html).toContain('42');
 }
});
it('garde les statistiques inconnues lisibles sans inventer une icône',()=>{
 const html=renderToStaticMarkup(<StatLabel statKey="new_stat" locale="en"/>);
 expect(html).not.toContain('<img');expect(html).toContain('new_stat');
});
it('associe les coefficients écrits dans la source à leur stat sans confondre dégâts magiques et AP',()=>{
 const text='Inflige 80 dégâts magiques (+60 % AP) et 40 % de la puissance.';
 const parts=abilitySegments(text);
 expect(parts.map(p=>p.text).join('')).toBe(text);
 expect(parts.filter(p=>p.statKey).map(p=>[p.text,p.statKey])).toEqual([['60 % AP','ability_power'],['40 % de la puissance','ability_power']]);
 expect(abilitySegments('magic damage with 50% bonus attack damage').filter(p=>p.statKey).map(p=>p.statKey)).toEqual(['attack_damage']);
});
it('redimensionne l’icône et son atlas avec le même facteur, sans rogner le sprite',()=>{
 const html=renderToStaticMarkup(<StatLabel statKey="cooldown" locale="fr" size={14}/>);
 expect(html).toContain('--stat-size:14px');expect(html).toContain('background-size:1433.6px 179.2px');
});
