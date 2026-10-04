import {readFile,writeFile,rename} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {resolveAbilityVariable} from './ability-calculations.mjs';

const slots=['Q','W','E','R'];
const lookup=(object,key)=>{
 const keys=Object.keys(object).filter(candidate=>candidate.toLowerCase()===key.toLowerCase());
 return keys.length===1?{key:keys[0],value:object[keys[0]]}:null;
};

export function compileAbility(record,bin,championKey){
 if(record?.kind!=='ability'||record.namespace!=='standard')return null;
 const slot=record.id.split(':')[1],index=slots.indexOf(slot),root=lookup(bin,`Characters/${championKey}/CharacterRecords/Root`)?.value;
 const name=root?.spellNames?.[index],technicalId=record.fields.technical_id?.value,tooltip=record.fields.tooltip?.value,ranks=record.fields.max_rank?.value;
 if(index<0||typeof name!=='string'||typeof technicalId!=='string'||typeof tooltip!=='string')return null;
 const spell=lookup(bin,`Characters/${championKey}/Spells/${name}`);
 if(spell?.value?.mScriptName?.toLowerCase()!==technicalId.toLowerCase())return null;
 const text=tooltip.replace(/\{\{\s*spellmodifierdescriptionappend\s*\}\}/gi,'').trim(),formulas={},unresolved=[];
 for(const [,raw] of text.matchAll(/\{\{\s*([^{}]+?)\s*\}\}/g)){
  const key=raw.trim().toLowerCase();
  if(Object.hasOwn(formulas,key)||unresolved.includes(key))continue;
  const formula=resolveAbilityVariable(spell.value.mSpell,key,ranks);
  if(formula)formulas[key]=formula;else unresolved.push(key);
 }
 return {technicalId,spellPath:spell.key,tooltip:text,formulas,unresolved};
}

async function main(){
 const app=resolve(dirname(fileURLToPath(import.meta.url)),'..'),catalog=resolve(app,'public/game-data/catalog/champions');
 const sourceDirectory=process.argv[2];
 if(!sourceDirectory)throw new Error('Indiquer le dossier des BIN et de index.json (sources versionnées avec SHA256).');
 const index=JSON.parse(await readFile(resolve(sourceDirectory,'index.json'),'utf8'));
 const roster=Object.keys(JSON.parse(await readFile(resolve(app,'public/game-data/champions.json'),'utf8'))).sort((a,b)=>Number(a)-Number(b));
 const abilities={},coverage={records:0,withFormulas:0,completeTooltips:0,unresolved:0,unmapped:[]};let version;
 for(const id of roster){
  const english=JSON.parse(await readFile(resolve(catalog,id,'en_US.json'),'utf8'));
  version??=english.version;if(version!==english.version)throw new Error('Versions de catalogue mélangées');
  const key=english.records.find(record=>record.kind==='champion')?.fields.technical_id?.value;
  if(typeof key!=='string'||!/^[a-z0-9]+$/i.test(key))throw new Error(`Champion invalide : ${id}`);
  const patch=version.split('.').slice(0,2).join('.'),url=`https://raw.communitydragon.org/${patch}/game/data/characters/${key.toLowerCase()}/${key.toLowerCase()}.bin.json`;
  const source=index.entries.find(entry=>String(entry.champion_id)===id);
  if(!source||source.url!==url||source.status!=='ok')throw new Error(`Source absente/incompatible : ${id}`);
  const raw=await readFile(resolve(sourceDirectory,`${key.toLowerCase()}.bin.json`)),sha256=createHash('sha256').update(raw).digest('hex');
  if(sha256!==source.sha256)throw new Error(`Empreinte incompatible : ${id}`);
  const bin=JSON.parse(raw);
  for(const locale of ['fr_FR','en_US']){
   const page=locale==='en_US'?english:JSON.parse(await readFile(resolve(catalog,id,`${locale}.json`),'utf8'));
   if(page.version!==version)throw new Error('Versions de langue mélangées');
   for(const record of page.records.filter(record=>record.kind==='ability')){
    const result=compileAbility(record,bin,key);coverage.records++;
    if(!result){coverage.unmapped.push(`${locale}:${record.id}`);continue}
    if(Object.keys(result.formulas).length)coverage.withFormulas++;
    if(!result.unresolved.length&&Object.keys(result.formulas).length)coverage.completeTooltips++;
    coverage.unresolved+=result.unresolved.length;
    abilities[`${locale}:${record.id}`]={...result,source:{url,sha256}};
   }
  }
 }
 const output=resolve(app,'public/game-data/ability-effects.json');
 await writeFile(`${output}.tmp`,JSON.stringify({schemaVersion:1,version,abilities,coverage})+'\n');
 await rename(`${output}.tmp`,output);
 console.log(JSON.stringify({version,...coverage,unmapped:coverage.unmapped.length}));
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url))main().catch(error=>{console.error(error.message);process.exitCode=1});
