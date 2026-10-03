import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {DraftPlayer} from '@olc/shared';
import {BuildChampionContext} from './BuildChampionContext';
const local:DraftPlayer={cellId:1,championId:103,locked:false,local:true,position:'middle',acting:true};
const render=(manual:number|null,player:DraftPlayer|undefined=local,locale:'fr'|'en'='fr')=>renderToStaticMarkup(<BuildChampionContext manual={manual} local={player} locale={locale} onChange={()=>{}}/>);
it('rattache visuellement le statut à la sélection et réserve le retour à la consultation',()=>{
 const own=render(null);expect(own).toContain('Votre prépick');expect(own).toContain('Ahri');expect(own).not.toContain('class="return-pick"');
 const browsing=render(99);expect(browsing).toContain('Consultation');expect(browsing).toContain('Lux');expect(browsing).toContain('Suivre mon champion · Ahri');
 expect(render(99,{...local,championId:null})).not.toContain('class="return-pick"');
});
it('reflète le verrouillage et traduit le contexte',()=>{
 expect(render(null,{...local,locked:true})).toContain('Votre pick verrouillé');
 expect(render(null,{...local,locked:true},'en')).toContain('Your locked pick');
 expect(render(99,local,'en')).toContain('Follow my champion · Ahri');
});
it('garde le retour vers un prépick absent du catalogue local',()=>{
 const html=render(99,{...local,championId:999999});
 expect(html).toContain('class="return-pick"');
 expect(html).toContain('Suivre mon champion · Indisponible dans ce catalogue');
 expect(render(null,{...local,championId:999999})).toContain('Indisponible dans ce catalogue');
});
