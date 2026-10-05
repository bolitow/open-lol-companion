import type {SpotlightMediaState} from '../../../../../packages/shared/src/spotlight';
/** Une réponse de lecture initiale ne doit pas écraser un événement plus récent. */
export function acceptMediaState(current:SpotlightMediaState,next:SpotlightMediaState):SpotlightMediaState {
 if(next.attempt!==current.attempt)return next.attempt>current.attempt?next:current;
 if(current.status==='loaded'||current.status==='failed')return current;
 if(current.status==='slow'&&next.status==='loading')return current;
 return next;
}
