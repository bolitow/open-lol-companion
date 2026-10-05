import type {CSSProperties,ReactNode} from 'react';
import type {Locale} from './state';
import {statLabels} from './preparationCopy';
import atlas from './statIcons.json';
import './statVisual.css';
const aliases:Record<string,string>={range:'attack_range',movement_speed_ratio:'movement_speed',health_regen_percent:'health_regeneration',mana_regen_percent:'mana_regeneration'};
function statKeyFor(key:string){return aliases[key]??key}
export function StatGlyph({statKey,size=16}:{statKey:string;size?:14|16|18}){
 const icon=(atlas.icons as Record<string,{x:number;y:number}>)[statKeyFor(statKey)];
 if(!icon)return null;
 const scale=size/atlas.iconSize;
 return <span aria-hidden="true" className="stat-glyph" style={{'--stat-size':`${size}px`,backgroundImage:`url(${atlas.atlas})`,backgroundSize:`${atlas.width*scale}px ${atlas.height*scale}px`,backgroundPosition:`${-icon.x*scale}px ${-icon.y*scale}px`} as CSSProperties}/>;
}
export function StatLabel({statKey,locale,size=16}:{statKey:string;locale:Locale;size?:14|16|18}){
 return <span className="stat-label"><StatGlyph statKey={statKey} size={size}/>{statLabels[statKey]?.[locale]??statKey}</span>;
}
export function StatAmount({statKey,children}:{statKey:string;children:ReactNode}){
 return <strong className="stat-tone" data-stat={statKeyFor(statKey)}>{children}</strong>;
}
export function StatMention({statKey,children}:{statKey:string;children:ReactNode}){
 return <strong className="stat-tone stat-mention" data-stat={statKeyFor(statKey)}><StatGlyph statKey={statKey} size={14}/>{children}</strong>;
}
