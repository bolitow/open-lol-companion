import {it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {BuildPatchNotice} from './BuildPatchNotice';
import {resolveBuildPatch} from './buildPatch';
it('signale le repli et le catalogue différent dans les deux langues',()=>{
 const choice=resolveBuildPatch({client:{patch:'16.20',gameVersion:'16.20.1'},clientError:null,manifest:{live_version:'16.20.1',versions:['16.20.1','16.19.1']},manifestError:null},'16.18');
 for(const locale of ['fr','en'] as const){const html=renderToStaticMarkup(<BuildPatchNotice choice={choice} actual="16.19" locale={locale}/>);expect(html).toContain('26.19');expect(html).toContain('26.18');expect(html).toContain(locale==='fr'?'imports suspendus':'imports paused');expect(html).toContain(locale==='fr'?'Patch antérieur':'Previous patch');}
});
it('conserve la provenance client fermé même avec un repli ancien',()=>{
 const choice=resolveBuildPatch({client:null,clientError:'unavailable',manifest:{live_version:'16.18.1',versions:['16.18.1']},manifestError:null},'16.19');
 const html=renderToStaticMarkup(<BuildPatchNotice choice={choice} actual="16.18" locale="fr"/>);
 expect(html).toContain('Client indisponible');expect(html).toContain('Patch antérieur');
});
