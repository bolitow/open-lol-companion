import {describe,it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {AccountControl,accountStatus,profileIconUrl} from './AccountControl';
import {copy} from './copy';
describe('compte compact du shell',()=>{
 it('distingue client connecté, attente, recherche, erreur et aperçu navigateur',()=>{
  expect(accountStatus({native:true,error:false,revision:1,connected:true})).toBe('connected');
  expect(accountStatus({native:true,error:true,revision:1,connected:true})).toBe('error');
  expect(accountStatus({native:false,error:false,revision:-1,connected:false})).toBe('browser');
  expect(accountStatus({native:true,error:false,revision:-1,connected:false})).toBe('checking');
  expect(accountStatus({native:true,error:false,revision:2,connected:false})).toBe('waiting');
 });
 it('borne les identifiants utilisés pour l’image publique et accepte zéro',()=>{
  expect(profileIconUrl(0)).toMatch(/\/profileicon\/0.png$/);
  for(const id of [null,undefined,-1,1.2,'../../test',4294967296])expect(profileIconUrl(id)).toBeNull();
 });
 it('donne un nom accessible au compte et à son état dans les deux langues',()=>{
  for(const t of [copy.fr,copy.en]){
   const html=renderToStaticMarkup(<AccountControl t={t} account={{platform:'EUW1',game_name:'Alpha',tag_line:'TEST',profile_icon_id:42}} status="connected" phase={t.phases.None} onProfile={()=>{}} onSession={()=>{}} onRetry={()=>{}}/>);
   expect(html).toContain('Alpha#TEST');expect(html).toContain(t.connection.connected);
   expect(html).toContain('/profileicon/42.png');expect(html).toContain(t.account.profile);
   expect(html).toContain('aria-expanded="false"');
  }
 });
 it('sans compte, conserve un statut lisible sans inventer un avatar ni proposer un profil',()=>{
  const html=renderToStaticMarkup(<AccountControl t={copy.fr} account={null} status="waiting" phase={null} onProfile={()=>{}} onSession={()=>{}} onRetry={()=>{}}/>);
  expect(html).not.toContain('<img');expect(html).not.toContain(copy.fr.account.profile);expect(html).toContain(copy.fr.account.empty);
 });
});

it('ne présente pas le dernier compte comme actif pendant l’identification du client',()=>{
 const html=renderToStaticMarkup(<AccountControl t={copy.fr} account={{platform:'EUW1',game_name:'Old',tag_line:'TEST'}} accountIsActive={false} status="connected" phase={null} onProfile={()=>{}} onSession={()=>{}} onRetry={()=>{}}/>);
 expect(html).toContain(copy.fr.account.remembered);expect(html).not.toContain(copy.fr.account.active);
});
