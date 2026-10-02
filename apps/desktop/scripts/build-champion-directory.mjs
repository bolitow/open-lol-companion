// Index léger dérivé des fiches officielles déjà normalisées par #61.
import {readFile,writeFile} from 'node:fs/promises';
const root=new URL('../public/game-data/',import.meta.url);
const index=JSON.parse(await readFile(new URL('champions.json',root),'utf8'));
const champions=[];let version;
for(const [id,names] of Object.entries(index)){
 const entry={id:Number(id),key:names.key,names:{fr:names.fr,en:names.en},titles:{},categories:[]};
 for(const [locale,language] of [['fr','fr_FR'],['en','en_US']]){
  const catalog=JSON.parse(await readFile(new URL(`catalog/champions/${id}/${language}.json`,root),'utf8'));
  version??=catalog.version;if(version!==catalog.version)throw new Error('Versions incompatibles');
  const record=catalog.records.find(r=>r.kind==='champion'&&r.id===id);
  if(!record)throw new Error(`Champion absent : ${id}`);
  const field=name=>['verified','derived','descriptive'].includes(record.fields[name]?.status)?record.fields[name].value:null;
  const categories=field('categories'),title=field('title');
  if(!Array.isArray(categories)||categories.some(c=>!['Assassin','Fighter','Mage','Marksman','Support','Tank'].includes(c))||typeof title!=='string')throw new Error(`Fiche invalide : ${id}`);
  entry.titles[locale]=title;entry.categories=categories;
 }
 champions.push(entry);
}
await writeFile(new URL('champion-directory.json',root),JSON.stringify({version,champions},null,2)+'\n');
console.log(`${champions.length} champions · ${version}`);
