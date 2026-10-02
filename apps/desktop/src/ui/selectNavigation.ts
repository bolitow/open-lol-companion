export function selectIndex(index:number,key:string,length:number,disabled:readonly number[]=[]):number|null {
 if(!length||!['Home','End','ArrowDown','ArrowUp'].includes(key))return null;
 const step=key==='End'||key==='ArrowUp'?-1:1;
 const start=key==='Home'?0:key==='End'?length-1:index<0?(step>0?0:length-1):(index+step+length)%length;
 for(let offset=0;offset<length;offset++){
  const candidate=(start+offset*step+length)%length;
  if(!disabled.includes(candidate))return candidate;
 }
 return null;
}
