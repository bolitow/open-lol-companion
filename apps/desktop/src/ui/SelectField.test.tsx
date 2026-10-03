import {expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {SelectField} from './SelectField';
it('ne présente pas la première option comme sélectionnée si la valeur est inconnue',()=>{
 const html=renderToStaticMarkup(<SelectField label="Région" value="unknown" placeholder="Choisir" options={[{value:'EUW1',label:'Europe'}]} onChange={()=>{}}/>);
 expect(html).toContain('Choisir');expect(html).not.toContain('Europe');expect(html).toContain('aria-expanded="false"');
});
it('désactive un sélecteur vide ou dont toutes les options sont indisponibles',()=>{
 const html=renderToStaticMarkup(<SelectField label="Rune" value="a" options={[{value:'a',label:'Rune',disabled:true}]} onChange={()=>{}}/>);
 expect(html).toContain('disabled=""');expect(html).toContain('role="combobox"');
});
