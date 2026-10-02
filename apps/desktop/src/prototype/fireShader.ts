// Shader original : bruit turbulent, papier carbonisé, halo et cœur incandescent.
const vertexSource=`attribute vec2 a_position;
void main(){gl_Position=vec4(a_position,0.0,1.0);}`;
const fragmentSource=`
precision highp float;
uniform vec2 u_resolution;
uniform float u_progress;
uniform float u_time;
uniform float u_light;
uniform float u_transition;
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float noise(vec2 p){
 vec2 i=floor(p),f=fract(p);f=f*f*(3.0-2.0*f);
 return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),f.x),f.y);
}
float fbm(vec2 p){
 float v=0.0,a=0.5;
 for(int i=0;i<4;i++){v+=a*noise(p);p=mat2(1.6,1.2,-1.2,1.6)*p+3.7;a*=0.5;}
 return v;
}
void main(){
 vec2 uv=gl_FragCoord.xy/u_resolution;
 vec2 p=(uv-.5)*vec2(u_resolution.x/u_resolution.y,1.0);
 float t=u_time;
 vec2 flow=vec2(fbm(p*4.0+vec2(t*.3,0)),fbm(p*4.0+vec2(0,-t*.65)));
 float turbulence=fbm(p*9.0+flow*2.0-vec2(0,t*1.8));
 float detail=noise(p*70.0+vec2(t*.4,-t*2.5));
 float radius=mix(-.24,length(vec2(u_resolution.x/u_resolution.y,1.0))*.64+.28,pow(u_progress,.95));
 float dist=length(p*vec2(1.0,.86))-radius+(turbulence-.48)*.22+(detail-.5)*.018;
 float sweep=p.x-mix(-u_resolution.x/u_resolution.y*.6-.3,u_resolution.x/u_resolution.y*.6+.3,u_progress)+(turbulence-.48)*.18+sin(p.y*7.0+t)*.04;
 dist=mix(dist,sweep,u_transition);
 float paper=smoothstep(-.008,.012,dist);
 float edge=exp(-abs(dist)*85.0);
 float fire=exp(-abs(dist+.022)*24.0)*(0.28+turbulence*.9);
 float bloom=exp(-abs(dist+.035)*9.0)*.33;
 float ash=exp(-abs(dist-.035)*27.0);
 vec3 base=mix(vec3(.082,.077,.086),vec3(.925,.938,.955),u_light);
 base+=(noise(gl_FragCoord.xy*.38)-.5)*.017;
 base*=1.0-ash*.75;
 vec3 red=mix(vec3(1.0,.075,.025),vec3(.20,.15,1.0),u_light);
 vec3 orange=mix(vec3(1.0,.36,.045),vec3(.06,.55,1.0),u_light);
 vec3 core=mix(vec3(1.0,.89,.54),vec3(.55,1.0,1.0),u_light);
 vec3 energy=red*bloom+mix(red,orange,turbulence)*fire*.9+core*edge*(.42+.48*noise(p*34.0+vec2(0,-t*2.0)));
 paper*=mix(1.0,.06,u_transition);
 float alpha=max(paper,clamp(fire*.8+bloom+edge,0.0,1.0));
 // Alpha prémultiplié : les halos se composent avec les couleurs de l’interface.
 gl_FragColor=vec4(min(base*paper+energy,vec3(alpha)),alpha);
}`;

/** Repli sans animation si WebGL est indisponible : l’application reste accessible. */
export function createFireRenderer(canvas:HTMLCanvasElement,light:boolean,transition=false) {
  const gl=canvas.getContext("webgl",{alpha:true,premultipliedAlpha:true,antialias:false,depth:false,stencil:false});
  if(!gl) return null;
  // Le bruit demande une mantisse suffisante ; certains GPU WebGL 1 n’en disposent pas.
  if(!gl.getShaderPrecisionFormat(gl.FRAGMENT_SHADER,gl.HIGH_FLOAT)?.precision){
    gl.getExtension("WEBGL_lose_context")?.loseContext();return null;
  }
  const shaders:WebGLShader[]=[];
  let program:WebGLProgram|null=null,buffer:WebGLBuffer|null=null;
  const dispose=()=>{
    if(buffer) gl.deleteBuffer(buffer);
    if(program) gl.deleteProgram(program);
    shaders.forEach(shader=>gl.deleteShader(shader));
    gl.getExtension("WEBGL_lose_context")?.loseContext();
  };
  const compile=(type:number,source:string)=>{
    const shader=gl.createShader(type);
    if(!shader) return null;
    shaders.push(shader);gl.shaderSource(shader,source);gl.compileShader(shader);
    return gl.getShaderParameter(shader,gl.COMPILE_STATUS)?shader:null;
  };
  const vertex=compile(gl.VERTEX_SHADER,vertexSource),fragment=compile(gl.FRAGMENT_SHADER,fragmentSource);
  if(!vertex||!fragment){dispose();return null;}
  program=gl.createProgram();buffer=gl.createBuffer();
  if(!program||!buffer){dispose();return null;}
  gl.attachShader(program,vertex);gl.attachShader(program,fragment);gl.linkProgram(program);
  if(!gl.getProgramParameter(program,gl.LINK_STATUS)){dispose();return null;}
  gl.useProgram(program);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
  gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([-1,-1,1,-1,-1,1,-1,1,1,-1,1,1]),gl.STATIC_DRAW);
  const position=gl.getAttribLocation(program,"a_position");
  gl.enableVertexAttribArray(position);gl.vertexAttribPointer(position,2,gl.FLOAT,false,0,0);
  gl.viewport(0,0,canvas.width,canvas.height);
  gl.uniform2f(gl.getUniformLocation(program,"u_resolution"),canvas.width,canvas.height);
  gl.uniform1f(gl.getUniformLocation(program,"u_light"),light?1:0);
  gl.uniform1f(gl.getUniformLocation(program,"u_transition"),transition?1:0);
  const progress=gl.getUniformLocation(program,"u_progress"),time=gl.getUniformLocation(program,"u_time");
  return {dispose,draw:(value:number,elapsed:number)=>{
    gl.uniform1f(progress,value);gl.uniform1f(time,elapsed/1000);gl.drawArrays(gl.TRIANGLES,0,6);
  }};
}
