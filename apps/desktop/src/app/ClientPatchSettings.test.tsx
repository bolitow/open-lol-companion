import {it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {ClientPatchView} from './ClientPatchSettings';
import {clientPatchCopy} from './clientPatch';
it('affiche le patch public et des erreurs distinctes dans les deux langues',()=>{
 for(const locale of ['fr','en'] as const){
  const state={native:true,pending:false,patch:'26.19',error:null};
  const html=renderToStaticMarkup(<ClientPatchView locale={locale} state={state} reload={()=>{}}/>);
  expect(html).toContain('26.19');expect(html).not.toContain('16.19');
  for(const error of ['unavailable','invalid_response'] as const){
   const failed=renderToStaticMarkup(<ClientPatchView locale={locale} state={{...state,patch:null,error}} reload={()=>{}}/>);
   expect(failed).toContain(clientPatchCopy[locale].errors[error]);
  }
 }
});
