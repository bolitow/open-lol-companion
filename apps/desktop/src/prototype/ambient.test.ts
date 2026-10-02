import {describe,it,expect,vi} from "vitest";
import {createEmberRenderer} from "./emberShader";
import {createAmbientLoop, emberSeeds, contactState, ambientSizes} from "./ambient";

describe("ambiance de braises",()=>{
  it("borne le nombre de particules et place les sources à l’extérieur des cartes",()=>{
    const rect={x:20,y:40,width:400,height:250};
    const seeds=emberSeeds([rect,rect,rect,rect]);
    expect(seeds.length).toBe(12*6);
    for(let i=0;i<seeds.length;i+=6){
      expect(seeds[i]).toBeGreaterThan(420);
      expect(seeds[i+1]).toBeGreaterThanOrEqual(40);
      expect(seeds[i+1]).toBeLessThanOrEqual(290);
    }
    expect([...seeds].every(Number.isFinite)).toBe(true);
    expect(emberSeeds([]).length).toBe(0);
  });
  it("place neuf étincelles au premier plan près des bords, avec des crépitements décalés",()=>{
    const rect={x:20,y:40,width:400,height:250};
    const seeds=emberSeeds([rect,rect,rect],"front");
    expect(seeds.length).toBe(9*6);
    expect([...seeds].every(Number.isFinite)).toBe(true);
    for(let i=0;i<seeds.length;i+=6){
      expect(seeds[i]).toBeGreaterThan(rect.x);
      expect(seeds[i]).toBeLessThan(rect.x+rect.width);
      expect(seeds[i+1]===rect.y+5||seeds[i+1]===rect.y+rect.height-5).toBe(true);
    }
    expect(contactState(150,0)).toBeGreaterThan(.5);
    expect(contactState(150,1)).toBe(0);
    expect(contactState(150,2)).toBe(0);
    expect(contactState(1100,0)).toBe(0);
    expect(contactState(6150,1)).toBeGreaterThan(.5);
  });
  it.each([[1440,900],[390,844],[5120,2160]])("partage un seul budget raster entre les deux profondeurs à %s×%s",(w,h)=>{
    const {back,front}=ambientSizes(w,h);
    expect(back.width*back.height+front.width*front.height).toBeLessThanOrEqual(1_100_000);
    expect(front.width).toBeGreaterThan(back.width);
  });
  it("limite les dessins, annule la boucle masquée et reprend sans rattrapage",()=>{
    let nextId=0;
    const pending=new Map<number,FrameRequestCallback>();
    const draw=vi.fn();
    const loop=createAmbientLoop(draw,cb=>{pending.set(++nextId,cb);return nextId;},id=>{pending.delete(id);});
    const tick=(now:number)=>{const entry=[...pending.entries()][0]!;pending.delete(entry[0]);entry[1](now);};
    loop.start();loop.start();expect(pending.size).toBe(1);
    tick(0);tick(10);tick(34);expect(draw).toHaveBeenCalledTimes(2);
    loop.stop();expect(pending.size).toBe(0);
    loop.start();tick(60000);expect(draw.mock.lastCall![0]).toBeLessThan(100);
    loop.stop();expect(pending.size).toBe(0);
  });
});

describe("repli des braises",()=>{
  it("omet les particules sans WebGL",()=>{
    expect(createEmberRenderer({getContext:()=>null} as unknown as HTMLCanvasElement,false)).toBeNull();
  });
  it("libère les ressources après échec de compilation",()=>{
    const deleteShader=vi.fn(),loseContext=vi.fn();
    const canvas={getContext:()=>({createShader:()=>({}),shaderSource:vi.fn(),compileShader:vi.fn(),getShaderParameter:()=>false,deleteShader,getExtension:()=>({loseContext})})} as unknown as HTMLCanvasElement;
    expect(createEmberRenderer(canvas,true)).toBeNull();
    expect(deleteShader).toHaveBeenCalledTimes(2);expect(loseContext).toHaveBeenCalledOnce();
  });
});

describe("sonde GPU bornée",()=>{
  it.each([false,true])("arrête les essais même si les résultats sont invalides (disponibles : %s)",available=>{
    const timer={TIME_ELAPSED_EXT:1,QUERY_RESULT_AVAILABLE_EXT:2,QUERY_RESULT_EXT:3,GPU_DISJOINT_EXT:4,
      createQueryEXT:()=>({}),deleteQueryEXT:vi.fn(),beginQueryEXT:vi.fn(),endQueryEXT:vi.fn(),
      getQueryObjectEXT:()=>available};
    const noop=()=>{};
    const gl={createShader:()=>({}),shaderSource:noop,compileShader:noop,getShaderParameter:()=>true,
      createProgram:()=>({}),createBuffer:()=>({}),attachShader:noop,linkProgram:noop,getProgramParameter:()=>true,
      useProgram:noop,bindBuffer:noop,getAttribLocation:()=>0,enableVertexAttribArray:noop,vertexAttribPointer:noop,
      getUniformLocation:()=>({}),uniform1f:noop,enable:noop,blendFunc:noop,clear:noop,drawArrays:noop,
      getParameter:()=>true,getExtension:(name:string)=>name==="EXT_disjoint_timer_query"?timer:null,
      deleteBuffer:noop,deleteProgram:noop,deleteShader:noop};
    const canvas={getContext:()=>gl,dataset:{} as Record<string,string>} as unknown as HTMLCanvasElement;
    const renderer=createEmberRenderer(canvas,false)!;
    for(let i=0;i<1000;i++)renderer.draw(i*34);
    expect(timer.beginQueryEXT.mock.calls.length).toBeLessThanOrEqual(6);
    expect(timer.deleteQueryEXT).toHaveBeenCalledOnce();
    expect(canvas.dataset.gpuTimer).toBe("inconclusive");
    renderer.dispose();expect(timer.deleteQueryEXT).toHaveBeenCalledOnce();
  });
});

describe("sphères de lumière",()=>{
  it("borne les sources et conserve des phases et profondeurs différentes",async()=>{
    const {orbSeeds}=await import("./ambient");
    const rect={x:20,y:40,width:400,height:250};
    const front=orbSeeds([rect,rect,rect],"front"),back=orbSeeds([rect,rect,rect],"back");
    expect(front.length).toBe(6*6);expect(back.length).toBe(6*6);
    expect([...front,...back].every(Number.isFinite)).toBe(true);
    expect(front[2]).not.toBe(back[2]);
    expect(front[5]).toBeGreaterThan(400);
    expect(orbSeeds([],"front").length).toBe(0);
  });
});
