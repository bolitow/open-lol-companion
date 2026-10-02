export function pageFlameState(elapsed:number,phase:boolean){
 const progress=Math.max(0,Math.min(1,elapsed/(phase?1100:800)));
 return {progress,opacity:progress>=1?0:Math.min(1,progress*8,(1-progress)*6),done:progress>=1};
}
