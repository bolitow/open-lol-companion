import {describe,it,expect} from "vitest";
import {initialDraft,draftReducer,suggestions,teams} from "./draft";

describe("parcours de draft simulé",()=>{
  it("actualise les suggestions après ban et pick sans recommander un champion indisponible",()=>{
    let state=initialDraft;
    expect(suggestions(state)).toContain("Lux");
    state=draftReducer(state,{type:"next"});
    expect(suggestions(state)).not.toContain("Lux");
    const before=suggestions(state);
    state=draftReducer(state,{type:"next"});
    expect(suggestions(state)).not.toEqual(before);
    expect(teams(state).red).toContain("Yasuo");
  });
  it("consulte un champion sans changer le prépick et revient à celui-ci",()=>{
    const state=draftReducer(initialDraft,{type:"preview",champion:"Orianna"});
    expect(state.prepick).toBe("Ahri");expect(state.preview).toBe("Orianna");
    expect(draftReducer(state,{type:"return"}).preview).toBe("Ahri");
    expect(draftReducer({...state,prepick:null},{type:"return"}).preview).toBe("Orianna");
  });
  it("empêche de prépick un champion banni et suit le verrouillage jusque dans la partie",()=>{
    let state=draftReducer(initialDraft,{type:"next"});
    state=draftReducer(state,{type:"prepick",champion:"Lux"});expect(state.prepick).toBe("Ahri");
    state=draftReducer(state,{type:"prepick",champion:"Orianna"});
    state=draftReducer(state,{type:"next"});
    state=draftReducer(state,{type:"next"});
    state=draftReducer(state,{type:"lock"});
    expect(state.stage).toBe(4);expect(state.preview).toBe("Orianna");
    expect(draftReducer(state,{type:"preview",champion:"Ahri"}).preview).toBe("Orianna");
    state=draftReducer(state,{type:"next"});expect(state.stage).toBe(5);
    state=draftReducer(state,{type:"next"});expect(state.stage).toBe(6);
    expect(draftReducer(state,{type:"next"}).stage).toBe(6);
  });
  it("ne verrouille rien sans prépick et inverse les équipes avec le côté du joueur",()=>{
    expect(draftReducer({...initialDraft,prepick:null},{type:"lock"}).stage).toBe(0);
    const red=draftReducer(initialDraft,{type:"side"});
    expect(red.side).toBe("red");expect(teams(red).red).toEqual(teams(initialDraft).blue);
  });
  it("refuse un verrouillage avant la fin des bans et ne perd jamais un champion verrouillé",()=>{
    let state=draftReducer(initialDraft,{type:"prepick",champion:"Lux"});
    expect(draftReducer(state,{type:"lock"}).stage).toBe(0);
    for(let i=0;i<3;i++)state=draftReducer(state,{type:"next"});
    expect(state.prepick).toBeNull();
    state=draftReducer(state,{type:"prepick",champion:"Ahri"});
    state=draftReducer(state,{type:"lock"});
    for(let i=0;i<4;i++)state=draftReducer(state,{type:"next"});
    expect(state.prepick).toBe("Ahri");expect(state.stage).toBe(6);
  });
  it("efface un prépick et son aperçu devenus bannis",()=>{
    const state=draftReducer({...initialDraft,prepick:"Lux",preview:"Lux"},{type:"next"});
    expect(state.prepick).toBeNull();expect(state.preview).toBe("Ahri");
  });
});
