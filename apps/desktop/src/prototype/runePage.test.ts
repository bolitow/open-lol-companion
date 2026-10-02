import {describe,it,expect} from "vitest";
import trees from "../../public/prototype/assets/rune-trees.json";
import {runePage} from "./runePage";
import type {DraftChampion} from "./draft";

describe("page de runes illustrative",()=>{
 it("sélectionne une rune par rangée principale et deux rangées secondaires distinctes",()=>{
  for(const champion of ["Ahri","Lux","Orianna"] as DraftChampion[])for(const stage of [0,2,4]){
   const page=runePage(champion,stage);
   const primary=trees.fr.find(t=>t.id===page.primary)!;
   const secondary=trees.fr.find(t=>t.id===page.secondary)!;
   expect(primary.id).not.toBe(secondary.id);
   expect(page.selected).toHaveLength(6);
   for(const row of primary.rows)expect(row.filter(r=>page.selected.includes(r.id))).toHaveLength(1);
   expect(secondary.rows[0]!.some(r=>page.selected.includes(r.id))).toBe(false);
   expect(secondary.rows.slice(1).map(row=>row.filter(r=>page.selected.includes(r.id)).length).sort()).toEqual([0,1,1]);
  }
 });
 it("suit le matchup du scénario et garde le champion consulté",()=>{
  expect(runePage("Ahri",0).selected).toContain(8112);
  expect(runePage("Ahri",2).selected).toContain(8230);
  expect(runePage("Lux",2).selected).toContain(8214);
  expect(runePage("Orianna",2).selected).toContain(8229);
 });
 it("garde les mêmes rangées et identifiants en français et anglais",()=>{
  expect(trees.fr.map(t=>t.rows.map(row=>row.map(r=>r.id)))).toEqual(trees.en.map(t=>t.rows.map(row=>row.map(r=>r.id))));
 });
});
