export const spotlightKinds = ['passive','q','w','e','r','recall','emotes','attack','movement','death'] as const;
export interface SpotlightSegment {kind:typeof spotlightKinds[number]; start:number; end:number}
export interface SkinSpotlight {
 durationSeconds?:number;segments?:SpotlightSegment[];skinId:number;championId:number;name:string;videoId:string;title:string;channelUrl:string;publishedAt:string;checkedAt:string;source:string;
}
const object=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
const date=(value:unknown):value is string=>typeof value==='string'&&/^\d{4}-\d{2}-\d{2}$/.test(value)&&Number.isFinite(Date.parse(value))&&new Date(value).toISOString().slice(0,10)===value;
/** Pas de rapprochement par nom traduit : uniquement des associations vérifiées, uniques et indexées. */
export function parseSkinSpotlights(value:unknown):ReadonlyMap<number,SkinSpotlight>{
 if(!object(value)||value.schemaVersion!==1||!Array.isArray(value.entries)||value.entries.length>10000)throw new Error('skin-spotlights-invalid');
 const result=new Map<number,SkinSpotlight>();const videoIds=new Set<string>();
 for(const item of value.entries){
  if(!object(item)||!Number.isSafeInteger(item.skinId)||!Number.isSafeInteger(item.championId)||Number(item.championId)<=0||Math.floor(Number(item.skinId)/1000)!==item.championId||Number(item.skinId)%1000===0||result.has(Number(item.skinId))||typeof item.videoId!=='string'||!/^[-_a-zA-Z0-9]{11}$/.test(item.videoId)||item.channelUrl!=='https://www.youtube.com/@SkinSpotlights'||item.source!==`https://www.youtube.com/watch?v=${item.videoId}`||!date(item.publishedAt)||!date(item.checkedAt)||item.checkedAt<item.publishedAt||!['name','title'].every(key=>typeof item[key]==='string'&&String(item[key]).trim().length>0&&String(item[key]).length<300))throw new Error('skin-spotlights-invalid');
  if(videoIds.has(String(item.videoId)))throw new Error('skin-spotlights-invalid');
  videoIds.add(String(item.videoId));
  if(item.durationSeconds!==undefined&&(!Number.isSafeInteger(item.durationSeconds)||Number(item.durationSeconds)<=0||Number(item.durationSeconds)>7200))throw new Error('skin-spotlights-invalid');
  if(item.segments!==undefined){
   if(!Array.isArray(item.segments)||item.segments.length>spotlightKinds.length)throw new Error('skin-spotlights-invalid');
   const kinds=new Set<string>();let previousEnd=0;
   for(const segment of item.segments){
    if(!object(segment)||!spotlightKinds.includes(segment.kind as SpotlightSegment['kind'])||kinds.has(String(segment.kind))||!Number.isSafeInteger(segment.start)||!Number.isSafeInteger(segment.end)||Number(segment.start)<previousEnd||Number(segment.end)<=Number(segment.start)||Number(segment.end)>Number(item.durationSeconds??7200))throw new Error('skin-spotlights-invalid');
    kinds.add(String(segment.kind));previousEnd=Number(segment.end);
   }
  }
  result.set(Number(item.skinId),item as unknown as SkinSpotlight);
 }
 return result;
}
export function findSkinSpotlight(index:ReadonlyMap<number,SkinSpotlight>,skinId:number,championId:number):SkinSpotlight|null{
 const video=index.get(skinId);return video?.championId===championId?video:null;
}
let catalog:Promise<ReadonlyMap<number,SkinSpotlight>>|null=null;
export async function loadSkinSpotlight(skinId:number,championId:number):Promise<SkinSpotlight|null>{
 if(!catalog)catalog=(async()=>{
  const controller=new AbortController(),timeout=setTimeout(()=>controller.abort(),10000);
  try{const response=await fetch('/game-data/skin-spotlights.json',{signal:controller.signal});if(!response.ok)throw new Error('skin-spotlights-unavailable');return parseSkinSpotlights(await response.json());}
  finally{clearTimeout(timeout)}
 })().catch(error=>{catalog=null;throw error});
 return findSkinSpotlight(await catalog,skinId,championId);
}
