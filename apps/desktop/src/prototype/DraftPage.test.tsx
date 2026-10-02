import {it,expect} from "vitest";
import {renderToStaticMarkup} from "react-dom/server";
import {DraftPage} from "./DraftPage";
import {draftReducer,initialDraft,type DraftState} from "./draft";

it("ne présente plus la carte vide comme un prépick après son ban",()=>{
 const render=(state:DraftState)=>renderToStaticMarkup(<DraftPage state={state} locale="fr" dispatch={()=>{}} onHome={()=>{}} playing={false} onPlaying={()=>{}}/>);
 const prepick=draftReducer(initialDraft,{type:"prepick",champion:"Lux"});
 expect(render(prepick)).toContain("Vous · Votre prépick");
 const banned=draftReducer(prepick,{type:"next"});
 expect(banned.prepick).toBeNull();
 expect(render(banned)).not.toContain("Vous · Votre prépick");
});
