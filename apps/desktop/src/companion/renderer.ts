import * as THREE from 'three';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
import {CompanionState,poseFor,WelcomeSession} from './state';
import {vertexShader,fragmentShader} from './fire';
const welcomeSession=new WelcomeSession();
export type CompanionOptions={motion:boolean;light:boolean;visible:boolean};
export function createCompanion(canvas:HTMLCanvasElement,initial:CompanionOptions,onFailure:()=>void){
 let options=initial,disposed=false,raf=0,last=performance.now(),time=0,lastDraw=0,active='',staticKey='',onScreen=false,started=false;
 const state=new CompanionState();if(!initial.motion)state.setMotion(false);
 const renderer=new THREE.WebGLRenderer({canvas,alpha:true,premultipliedAlpha:false,antialias:false,powerPreference:'low-power'});
 renderer.toneMapping=THREE.ACESFilmicToneMapping;renderer.toneMappingExposure=1.15;
 const target=new THREE.WebGLRenderTarget(256,256,{samples:4,depthBuffer:true});
 const scene=new THREE.Scene(),camera=new THREE.PerspectiveCamera(33,1,.1,20);camera.position.set(0,1.7,7);camera.lookAt(0,1.7,0);
 scene.add(new THREE.HemisphereLight(0xffdfbc,0x503039,2));
 for(const [color,power,x,y,z]of [[0xffe1b0,3,3,5,4],[0xff6533,2,-3,3,-2],[0xadc6ff,.6,-3,2,3]]){
  const light=new THREE.DirectionalLight(color!,power!);light.position.set(x!,y!,z!);scene.add(light);
 }
 const root=new THREE.Group();scene.add(root);
 const material=new THREE.ShaderMaterial({uniforms:{uModel:{value:target.texture},uTime:{value:0},uHeat:{value:0},uEnergy:{value:0},uLight:{value:0},uMotion:{value:initial.motion?1:0}},vertexShader,fragmentShader,depthTest:false,depthWrite:false});
 const quad=new THREE.Mesh(new THREE.PlaneGeometry(2,2),material),output=new THREE.Scene();output.add(quad);const outputCamera=new THREE.Camera();
 let model:THREE.Group|undefined,mixer:THREE.AnimationMixer|undefined,clips:Record<string,THREE.AnimationClip>={};
 const skins:THREE.MeshStandardMaterial[]=[];
 function release(object:THREE.Object3D){const geometries=new Set<THREE.BufferGeometry>(),materials=new Set<THREE.Material>();object.traverse(o=>{if(o instanceof THREE.SkinnedMesh)o.skeleton.dispose();if(o instanceof THREE.Mesh){geometries.add(o.geometry);for(const m of Array.isArray(o.material)?o.material:[o.material])materials.add(m);}});geometries.forEach(g=>g.dispose());materials.forEach(m=>m.dispose());}
 function fail(){if(disposed)return;dispose();onFailure();}
 const observer=new ResizeObserver(()=>{const size=Math.min(512,Math.max(192,Math.round(canvas.clientWidth*Math.min(devicePixelRatio,1.5))));renderer.setSize(size,size,false);target.setSize(size,size);staticKey='';});observer.observe(canvas);
 const intersection=new IntersectionObserver(entries=>{onScreen=entries.some(e=>e.isIntersecting);});intersection.observe(canvas);
 new GLTFLoader().load('/companion/flame.glb',gltf=>{
  if(disposed){release(gltf.scene);return;}
  model=gltf.scene;root.add(model);mixer=new THREE.AnimationMixer(model);clips=Object.fromEntries(gltf.animations.map(c=>[c.name,c]));
  model.traverse(o=>{if(o instanceof THREE.Mesh){o.frustumCulled=false;if(o.name.includes('peau')&&o.material instanceof THREE.MeshStandardMaterial){o.material.roughness=.62;o.material.emissive=new THREE.Color(0xff6418);o.material.emissiveIntensity=.1;skins.push(o.material);}}});
  last=performance.now();
 },undefined,()=>{fail();});
 const lost=(event:Event)=>{event.preventDefault();fail();};canvas.addEventListener('webglcontextlost',lost);
 function draw(now:number){
  if(disposed)return;raf=requestAnimationFrame(draw);
  if(document.hidden||!options.visible||!onScreen){last=now;return;}
  if(now-lastDraw<33||!model||!mixer)return;
  if(!started){state.enter(welcomeSession.claim()&&options.motion?'appear':'idle');started=true;}
  lastDraw=now;const dt=Math.min((now-last)/1000,.1);last=now;time+=dt;state.advance(dt);
  canvas.dataset.phase=state.phase;canvas.dataset.motion=String(options.motion);const pose=poseFor(state),key=`${pose.clip}:${options.light}:${options.motion}`;
  if(!options.motion&&key===staticKey)return;staticKey=key;
  if(active!==pose.clip){mixer.stopAllAction();const clip=clips[pose.clip];if(clip){const action=mixer.clipAction(clip).reset().play();if(pose.clip.startsWith('To')||pose.clip.startsWith('Wave')){action.setLoop(THREE.LoopOnce,1);action.clampWhenFinished=true;if(pose.clipDuration)action.setEffectiveTimeScale(clip.duration/pose.clipDuration);}}active=pose.clip;}
  if(options.motion)mixer.update(dt);else mixer.setTime(.1);
  root.scale.set(pose.scale/Math.sqrt(pose.squash),pose.scale*pose.squash,pose.scale);
  model.scale.set(1+(options.motion?.007*Math.sin(time*2):0),1+(options.motion?.01*Math.sin(time*2+1):0),1);
  if(options.motion){model.getObjectByName('tip')?.rotateZ(.035*Math.sin(time*3.3));model.getObjectByName('tip_end')?.rotateZ(.06*Math.sin(time*4.1+.5));}
  for(const skin of skins){skin.color.setRGB(1,1-.5*pose.heat,1-.55*pose.heat);skin.emissiveIntensity=.1+.1*pose.heat;}
  material.uniforms.uTime!.value=options.motion?time:0;material.uniforms.uHeat!.value=pose.heat;material.uniforms.uEnergy!.value=pose.energy;material.uniforms.uLight!.value=0;material.uniforms.uMotion!.value=options.motion?1:0;
  renderer.setRenderTarget(target);renderer.setClearColor(0,0);renderer.clear();renderer.render(scene,camera);
  renderer.setRenderTarget(null);renderer.clear();renderer.render(output,outputCamera);
 }
 function dispose(){if(disposed)return;disposed=true;cancelAnimationFrame(raf);observer.disconnect();intersection.disconnect();canvas.removeEventListener('webglcontextlost',lost);mixer?.stopAllAction();if(model){mixer?.uncacheRoot(model);release(model);}target.dispose();quad.geometry.dispose();material.dispose();renderer.dispose();renderer.forceContextLoss();}
 raf=requestAnimationFrame(draw);
 return {
  interact(){state.interact();staticKey='';},
  configure(next:CompanionOptions){if(next.motion!==options.motion){state.setMotion(next.motion);active='';staticKey='';}options=next;},
  dispose,
 };
}
