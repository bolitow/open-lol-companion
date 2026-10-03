import {createContext,useContext,useEffect,useState,type ReactNode} from 'react';
import {createAbilityVideoSession} from './abilityVideoSession';

type Session=ReturnType<typeof createAbilityVideoSession<HTMLVideoElement>>;
// undefined : lecteur autonome ; null : fiche en fermeture ou session pas encore prête.
const VideoSessionContext=createContext<Session|null|undefined>(undefined);
export const useAbilityVideoSession=()=>useContext(VideoSessionContext);
export function ChampionVideoScope({championId,closing,children}:{championId:number;closing:boolean;children:ReactNode}){
 const [session,setSession]=useState<Session|null>(null);
 useEffect(()=>{
  if(closing)return;
  const current=createAbilityVideoSession(championId,()=>document.createElement('video'));setSession(current);
  return()=>current.dispose();
 },[championId,closing]);
 const active=!closing&&session?.championId===championId&&!session.disposed?session:null;
 return <VideoSessionContext.Provider value={active}>{children}</VideoSessionContext.Provider>;
}
