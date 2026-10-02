type Rect={x:number;y:number;width:number;height:number};

export type EmberDepth="back"|"front";

/** Deux résolutions partagent le budget : le premier plan est plus net que le fond. */
export function ambientSizes(width:number,height:number) {
  const size=(share:number)=>{
    const scale=Math.min(share===.65?1:.7,Math.sqrt(1_100_000*share/(width*height)));
    return {width:Math.max(1,Math.floor(width*scale)),height:Math.max(1,Math.floor(height*scale))};
  };
  return {back:size(.35),front:size(.65)};
}

/** Sources sur les bords : douze braises derrière, neuf étincelles devant au maximum. */
export function emberSeeds(rects:Rect[],depth:EmberDepth="back") {
  const noise=(seed:number)=>{const value=Math.sin(seed*127.1)*43758.5453;return value-Math.floor(value);};
  return new Float32Array(rects.slice(0,3).flatMap((rect,group)=>Array.from({length:depth==="front"?3:4},(_,i)=>{
    const seed=group*17+i+1;
    if(depth==="front")return [rect.x+(group===1?rect.width*.72:rect.width-18),
      group===1?rect.y+rect.height-5:rect.y+5,
      (group/3-i*.014+1)%1,9,24+noise(seed)*10,(i-1)*.75+.2];
    return [rect.x+rect.width+7+noise(seed)*3,rect.y+rect.height*(.16+noise(seed+41)*.8),
      noise(seed+83),8+(i%3)*2,28+noise(seed+109)*12,group+i*.7];
  }).flat()));
}

/** Trois crépitements décalés, jamais un clignotement simultané de toute l’interface. */
export function contactState(elapsed:number,group:number) {
  const time=((elapsed/1000+group*3)%9+9)%9;
  if(time>1||time<.03)return 0;
  const envelope=Math.min(1,time/.12)*Math.max(0,1-time);
  return envelope*(.72+.28*Math.sin(time*27)**2);
}

/** Une seule RAF en attente, dessins à 30 Hz maximum, horloge gelée en pause. */
export function createAmbientLoop(draw:(elapsed:number)=>void,request:(cb:FrameRequestCallback)=>number,cancel:(id:number)=>void) {
  let id=0,running=false,last:number|null=null,elapsed=0;
  const tick=(now:number)=>{
    if(!running)return;
    if(last===null){last=now;draw(elapsed);}
    else if(now-last>=1000/30){elapsed+=Math.min(now-last,100);last=now;draw(elapsed);}
    if(running)id=request(tick);
  };
  return {
    start(){if(running)return;running=true;last=null;id=request(tick);},
    stop(){running=false;cancel(id);last=null;},
  };
}

/** Douze sphères au total, trajectoires lentes traversant la matière des panneaux. */
export function orbSeeds(rects:Rect[],depth:EmberDepth){
 return new Float32Array(rects.slice(0,3).flatMap((r,g)=>[0,1].flatMap(i=>[
   r.x-48,r.y+r.height*(i===0?.35:.78),
   (g*.23+i*.49+(depth==="front"?.17:.38))%1,
   [12,15,18][g]!,depth==="front"?48+i*18:82+i*18,
   10+r.width+96,
 ])));
}

/** Reflet de la sphère avant, dans les coordonnées de sa carte. */
export function orbSurfaceState(elapsed:number,width:number,height:number,group:number,index:number){
 const phase=(group*.23+index*.49+.17)%1;
 const age=(phase+elapsed/1000/[12,15,18][group]!)%1;
 const smooth=(a:number,b:number,x:number)=>{const v=Math.max(0,Math.min(1,(x-a)/(b-a)));return v*v*(3-2*v);};
 return {x:-48+age*(width+96),y:height*(index===0?.35:.78)+Math.sin(age*Math.PI*2+phase*9)*24-age*32,
 opacity:smooth(0,.18,age)*(1-smooth(.65,1,age))*(.55+.45*Math.sin(age*8+phase*6)**2)};
}
