import {expect,it,vi} from 'vitest';
import {createCatalogRevalidation} from './catalogRevalidation';
it('limite le retour au premier plan et ignore la minuterie masquée',()=>{
 let time=0,visible=true;const refresh=vi.fn();const policy=createCatalogRevalidation(refresh,()=>time,()=>visible);
 policy.force();time=299999;policy.focus();expect(refresh).toHaveBeenCalledTimes(1);
 time=300000;policy.focus();expect(refresh).toHaveBeenCalledTimes(2);
 visible=false;time=1800000;policy.periodic();expect(refresh).toHaveBeenCalledTimes(2);
 visible=true;policy.periodic();expect(refresh).toHaveBeenCalledTimes(3);policy.focus();expect(refresh).toHaveBeenCalledTimes(3);
});
