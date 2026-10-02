import type {Locale} from './state';
export function formatStat(value:number,unit:string|null,locale:Locale):string|null{
 if(!Number.isFinite(value))return null;
 if(unit==='ratio'||unit==='percent')return new Intl.NumberFormat(locale,{style:'percent',maximumFractionDigits:2}).format(unit==='ratio'?value:value/100);
 const formatted=new Intl.NumberFormat(locale,{maximumFractionDigits:2}).format(value);
 if(unit==='points'||unit==='game_units')return formatted;
 if(unit==='attacks_per_second')return `${new Intl.NumberFormat(locale,{maximumFractionDigits:3}).format(value)} / s`;
 if(unit==='points_per_second')return `${formatted} / s`;
 if(unit==='points_per_5_seconds')return `${formatted} / 5 s`;
 return null;
}
