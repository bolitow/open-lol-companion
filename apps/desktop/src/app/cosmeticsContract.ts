/** Métadonnées publiques du même instantané ; les images restent chargées à la demande par Rust. */
export interface CosmeticsCatalog {
 schema_version:1;version:string;profile_icons:number[];
 skin_lines:{fr:Record<string,string>;en:Record<string,string>};
 skins:Record<string,{tile:string|null;splash:string|null}>;
}
const object=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
export const cosmeticId=(value:unknown):value is number=>typeof value==='number'&&Number.isInteger(value)&&value>=0&&value<=4294967295;
const idKey=(value:string)=>/^(0|[1-9][0-9]*)$/.test(value)&&cosmeticId(Number(value));
const asset=(value:unknown)=>value===null||typeof value==='string'&&value.length<=512&&/^assets\/(?:[a-z0-9_.-]+\/)*[a-z0-9_.-]+\.(?:png|jpe?g|webp)$/.test(value)&&!value.split('/').some(p=>p==='.'||p==='..');
export function parseCosmetics(value:unknown,version:string):CosmeticsCatalog|null {
 if(value===null)return null;
 if(!object(value)||value.schema_version!==1||value.version!==version||!Array.isArray(value.profile_icons)||value.profile_icons.length===0||value.profile_icons.length>50000||!value.profile_icons.every(cosmeticId)||new Set(value.profile_icons).size!==value.profile_icons.length||!object(value.skin_lines)||!object(value.skins)||Object.keys(value.skins).length===0||Object.keys(value.skins).length>20000||Object.keys(value.skin_lines).length!==2)throw Error('catalog-invalid');
 const {fr,en}=value.skin_lines;
 const names=(lines:unknown):lines is Record<string,string>=>object(lines)&&Object.keys(lines).length>0&&Object.keys(lines).length<=10000&&Object.entries(lines).every(([id,name])=>idKey(id)&&typeof name==='string'&&(id==='0'||name.length>0)&&new TextEncoder().encode(name).length<=1024&&!/[\u0000-\u001f\u007f-\u009f]/.test(name));
 if(!names(fr)||!names(en)||Object.keys(fr).length!==Object.keys(en).length||Object.keys(fr).some(id=>!Object.hasOwn(en,id)))throw Error('catalog-invalid');
 for(const [id,skin] of Object.entries(value.skins))if(!idKey(id)||Number(id)===0||!object(skin)||!asset(skin.tile)||!asset(skin.splash))throw Error('catalog-invalid');
 return value as unknown as CosmeticsCatalog;
}
