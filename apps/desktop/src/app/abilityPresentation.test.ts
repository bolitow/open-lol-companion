import {expect,it} from 'vitest';
import type {CatalogRecord,CatalogValue} from '@olc/shared';
import {abilityDamageTone,abilityMetrics,abilitySegments} from './abilityPresentation';
const field=(value:unknown,status='verified',unit='seconds')=>({value,status,unit,sources:[]} as CatalogValue);
const record=(fields:CatalogRecord['fields'])=>({kind:'ability',fields} as CatalogRecord);
it('compacte les constantes, conserve chaque rang et localise les décimales',()=>{
 expect(abilityMetrics(record({cooldown:field([12,11.5,11]),cost:field([60,60,60],'verified','resource_points')}),'fr').map(m=>m.value)).toEqual(['12 / 11,5 / 11 s','60']);
});
it('ignore les valeurs ambiguës, unités incorrectes et tableaux invalides',()=>{
 expect(abilityMetrics(record({cooldown:field([1],'conflict'),cost:field([1,-2],'verified','resource_points'),range:field([550],'verified','seconds')}),'en')).toEqual([]);
 expect(abilityMetrics(record({cooldown:field([]),cost:field(['{{ cost }}'],'verified','resource_points')}),'en')).toEqual([]);
});
it('ne présente pas le coût numérique comme un coût total et signale un supplément non résolu',()=>{
 const metrics=abilityMetrics(record({cost:field([40,45],'verified','resource_points'),resource:field('{{ percenthealthcost*100 }}% HP, {{ cost }} Mana','descriptive',null as never)}),'en');
 expect(metrics[0]?.label).toBe('Mana');expect(metrics[0]?.note).toBe('Additional resource change not quantified');
});
it('colore dégâts, soins et boucliers sans modifier le texte ni créer de chiffres',()=>{
 const text='Inflige 80 dégâts magiques, soigne 30 PV et donne un bouclier.';
 const segments=abilitySegments(text);expect(segments.map(s=>s.text).join('')).toBe(text);
 expect(segments.filter(s=>s.tone).map(s=>s.tone)).toEqual(['value','magic-damage','heal','value','shield']);
 expect(abilitySegments('deals damage and heals with a shield').filter(s=>s.tone).map(s=>s.tone)).toEqual(['damage','heal','shield']);
});
it('distingue les types de dégâts FR/EN sans les déduire des ratios AD ou AP',()=>{
 for(const text of ['dégâts bruts, dégâts magiques, dégâts physiques','true damage, magic damage, physical damage']){
  const segments=abilitySegments(text);expect(segments.map(s=>s.text).join('')).toBe(text);
  expect(segments.filter(s=>s.tone).map(s=>s.tone)).toEqual(['true-damage','magic-damage','damage']);
 }
 expect(abilitySegments('130% AD et 40% AP').filter(s=>s.statKey).map(s=>s.statKey)).toEqual(['attack_damage','ability_power']);
});
it('reconnaît magical damage et les dégâts en pourcentage de PV sans confondre soins ou réduction',()=>{
 expect(abilitySegments('magical damage').filter(s=>s.tone).map(s=>s.tone)).toEqual(['magic-damage']);
 expect(abilityDamageTone('% des PV max en dégâts magiques')).toBe('magic-damage');
 expect(abilityDamageTone('% max Health magic damage')).toBe('magic-damage');
 expect(abilityDamageTone('% de ses PV max en dégâts bruts')).toBe('true-damage');
 expect(abilityDamageTone('% max Health healing and physical damage')).toBeUndefined();
 expect(abilityDamageTone('% reduced physical damage')).toBeUndefined();
});
it('ne confond pas health ou une portion de PV sacrifiée avec un soin',()=>{
 expect(abilitySegments('sacrifices health and gains movement speed').filter(s=>s.tone)).toEqual([]);
 expect(abilitySegments('récupère des PV et rend immédiatement des PV').filter(s=>s.tone).map(s=>s.tone)).toEqual(['heal','heal']);
 expect(abilitySegments('recovers Health, restoring health').filter(s=>s.tone).map(s=>s.tone)).toEqual(['heal','heal']);
});
const champion=(id:string,resource:string)=>({kind:'champion',id,fields:{resource:field(resource,'descriptive',null as never)}} as unknown as CatalogRecord);
it('associe les coûts au mana ou à l’énergie du même champion, sans transformer une ressource inconnue en mana',()=>{
 const spell={...record({cost:field([55,65,75,85,95],'verified','resource_points'),resource:field('{{ cost }} {{ abilityresourcename }}','descriptive',null as never)}),id:'103:Q'};
 expect(abilityMetrics(spell,'fr',champion('103','Mana'))[0]).toMatchObject({statKey:'mana',label:'Mana',value:'55 / 65 / 75 / 85 / 95'});
 expect(abilityMetrics({...spell,id:'84:Q'},'en',champion('84','Energy'))[0]).toMatchObject({statKey:'energy',label:'Energy'});
 expect(abilityMetrics(spell,'fr',champion('84','Mana'))[0]?.statKey).toBe('cost');
 expect(abilityMetrics(spell,'fr')[0]?.statKey).toBe('cost');
});
it('identifie la portion mana d’un coût mixte sans oublier le supplément de PV non résolu',()=>{
 const spell=record({cost:field([40,45],'verified','resource_points'),resource:field('{{ percenthealthcost*100 }}% Max Health, {{ cost }} Mana','descriptive',null as never)});
 expect(abilityMetrics(spell,'en')[0]).toMatchObject({statKey:'mana',note:'Additional resource change not quantified'});
});
it('ne colore pas un sort gratuit ou une formule conflictuelle avec la ressource du champion',()=>{
 const spell={...record({cost:field([0,0],'verified','resource_points'),resource:field('No Cost','descriptive',null as never)}),id:'84:R'};
 expect(abilityMetrics(spell,'en',champion('84','Energy'))[0]?.statKey).toBe('cost');
 expect(abilityMetrics({...spell,fields:{...spell.fields,resource:field('{{ cost }} Mana','conflict',null as never)}},'en')[0]?.statKey).toBe('cost');
});
it('conserve la fréquence du coût et ne lui attribue pas celle d’un supplément distinct',()=>{
 const spell=(text:string)=>({...record({cost:field([13],'verified','resource_points'),resource:field(text,'descriptive',null as never)}),id:'27:Q'});
 expect(abilityMetrics(spell('{{ cost }} Mana per Second'),'en')[0]?.value).toBe('13 /s');
 expect(abilityMetrics(spell('{{ cost }} {{ abilityresourcename }} par sec'),'fr',champion('27','Mana'))[0]?.value).toBe('13 /s');
 expect(abilityMetrics(spell('{{ cost }} pts de mana par roquette'),'fr')[0]?.value).toBe('13 / roquette');
 expect(abilityMetrics(spell('{{ cost }} Mana Per Rocket'),'en')[0]?.value).toBe('13 / rocket');
 expect(abilityMetrics(spell('{{ cost }} Mana and {{ healthcost*100 }}% Current Health Per Second'),'en')[0]).toMatchObject({value:'13',note:'Additional resource change not quantified'});
});
it('ne présente pas une restauration non chiffrée comme un coût additionnel',()=>{
 const spell=record({cost:field([0],'verified','resource_points'),resource:field("Rend {{ energyrestore }} pts d’énergie",'descriptive',null as never)});
 expect(abilityMetrics(spell,'fr')[0]).toMatchObject({statKey:'cost',value:'0',note:'Autre variation de ressource non chiffrée'});
});
