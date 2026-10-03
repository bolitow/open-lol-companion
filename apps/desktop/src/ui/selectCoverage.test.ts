import {expect,it} from 'vitest';

const sources=import.meta.glob('../**/*.tsx',{query:'?raw',import:'default',eager:true}) as Record<string,string>;
it('utilise le composant partagé pour chaque liste de sélection de l’application et du prototype',()=>{
 const nativeSelects=Object.entries(sources).filter(([path,source])=>!path.includes('.test.')&&!/ 2\./.test(path)&&/<select(?:\s|>)/.test(source)).map(([path])=>path);
 expect(nativeSelects).toEqual([]);
});
