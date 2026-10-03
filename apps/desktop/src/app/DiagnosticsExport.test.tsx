import {it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {DiagnosticsExport} from './DiagnosticsExport';
it('rend un bouton indisponible et expliqué dans le navigateur en FR et EN',()=>{
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<DiagnosticsExport locale={locale}/>);
  expect(html).toContain('disabled=""');expect(html).toContain('aria-describedby=');expect(html).not.toContain('<dialog');
  expect(html).toContain(locale==='fr'?'Disponible dans l’application desktop.':'Available in the desktop application.');
 }
});
