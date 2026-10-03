import type {DraftPlayer} from '@olc/shared';

/** La consultation ne change jamais le champion annoncé dans le client. */
export function draftChampionContext(manual:number|null,local:DraftPlayer|undefined){
 const ownId=local?.championId||null;
 return {
  championId:manual??ownId??0,
  state:manual!==null?'browsing':ownId?(local?.locked?'locked':'prepick'):'empty',
  returnId:manual!==null?ownId:null,
  returnLocked:!!ownId&&!!local?.locked,
 } as const;
}

export function manualChampionSelection(championId:number,local:DraftPlayer|undefined):number|null{
 return !championId||championId===local?.championId?null:championId;
}
