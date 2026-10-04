import {catalogRuntime} from './catalogRuntime';
import {cosmeticId} from './cosmeticsContract';
import embeddedDirectory from '../../public/game-data/champion-directory.json';
import embeddedLines from './collection/skinLines.json';
function localAsset(kind:'profile'|'tile'|'splash',id:number){
 const {active}=catalogRuntime.getSnapshot();if(!active||!/^[a-f0-9]{64}$/.test(active.snapshotId))return null;
 const base=active.assetBase===`catalog://localhost/${active.snapshotId}/`?`cosmetic://localhost/${active.snapshotId}/`:active.assetBase===`http://catalog.localhost/${active.snapshotId}/`?`http://cosmetic.localhost/${active.snapshotId}/`:null;
 return base?`${base}${kind}/${id}`:null;
}
export function cosmeticProfileUrl(id:unknown){
 if(!cosmeticId(id))return null;
 const {cosmetics}=catalogRuntime.getSnapshot();
 return cosmetics?cosmetics.profile_icons.includes(id)?localAsset('profile',id):null:`https://ddragon.leagueoflegends.com/cdn/${embeddedDirectory.version}/img/profileicon/${id}.png`;
}
export function skinImageUrl(id:number,kind:'tile'|'splash',fallback:string|null){
 const {cosmetics}=catalogRuntime.getSnapshot();
 if(cosmetics)return cosmeticId(id)&&cosmetics.skins[String(id)]?.[kind]?localAsset(kind,id):null;
 // Les anciens paquets gardent les URL versionnées fournies par Rust, jamais latest.
 if(!fallback)return null;
 try{const url=new URL(fallback);return url.protocol==='https:'&&!url.username&&!url.password&&!url.pathname.split('/').includes('latest')?url.href:null}catch{return null}
}
export function skinLineEntries(){
 const {cosmetics}=catalogRuntime.getSnapshot();
 return cosmetics?Object.keys(cosmetics.skin_lines.fr).filter(id=>id!=='0').map(id=>({id:Number(id),names:{fr:cosmetics.skin_lines.fr[id]!,en:cosmetics.skin_lines.en[id]!}})):embeddedLines.entries;
}
export function validLocalSkinImage(value:string){
 // Comparaison exacte : ni query, ni port, ni chemin normalisé hors du catalogue actif.
 const match=/^(?:cosmetic:\/\/localhost|http:\/\/cosmetic\.localhost)\/[a-f0-9]{64}\/(tile|splash)\/(0|[1-9][0-9]*)$/.exec(value);
 return !!match&&skinImageUrl(Number(match[2]),match[1] as 'tile'|'splash',null)===value;
}
