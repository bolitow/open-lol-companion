import type {ClientPatch,ClientPatchError} from '@olc/shared';
/** Libellé public des patchs, versions catalogue et builds client numériques. Depuis 15, le jeu affiche l’année. */
export function publicClientPatch(version:string):string|null {
 const match=/^([1-9]\d?)\.([1-9]\d?)(?:\.\d+){0,2}$/.exec(version);
 if(!match)return null;
 const major=Number(match[1]);
 return `${major>=15?major+10:major}.${match[2]}`;
}
export interface ClientPatchSnapshot {native:boolean;pending:boolean;patch:string|null;error:ClientPatchError|null}
export function createClientPatchStore(native:boolean,read:()=>Promise<ClientPatch>){
 let state:ClientPatchSnapshot={native,pending:false,patch:null,error:null};
 const listeners=new Set<()=>void>();
 const update=(patch:Partial<ClientPatchSnapshot>)=>{state={...state,...patch};listeners.forEach(fn=>fn())};
 return {getSnapshot:()=>state,subscribe:(fn:()=>void)=>{listeners.add(fn);return()=>{listeners.delete(fn)}},
  async load(){
   if(!native||state.pending)return;
   update({pending:true,error:null});
   try{
    const value=await read(),patch=publicClientPatch(value.patch);
    // Le client ajoute un suffixe de build Riot ; seule la paire majeure/mineure sert au libellé.
    const raw=value.gameVersion;
    if(raw.length>256||!/^\d+\.\d+(?:\.[!-~]+)?$/.test(raw)||raw.split('.').some(part=>!part)||!patch||publicClientPatch(raw.split('.').slice(0,2).join('.'))!==patch)throw 'invalid_response';
    update({patch});
   }catch(error){update({patch:null,error:error==='invalid_response'?'invalid_response':'unavailable'})}
   finally{update({pending:false})}
  },
 };
}
export const clientPatchCopy={
 fr:{title:'Version du client LoL',path:'League / Client',loading:'Lecture de la version…',reload:'Actualiser',desktop:'Disponible dans l’application desktop.',empty:'Version non lue',errors:{unavailable:'Client LoL introuvable',invalid_response:'Version du client illisible'}},
 en:{title:'League client version',path:'League / Client',loading:'Reading version…',reload:'Refresh',desktop:'Available in the desktop application.',empty:'Version not read',errors:{unavailable:'League client not found',invalid_response:'Unreadable client version'}},
} satisfies Record<'fr'|'en',{title:string;path:string;loading:string;reload:string;desktop:string;empty:string;errors:Record<ClientPatchError,string>}>;
