// Même palette et turbulence que prototype/fireShader.ts. La source de feu est
// ici l'alpha du modèle animé : aucun plan de flammes placé derrière le corps.
export const vertexShader=`varying vec2 vUv;void main(){vUv=uv;gl_Position=vec4(position.xy,0.,1.);}`;
export const fragmentShader=`
precision highp float;
varying vec2 vUv;
uniform sampler2D uModel;
uniform float uTime,uHeat,uEnergy,uLight,uMotion;
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float noise(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),f.x),f.y);}
float fbm(vec2 p){float v=0.,a=.5;for(int i=0;i<4;i++){v+=a*noise(p);p=mat2(1.6,1.2,-1.2,1.6)*p+3.7;a*=.5;}return v;}
float silhouette(vec2 uv){return texture2D(uModel,clamp(uv,vec2(.001),vec2(.999))).a;}
void main(){
 vec2 uv=vUv;float t=uTime;
 vec2 flow=vec2(fbm(uv*4.+vec2(t*.3,0)),fbm(uv*4.+vec2(0,-t*.65)));
 float turbulence=fbm(uv*9.+flow*2.-vec2(0,t*1.8));
 float tongues=smoothstep(.30,.72,fbm(vec2(uv.x*22.,uv.y*7.-t*1.5)+flow));
 vec4 body=texture2D(uModel,uv);
 float extent=mix(.014,.06+.045*uHeat+.045*uEnergy,uMotion);
 float flame=0.;
 // Remonter la silhouette sur plusieurs distances produit des langues attachées
 // aux bras et à la tête, y compris pendant la rotation et le salut.
 for(int i=1;i<=12;i++){
  float k=float(i)/12.;
  float lift=k*extent*(.3+1.5*tongues);
  float wind=(flow.x-.5)*extent*k*1.4;
  float spread=extent*.08*k;
  float a=max(silhouette(uv-vec2(wind+spread,lift)),silhouette(uv-vec2(wind-spread,lift)));
  flame=max(flame,a*(1.-k*.72));

 }
 float outside=1.-body.a;
 // Le bas reste sombre : seuls les contours orientés vers le haut brûlent.
 float upper=smoothstep(.24,.55,uv.y);
 float fire=smoothstep(.22,.80,flame)*mix(.25,1.,tongues)*outside*upper;
 vec3 red=vec3(1.,.075,.025),orange=vec3(1.,.36,.045);
 vec3 fireColor=mix(red,orange,smoothstep(.45,.9,flame)*.7);
 // Alpha droit : éviter de multiplier deux fois le feu par son opacité.
 float alpha=body.a+fire*(1.-body.a);
 vec3 rgb=(body.rgb*body.a+fireColor*fire*(1.-body.a))/max(alpha,.001);
 gl_FragColor=vec4(rgb,alpha);
 #include <tonemapping_fragment>
 #include <colorspace_fragment>
}
`;
