import {expect,it} from 'vitest';
import {acceptMediaState} from './spotlightMedia';
it('ignore un état périmé après retry ou fermeture et un timeout après succès',()=>{
 expect(acceptMediaState({attempt:4,status:'loading'},{attempt:3,status:'loaded'})).toEqual({attempt:4,status:'loading'});
 expect(acceptMediaState({attempt:4,status:'loaded'},{attempt:4,status:'slow'})).toEqual({attempt:4,status:'loaded'});
 expect(acceptMediaState({attempt:4,status:'slow'},{attempt:4,status:'loaded'})).toEqual({attempt:4,status:'loaded'});
 expect(acceptMediaState({attempt:4,status:'loaded'},{attempt:5,status:'idle'})).toEqual({attempt:5,status:'idle'});
});
