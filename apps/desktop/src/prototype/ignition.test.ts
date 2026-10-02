import { describe, expect, it } from "vitest";
import { openingState, gutterPoints, surfaceState, renderSize } from "./ignition";

describe("combustion et traces", () => {
  it("décale la seconde vague puis arrête complètement le mouvement", () => {
    expect(openingState(-100)).toEqual({reveal:0,follow:0,opacity:0,done:false});
    expect(openingState(160).reveal).toBeGreaterThan(0);
    expect(openingState(160).follow).toBe(0);
    expect(openingState(700).reveal).toBeGreaterThan(openingState(700).follow);
    expect(openingState(1500).reveal).toBeCloseTo(.8);
    expect(openingState(300).follow).toBe(0);
    expect(openingState(2200).done).toBe(false);
    expect(openingState(2200).opacity).toBeGreaterThan(0);
    expect(openingState(2750)).toEqual({reveal:1,follow:1,opacity:0,done:true});
  });
  it("fait traverser la surface après la révélation puis éteint son reflet", () => {
    expect(surfaceState(0)).toEqual({progress:0,opacity:0});
    expect(surfaceState(700).opacity).toBe(0);
    expect(surfaceState(1600).progress).toBeGreaterThan(.4);
    expect(surfaceState(1600).opacity).toBe(1);
    expect(surfaceState(2400).opacity).toBeLessThan(1);
    expect(surfaceState(2750)).toEqual({progress:1,opacity:0});
    expect(surfaceState(60000).opacity).toBe(0);
  });
  it("garde les chemins des interstices hors du contenu", () => {
    const points=gutterPoints({x:20,y:40,width:400,height:250});
    expect(points.every(p=>p.x<20||p.x>420||p.y<40||p.y>290)).toBe(true);
    expect(points.every(p=>Number.isFinite(p.x)&&Number.isFinite(p.y))).toBe(true);
  });
  it.each([[1440,900,2],[390,844,3],[5120,2160,2]])("borne le coût GPU sans déformer %s × %s", (w,h,dpr) => {
    const size=renderSize(w,h,dpr);
    expect(size.width*size.height).toBeLessThanOrEqual(1_100_000);
    expect(size.width/size.height).toBeCloseTo(w/h,2);
    expect(size.width).toBeLessThanOrEqual(w*dpr);
  });
});
