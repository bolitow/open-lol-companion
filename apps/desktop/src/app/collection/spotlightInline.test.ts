import {expect,it} from 'vitest';
import {createInlineSpotlightStore,visibleSpotlightBounds,guardInlineOpen} from './spotlightInline';
it('un ancien nettoyage ne retire pas la nouvelle fiche',()=>{
 const store=createInlineSpotlightStore();const a={skinId:1,element:{} as HTMLElement},b={skinId:2,element:{} as HTMLElement};
 const removeA=store.register(a);const removeB=store.register(b);removeA();expect(store.getSnapshot()).toBe(b);removeB();expect(store.getSnapshot()).toBeNull();
});
it('ne place la vidéo que dans une zone entièrement visible de taille suffisante',()=>{
 const rect={x:20,y:30,width:320,height:200};const viewport={x:0,y:0,width:960,height:600};
 expect(visibleSpotlightBounds(rect,viewport,viewport,false)).toEqual(rect);
 expect(visibleSpotlightBounds(rect,viewport,viewport,true)).toBeNull();
 expect(visibleSpotlightBounds({...rect,y:-1},viewport,viewport,false)).toBeNull();
 expect(visibleSpotlightBounds({...rect,height:199},viewport,viewport,false)).toBeNull();
 expect(visibleSpotlightBounds(rect,{...viewport,height:200},viewport,false)).toBeNull();
});
it('ferme seulement la révision ouverte si la fiche disparaît pendant la commande',async()=>{
 let resolve!:(value:{revision:number;detached:boolean})=>void;let connected=true;const closed:number[]=[];
 const request=guardInlineOpen(new Promise<{revision:number;detached:boolean}>(done=>{resolve=done;}),()=>connected,async revision=>{closed.push(revision);});
 connected=false;resolve({revision:7,detached:false});await request;expect(closed).toEqual([7]);
 await guardInlineOpen(Promise.resolve({revision:9,detached:true}),()=>false,async revision=>{closed.push(revision);});expect(closed).toEqual([7]);
});
