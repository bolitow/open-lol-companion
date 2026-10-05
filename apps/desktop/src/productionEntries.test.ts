import {expect,it} from 'vitest';
import config from '../vite.config';
it('conserve les entrées app et overlay sans empaqueter la démo',()=>{
 expect(config.build?.rollupOptions?.input).toEqual({app:'index.html',overlay:'overlay.html'});
});

import {mkdtemp,mkdir,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {productionAssets} from '../productionAssets';
it('exclut tous les assets propres à la démo et conserve les assets réels binaires',async()=>{
 const root=await mkdtemp(join(tmpdir(),'olc-production-'));
 try{
  await mkdir(join(root,'prototype/layouts'),{recursive:true});await mkdir(join(root,'fonts'));
  await writeFile(join(root,'prototype/layouts/demo.png'),'demo');
  await writeFile(join(root,'fonts/font.ttf'),Buffer.from([0,255,17]));
  const files=await productionAssets(root);
  expect(files.map(file=>file.fileName)).toEqual(['fonts/font.ttf']);
  expect([...files[0]!.source]).toEqual([0,255,17]);
 }finally{await rm(root,{recursive:true,force:true})}
});
