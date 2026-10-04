import {expect,it} from 'vitest';
import {formatStat} from './catalogFormat';
it('convertit un ratio en pourcentage sans confondre avec une valeur déjà en pourcent',()=>{
 expect(formatStat(.3,'ratio','en')).toBe('30%');
 expect(formatStat(30,'percent','en')).toBe('30%');
 expect(formatStat(15,'points','fr')).toBe('15');
});
it('conserve la période des régénérations et masque une unité inconnue',()=>{
 expect(formatStat(1.5,'points_per_second','en')).toBe('1.5 / s');
 expect(formatStat(10,'points_per_5_seconds','fr')).toBe('10 / 5 s');
 expect(formatStat(15,null,'fr')).toBeNull();
 expect(formatStat(NaN,'points','fr')).toBeNull();
});
it('conserve la précision de la vitesse d’attaque et reconnaît les unités de portée',()=>{
 expect(formatStat(.668,'attacks_per_second','fr')).toBe('0,668 / s');
 expect(formatStat(550,'game_units','en')).toBe('550');
});
