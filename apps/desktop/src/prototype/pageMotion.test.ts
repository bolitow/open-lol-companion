import {it,expect} from "vitest";
import {pageFlameState} from "./pageMotion";
it("borne la flamme de navigation et allonge le passage de phase",()=>{
 expect(pageFlameState(0,false).opacity).toBe(0);
 expect(pageFlameState(240,false).opacity).toBeGreaterThan(.5);
 expect(pageFlameState(800,false).done).toBe(true);
 expect(pageFlameState(800,true).done).toBe(false);
 expect(pageFlameState(1100,true)).toEqual({progress:1,opacity:0,done:true});
});
