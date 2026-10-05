import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import type {CatalogRecord} from '@olc/shared';
import {ChampionAbilities} from './ChampionAbilities';
const ability={kind:'ability',id:'103:Q',name:'Orbe',description:'Inflige des dégâts magiques.',fields:{cooldown:{value:[7,7,7,7,7],status:'verified',unit:'seconds',sources:[]}},stats:{},effects:[],coverage:{issues:[]}} as unknown as CatalogRecord;
const champion={...ability,kind:'champion',id:'103',stats:{health:{value:590,status:'verified',unit:'points',sources:[]},attack_speed:{value:.668,status:'verified',unit:'attacks_per_second',sources:[]}}} as CatalogRecord;
it('conserve les métriques visibles et replie les descriptions et attributs généraux',()=>{
 const html=renderToStaticMarkup(<ChampionAbilities championId={103} records={[champion,ability]} locale="fr" version="16.19.1" onOpen={()=>{}}/>);
 expect(html).toContain('class="motion-disclosure champion-base-stats"');expect(html).toContain('aria-expanded="false"');expect(html).toContain('inert=""');
 expect(html).toContain('Statistiques de base');expect(html).toContain('590');expect(html).toContain('0,668 / s');expect(html).toContain('7 s');
 expect(html).toContain('class="champion-ability-heading"');expect(html).toContain('Chargement des effets');
 expect(html).toContain('Orbe · Démonstration du sort');expect(html).not.toContain('<video');expect(html).not.toContain('riotcdn.net');
});
it('traduit les libellés et touches sans inventer de valeurs sur le passif',()=>{
 const html=renderToStaticMarkup(<ChampionAbilities championId={103} records={[{...ability,id:'103:passive',fields:{}}]} locale="en" version="16.19.1" onOpen={()=>{}}/>);
 expect(html).toContain('Passive');expect(html).toContain('Loading effects');expect(html).not.toContain('ability-metrics');expect(html).not.toContain('champion-base-stats');
});
