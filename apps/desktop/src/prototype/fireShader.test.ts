import { describe, expect, it, vi } from "vitest";
import { createFireRenderer } from "./fireShader";

// Ces replis doivent laisser l’interface accessible, sans programme GPU orphelin.
describe("repli du moteur de combustion",()=>{
  it("accepte l’absence de WebGL",()=>{
    const canvas={getContext:()=>null} as unknown as HTMLCanvasElement;
    expect(createFireRenderer(canvas,false)).toBeNull();
  });
  it("refuse une précision insuffisante et libère le contexte",()=>{
    const loseContext=vi.fn();
    const canvas={getContext:()=>({getShaderPrecisionFormat:()=>({precision:0}),getExtension:()=>({loseContext})})} as unknown as HTMLCanvasElement;
    expect(createFireRenderer(canvas,false)).toBeNull();
    expect(loseContext).toHaveBeenCalledOnce();
  });
  it("libère les shaders si la compilation échoue",()=>{
    const deleteShader=vi.fn(),loseContext=vi.fn();
    const canvas={getContext:()=>({
      getShaderPrecisionFormat:()=>({precision:23}),getExtension:()=>({loseContext}),
      createShader:()=>({}),shaderSource:vi.fn(),compileShader:vi.fn(),getShaderParameter:()=>false,deleteShader,
    })} as unknown as HTMLCanvasElement;
    expect(createFireRenderer(canvas,true)).toBeNull();
    expect(deleteShader).toHaveBeenCalledTimes(2);
    expect(loseContext).toHaveBeenCalledOnce();
  });
});
