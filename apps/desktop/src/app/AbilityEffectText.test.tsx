import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {AbilityEffectText} from './AbilityEffects';
import type {AbilityEffectEntry} from './abilityEffectCatalog';
it('colore les montants selon les dégâts annoncés et conserve les couleurs des ratios',()=>{
 for(const tooltip of ['{{ first }} pts de dégâts magiques et {{ second }} pts de dégâts bruts ; {{ third }} pts de dégâts physiques','{{ first }} magic damage and {{ second }} true damage; {{ third }} physical damage']){
  const formula={terms:[{values:[35,60]},{values:[.5],stat:'ability_power'}]};
  const entry={tooltip,formulas:{first:formula,second:formula,third:formula},unresolved:[]} as unknown as AbilityEffectEntry;
  const html=renderToStaticMarkup(<AbilityEffectText entry={entry} locale="fr"/>);
  expect(html).toContain('<span class="ability-magic-damage"><strong class="ability-formula"');
  expect(html).toContain('<span class="ability-true-damage"><strong class="ability-formula"');
  expect(html).toContain('<span class="ability-damage"><strong class="ability-formula"');
  expect(html.match(/data-stat="ability_power"/g)).toHaveLength(3);
 }
});
it('retire les jetons d’icônes Riot sans supprimer les pourcentages utiles',()=>{
 const entry={tooltip:'Gain +{{ speed }}% %i:scaleAS%%i:scaleAP% ; 50% permanent.',formulas:{speed:{terms:[{values:[25]}]}},unresolved:[]} as unknown as AbilityEffectEntry;
 const html=renderToStaticMarkup(<AbilityEffectText entry={entry} locale="fr"/>);
 expect(html).not.toContain('%i:');expect(html).toContain('25');expect(html).toContain('50');expect(html).toContain('%');
});
it('distingue les ratios AP/AD et signale une variable inconnue sans zéro ni nom technique',()=>{
 const entry={tooltip:'{{ damage }} damage; {{ missing }} seconds',formulas:{damage:{terms:[{values:[20,45]},{values:[1.3],stat:'attack_damage'},{values:[.4],stat:'ability_power'}]}},unresolved:['missing']} as unknown as AbilityEffectEntry;
 const html=renderToStaticMarkup(<AbilityEffectText entry={entry} locale="en"/>);
 expect(html).toContain('data-stat="ability_power"');expect(html).toContain('data-stat="attack_damage"');
 expect(html).toContain('130%');expect(html).toContain('40%');expect(html).toContain('20 / 45');expect(html).toContain('Value unavailable');expect(html).not.toContain('{{');
});
