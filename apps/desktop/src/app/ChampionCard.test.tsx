import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {ChampionCard} from './ChampionCard';
import {copy} from './copy';
it('rend une carte connue consultable et distingue le pick local de la consultation',()=>{
 const html=renderToStaticMarkup(<ChampionCard player={{cellId:0,championId:103,local:true,locked:false,acting:true,position:'middle'}} locale="fr" t={copy.fr} selected onSelect={()=>{}}/>);
 expect(html).toMatch(/aria-label="[^"]*Ahri[^"]*Présélection[^"]*Vous[^"]*Mid/);expect(html).toContain('<button');expect(html).toContain('aria-pressed="true"');expect(html).toContain('Ahri');expect(html).toContain('Présélection');expect(html).toContain('Vous');expect(html).toContain('Consulter la préparation');
});
it('ne rend pas un emplacement vide interactif et traduit le verrouillage',()=>{
 const empty=renderToStaticMarkup(<ChampionCard locale="en" t={copy.en} selected={false} onSelect={()=>{}}/>);
 expect(empty).not.toContain('<button');
 const locked=renderToStaticMarkup(<ChampionCard player={{cellId:1,championId:222,local:false,locked:true,acting:false,position:null}} locale="en" t={copy.en} selected={false} onSelect={()=>{}}/>);
 expect(locked).toMatch(/aria-label="[^"]*Jinx[^"]*Locked/);expect(locked).toContain('Locked');expect(locked).toContain('Browse preparation');
});
