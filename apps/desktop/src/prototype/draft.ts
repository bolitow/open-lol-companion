export type DraftChampion="Ahri"|"Lux"|"Orianna";
export interface DraftState {stage:number;side:"blue"|"red";prepick:DraftChampion|null;preview:DraftChampion}
export const initialDraft:DraftState={stage:0,side:"blue",prepick:"Ahri",preview:"Ahri"};
export type DraftAction={type:"next"|"reset"|"side"|"return"|"lock"}|{type:"preview"|"prepick";champion:DraftChampion};
export function suggestions(state:DraftState):DraftChampion[]{
  // Ordre scénarisé pour juger l’interface, sans modèle de recommandation réel.
  return state.stage>=2?["Orianna","Ahri"]:state.stage>=1?["Ahri","Orianna"]:["Ahri","Lux","Orianna"];
}
export function draftReducer(state:DraftState,action:DraftAction):DraftState {
  if(action.type==="reset")return {...initialDraft,side:state.side};
  if(action.type==="side")return {...state,side:state.side==="blue"?"red":"blue"};
  if(action.type==="next"){
    if(state.stage===3&&!state.prepick)return state;
    const stage=Math.min(6,state.stage+1);
    const banned=stage>=1&&state.prepick==="Lux";
    return {...state,stage,prepick:banned?null:state.prepick,preview:stage===4?state.prepick!:(stage>=1&&state.preview==="Lux"?"Ahri":state.preview)};
  }
  if(state.stage>=4)return state;
  if(action.type==="lock")return state.stage===3&&state.prepick?{...state,stage:4,preview:state.prepick}:state;
  if(action.type==="return")return state.prepick?{...state,preview:state.prepick}:state;
  if(!("champion" in action))return state;
  if(!suggestions(state).includes(action.champion))return state;
  return {...state,preview:action.champion,...(action.type==="prepick"?{prepick:action.champion}:{})};
}
export function teams(state:DraftState){
  const ally=["Ornn","LeeSin",state.stage>=4?state.prepick:null,"Jinx","Thresh"];
  const enemy=["Renekton","Viego",state.stage>=2?"Yasuo":null,"Ezreal","Leona"];
  return state.side==="blue"?{blue:ally,red:enemy}:{blue:enemy,red:ally};
}
