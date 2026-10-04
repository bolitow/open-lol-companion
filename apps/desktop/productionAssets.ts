import {readdir,readFile} from 'node:fs/promises';
import {join} from 'node:path';

/** Le serveur dev garde public/prototype ; seul le paquet distribué l’exclut. */
export async function productionAssets(root:string,prefix=''):Promise<{fileName:string;source:Uint8Array}[]>{
 const files:{fileName:string;source:Uint8Array}[]=[];
 for(const entry of await readdir(join(root,prefix),{withFileTypes:true})){
  if(!prefix&&entry.name==='prototype')continue;
  const fileName=prefix?`${prefix}/${entry.name}`:entry.name;
  if(entry.isDirectory())files.push(...await productionAssets(root,fileName));
  else if(entry.isFile())files.push({fileName,source:await readFile(join(root,fileName))});
 }
 return files;
}
