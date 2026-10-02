import type {EmberDepth} from "./ambient";
type GpuTimer={
  TIME_ELAPSED_EXT:number;QUERY_RESULT_AVAILABLE_EXT:number;QUERY_RESULT_EXT:number;GPU_DISJOINT_EXT:number;
  createQueryEXT:()=>WebGLQuery|null;deleteQueryEXT:(q:WebGLQuery)=>void;
  beginQueryEXT:(target:number,q:WebGLQuery)=>void;endQueryEXT:(target:number)=>void;
  getQueryObjectEXT:(q:WebGLQuery,pname:number)=>number|boolean;
};
// Sprites procéduraux limités aux points : aucun shader plein écran en boucle.
const vertex=`
attribute vec2 a_origin;
attribute vec4 a_seed;
uniform vec2 u_view;
uniform float u_time;
uniform float u_scale;
uniform mediump float u_front;
varying float v_alpha;
varying float v_heat;
varying float v_orb;
void main(){
 float age=fract(a_seed.x+u_time/a_seed.y);
 float burst=age*a_seed.y;
 vec2 drift=vec2(sin(age*5.0+a_seed.w)*3.0,-age*38.0);
 vec2 spark=vec2(a_seed.w*burst*26.0,-burst*46.0+burst*burst*12.0);
 v_orb=step(10.0,a_seed.w);
 vec2 orbit=vec2(age*(a_seed.w-10.0),sin(age*6.283+a_seed.x*9.0)*24.0-age*32.0);
 vec2 p=a_origin+mix(mix(drift,spark,u_front),orbit,v_orb);
 gl_Position=vec4(p/u_view*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.0,1.0);
 gl_PointSize=a_seed.z*u_scale*mix(1.0,.82+.18*sin(age*6.283),v_orb);
 float calm=smoothstep(0.0,.18,age)*(1.0-smoothstep(.65,1.0,age));
 float flash=smoothstep(0.0,.05,burst)*(1.0-smoothstep(.35,.9,burst));
 v_alpha=mix(mix(calm,flash,u_front),calm*(.55+.45*sin(age*8.0+a_seed.x*6.0)*sin(age*8.0+a_seed.x*6.0)),v_orb);
 v_heat=.4+.6*sin(age*3.14159);
}`;
const fragment=`precision mediump float;
uniform float u_light;
uniform mediump float u_front;
varying float v_alpha;
varying float v_heat;
varying float v_orb;
void main(){
 vec2 p=(gl_PointCoord-.5)*2.0;
 float r=length(p);
 float core=exp(-dot(p*vec2(1.8,.6),p*vec2(1.8,.6))*95.0);
 float glow=exp(-r*r*mix(6.0,13.0,u_front))*mix(.18,.32,u_front);
 float sphere=exp(-r*r*45.0)*.78;
 float ring=exp(-pow((r-.25)*24.0,2.0))*.13;
 float aura=exp(-r*r*5.5)*.20;
 core=mix(core,sphere,v_orb);
 float alpha=mix(core*mix(.12,1.0,u_front)+glow,(sphere+ring+aura)*mix(.4,.9,u_front),v_orb)*v_alpha;
 vec3 ember=mix(vec3(1.0,.24,.045),vec3(.12,.40,.85),u_light);
 vec3 hot=mix(vec3(1.0,.83,.42),vec3(.3,.7,1.0),u_light);
 gl_FragColor=vec4(mix(ember,hot,core*v_heat)*alpha,alpha);
}`;

export function createEmberRenderer(canvas:HTMLCanvasElement,light:boolean,depth:EmberDepth="back") {
  const gl=canvas.getContext("webgl",{alpha:true,premultipliedAlpha:true,antialias:false,depth:false,stencil:false});
  if(!gl)return null;
  const shaders:WebGLShader[]=[];
  let program:WebGLProgram|null=null,buffer:WebGLBuffer|null=null,count=0;
  let timer:GpuTimer|null=null,query:WebGLQuery|null=null,pending=false,frames=0,samples=0,attempts=0,gpuTotal=0;
  const dispose=()=>{
    if(query)timer?.deleteQueryEXT(query);
    if(buffer)gl.deleteBuffer(buffer);
    if(program)gl.deleteProgram(program);
    shaders.forEach(shader=>gl.deleteShader(shader));
    gl.getExtension("WEBGL_lose_context")?.loseContext();
  };
  const compile=(type:number,source:string)=>{
    const shader=gl.createShader(type);if(!shader)return null;
    shaders.push(shader);gl.shaderSource(shader,source);gl.compileShader(shader);
    return gl.getShaderParameter(shader,gl.COMPILE_STATUS)?shader:null;
  };
  const vs=compile(gl.VERTEX_SHADER,vertex),fs=compile(gl.FRAGMENT_SHADER,fragment);
  if(!vs||!fs){dispose();return null;}
  program=gl.createProgram();buffer=gl.createBuffer();
  if(!program||!buffer){dispose();return null;}
  gl.attachShader(program,vs);gl.attachShader(program,fs);gl.linkProgram(program);
  if(!gl.getProgramParameter(program,gl.LINK_STATUS)){dispose();return null;}
  gl.useProgram(program);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
  const origin=gl.getAttribLocation(program,"a_origin"),seed=gl.getAttribLocation(program,"a_seed");
  gl.enableVertexAttribArray(origin);gl.vertexAttribPointer(origin,2,gl.FLOAT,false,24,0);
  gl.enableVertexAttribArray(seed);gl.vertexAttribPointer(seed,4,gl.FLOAT,false,24,8);
  gl.uniform1f(gl.getUniformLocation(program,"u_light"),light?1:0);
  gl.uniform1f(gl.getUniformLocation(program,"u_front"),depth==="front"?1:0);
  const view=gl.getUniformLocation(program,"u_view"),time=gl.getUniformLocation(program,"u_time"),scale=gl.getUniformLocation(program,"u_scale");
  gl.enable(gl.BLEND);gl.blendFunc(gl.ONE,gl.ONE_MINUS_SRC_ALPHA);
  // Six échantillons GPU facultatifs, asynchrones, sans gl.finish ni boucle d’attente.
  timer=gl.getExtension("EXT_disjoint_timer_query") as GpuTimer|null;
  query=timer?.createQueryEXT()??null;
  canvas.dataset.gpuTimer=query?"sampling":"unavailable";
  return {
    dispose,
    resize(width:number,height:number,points:Float32Array){
      gl.viewport(0,0,canvas.width,canvas.height);
      gl.uniform2f(view,width,height);gl.uniform1f(scale,canvas.width/width);
      gl.bufferData(gl.ARRAY_BUFFER,points,gl.STATIC_DRAW);count=points.length/6;
    },
    draw(elapsed:number){
      frames++;
      if(timer&&query&&frames>240){timer.deleteQueryEXT(query);query=null;canvas.dataset.gpuTimer="inconclusive";}
      if(timer&&query&&pending&&timer.getQueryObjectEXT(query,timer.QUERY_RESULT_AVAILABLE_EXT)){
        if(!gl.getParameter(timer.GPU_DISJOINT_EXT)){
          gpuTotal+=Number(timer.getQueryObjectEXT(query,timer.QUERY_RESULT_EXT))/1e6;samples++;
          canvas.dataset.gpuMs=(gpuTotal/samples).toFixed(3);canvas.dataset.gpuSamples=String(samples);
        }
        pending=false;
      }
      const measuring=!!timer&&!!query&&!pending&&attempts<6&&(frames-1)%30===0;
      if(measuring){attempts++;timer!.beginQueryEXT(timer!.TIME_ELAPSED_EXT,query!);}
      gl.clear(gl.COLOR_BUFFER_BIT);gl.uniform1f(time,(elapsed/1000)%360);gl.drawArrays(gl.POINTS,0,count);
      if(measuring){timer!.endQueryEXT(timer!.TIME_ELAPSED_EXT);pending=true;}
      if(attempts>=6&&!pending&&query){timer!.deleteQueryEXT(query);query=null;canvas.dataset.gpuTimer=samples===6?"complete":"inconclusive";}
    },
  };
}
