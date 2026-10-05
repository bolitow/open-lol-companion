/** Les révisions tardives et les écoutes résolues après démontage ne relancent rien. */
export function connectPatchPublications(listen:(receive:(value:{revision:number})=>void)=>Promise<()=>void>,initial:()=>Promise<{revision:number}>,refresh:()=>void){
 let disposed=false,revision=-1,stop:(()=>void)|undefined;
 const changed=(value:{revision:number})=>{if(!disposed&&value.revision>revision){revision=value.revision;refresh()}};
 void listen(changed).then(unlisten=>{if(disposed)unlisten();else{stop=unlisten;void initial().then(changed).catch(()=>{})}}).catch(()=>{});
 return()=>{disposed=true;stop?.()};
}
