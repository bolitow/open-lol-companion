import {it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {ApiAccessSettings} from './ApiAccessSettings';
import {createApiAccessStore} from './apiAccess';
it('affiche le refus plutôt que configuré et propose la saisie sans exposer le jeton',async()=>{
 const store=createApiAccessStore({native:true,read:async()=>({source:'keychain',url:'https://example.com',error:null,authorizationRejected:true}),save:async()=>{throw 'unused'},clear:async()=>{throw 'unused'}});
 await store.load();
 for(const locale of ['fr','en'] as const){
  const html=renderToStaticMarkup(<ApiAccessSettings locale={locale} storeOverride={store}/>);
  expect(html).toContain(locale==='fr'?'Jeton refusé ou expiré':'Token rejected or expired');
  expect(html).not.toContain(locale==='fr'?'Configuré depuis':'Configured from');
  expect(html).toContain('type="password"');expect(html).toContain('value=""');
 }
});
