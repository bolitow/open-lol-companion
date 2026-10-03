import {useCallback,useEffect,useRef,useState,type RefObject} from 'react';
import {AmbientEmbers} from '../prototype/AmbientEmbers';
import {BurnReveal} from '../prototype/BurnReveal';
import {PageFlame} from '../prototype/PageFlame';
import {transitionKind,type Scene} from './visualMotion';

// Surfaces stables : les listes asynchrones et les cartes vides ne pilotent pas le feu.
const panels='.home-player,.welcome,.team,.champions-library,.player-search-form,.settings-search';
export function ApplicationEffects({root,scene,theme,enabled}:{root:RefObject<HTMLDivElement|null>;scene:Scene;theme:'dark'|'light';enabled:boolean}){
 const [ready,setReady]=useState(false);
 // Les effets enfants doivent attendre l’attachement de la ref du conteneur.
 useEffect(()=>setReady(true),[]);
 const [opening,setOpening]=useState(enabled),[flaming,setFlaming]=useState(false);
 const [transition,setTransition]=useState<{cue:string;phase:boolean}|null>(null);
 const previous=useRef<Scene|null>(null),sequence=useRef(0);
 const finishOpening=useCallback(()=>setOpening(false),[]);
 useEffect(()=>{
  const kind=transitionKind(previous.current,scene);previous.current=scene;
  if(!enabled){setOpening(false);setTransition(null);return;}
  if(kind){setOpening(false);setTransition({cue:String(++sequence.current),phase:kind==='phase'});}
 },[scene.screen,scene.phase,scene.connected,enabled]);
 if(!ready)return null;
 return <>
  <AmbientEmbers root={root} theme={theme} panelSelector={panels} scene={scene.screen} enabled={enabled&&!opening&&!flaming}/>
  {transition&&<PageFlame root={root} theme={theme} cue={transition.cue} phase={transition.phase} enabled={enabled&&!opening} onActive={setFlaming}/>}
  {opening&&enabled&&<BurnReveal root={root} theme={theme} panelSelector={panels} onDone={finishOpening}/>}
 </>;
}
