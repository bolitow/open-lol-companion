import {expect,it} from 'vitest';
import {nextProfileTab} from './championKeyboard';
it('parcourt les onglets avec les flèches et revient aux extrémités',()=>{
 expect(nextProfileTab('abilities','ArrowRight')).toBe('builds');
 expect(nextProfileTab('abilities','ArrowLeft')).toBe('catalog');
 expect(nextProfileTab('catalog','ArrowRight')).toBe('abilities');
 expect(nextProfileTab('builds','Home')).toBe('abilities');
 expect(nextProfileTab('abilities','End')).toBe('catalog');
 expect(nextProfileTab('abilities','Tab')).toBeNull();
});
