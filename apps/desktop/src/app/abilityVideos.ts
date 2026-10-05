export interface AbilityVideo {
 page:string;poster:string|null;width:number;height:number;
 sources:{src:string;type:'video/mp4'|'video/webm'}[];
}
const object=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
const invalid=()=>new Error('ability-video-catalog-invalid');
export function parseAbilityVideos(value:unknown):Record<string,AbilityVideo>{
 if(!object(value)||value.schemaVersion!==1||typeof value.checkedAt!=='string'||!Number.isFinite(Date.parse(value.checkedAt))||!object(value.abilities))throw invalid();
 const result:Record<string,AbilityVideo>={};
 for(const [id,entry] of Object.entries(value.abilities)){
  const identity=/^([1-9]\d*):(passive|Q|W|E|R)$/.exec(id);
  if(!identity||!object(entry)||typeof entry.page!=='string'||!/^https:\/\/www\.leagueoflegends\.com\/en-us\/champions\/[a-z0-9-]+\/$/.test(entry.page)||typeof entry.width!=='number'||typeof entry.height!=='number'||![entry.width,entry.height].every(n=>Number.isFinite(n)&&n>0&&n<=8192)||!Array.isArray(entry.sources)||entry.sources.length>2)throw invalid();
  const champion=identity[1]!.padStart(4,'0'),slot=identity[2]==='passive'?'P':identity[2];
  const base=`https://lol.dyn.riotcdn.net/x/videos/champion-abilities/${champion}/ability_${champion}_${slot}1`;
  if(entry.poster!==null&&entry.poster!==`${base}.jpg`)throw invalid();
  for(const source of entry.sources){
   if(!object(source)||!['video/mp4','video/webm'].includes(String(source.type))||source.src!==`${base}.${source.type==='video/mp4'?'mp4':'webm'}`)throw invalid();
  }
  result[id]=entry as unknown as AbilityVideo;
 }
 return result;
}
let catalog:Promise<Record<string,AbilityVideo>>|null=null;
/** Catalogue partagé en mémoire ; aucune requête au CDN avant la consultation. */
export async function loadAbilityVideo(id:string):Promise<AbilityVideo|null>{
 if(!catalog){
  catalog=(async()=>{
   const controller=new AbortController(),timeout=setTimeout(()=>controller.abort(),10000);
   try{
    const response=await fetch('/game-data/ability-videos.json',{signal:controller.signal});
    if(!response.ok)throw new Error('ability-video-catalog-unavailable');
    return parseAbilityVideos(await response.json());
   }finally{clearTimeout(timeout)}
  })().catch(error=>{catalog=null;throw error});
 }
 return (await catalog)[id]??null;
}
