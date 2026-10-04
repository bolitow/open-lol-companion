/** Retarder le démontage, avec annulation en cas de réouverture. */
export function scheduleDeparture(finish:()=>void,duration:number,reduced:boolean){
 const timer=setTimeout(finish,reduced?0:duration);
 return ()=>clearTimeout(timer);
}
