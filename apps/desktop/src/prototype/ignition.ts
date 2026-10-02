// Un même ralenti garde synchronisés la révélation, les turbulences et les braises.
export const IGNITION_TIME_SCALE = .8;
export const IGNITION_DURATION = 2200 / IGNITION_TIME_SCALE;
const clamp = (value: number) => Math.max(0, Math.min(1, value));
export type Point = { x: number; y: number };
type Rect = {x:number;y:number;width:number;height:number};

/** La seconde vague reste derrière la combustion puis laisse place aux traces fixes. */
export function openingState(elapsed: number) {
  const time = elapsed * IGNITION_TIME_SCALE;
  return {
    reveal: clamp(time / 1500),
    follow: clamp((time - 260) / 1550),
    opacity: clamp((time - 260) / 200) * (1 - clamp((time - 1750) / 450)),
    done: elapsed >= IGNITION_DURATION,
  };
}

/** Limite le travail du fragment shader, y compris sur écran Retina/ultralarge. */
export function renderSize(width:number,height:number,dpr:number) {
  const scale=Math.min(dpr,1.25,Math.sqrt(1_100_000/(width*height)));
  return {width:Math.max(1,Math.floor(width*scale)),height:Math.max(1,Math.floor(height*scale))};
}

/** Irrégularités limitées à la gouttière : aucune flamme sur le texte des panneaux. */
export function gutterPoints(rect: Rect): Point[] {
  const x=rect.x-5,y=rect.y-5,w=rect.width+10,h=rect.height+10;
  const points:Point[]=[];
  const corners=[{x,y},{x:x+w,y},{x:x+w,y:y+h},{x,y:y+h},{x,y}];
  for(let side=0;side<4;side++) {
    const a=corners[side]!,b=corners[side+1]!;
    const count=Math.ceil(Math.hypot(b.x-a.x,b.y-a.y)/4);
    for(let i=0;i<count;i++) {
      const rough=(Math.sin(i*2.3+side)*1.3+Math.sin(i*.67)*.7)*Math.sin(Math.PI*i/count);
      points.push({x:a.x+(b.x-a.x)*i/count+(side%2?rough:0),y:a.y+(b.y-a.y)*i/count+(side%2?0:rough)});
    }
  }
  return [...points,points[0]!];
}

/** Le reflet de surface reste visible après le passage du voile au premier plan. */
export function surfaceState(elapsed:number) {
  const time=elapsed*IGNITION_TIME_SCALE;
  return {progress:clamp((time-600)/1400),opacity:clamp((time-600)/260)*(1-clamp((time-1700)/400))};
}
