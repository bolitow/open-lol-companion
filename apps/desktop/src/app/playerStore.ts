import type {PlayerError,PlayerRequest,PlayerHistoryRequest,PlayerProfile,PlayerHistory,PlayerMatchView} from '@olc/shared';

export const playerPlatforms=['EUW1','EUN1','NA1','KR','BR1','JP1','LA1','LA2','ME1','OC1','RU','SG2','TR1','TW2','VN2'] as const;
const validText=(value:string,max:number)=>!!value&&value.trim()===value&&!['.','..'].includes(value)&&!/[\u0000-\u001f\u007f-\u009f#]/u.test(value)&&new TextEncoder().encode(value).length<=max;
export function parsePlayerQuery(value:string,platform:string):PlayerRequest|null {
    // Refuser les contrôles avant trim, pour rester cohérent avec la frontière Rust.
    if(/[\u0000-\u001f\u007f-\u009f]/u.test(value))return null;
    const parts=value.split('#').map(part=>part.trim()),name=parts[0]??'',tag=parts[1]??'';
    return parts.length===2&&validText(name,64)&&validText(tag,32)&&playerPlatforms.includes(platform as typeof playerPlatforms[number])?{platform,game_name:name,tag_line:tag}:null;
}
export function parseHomePlayer(raw:string|null):PlayerRequest|null {
    try{const value:unknown=JSON.parse(raw??'null');if(!value||typeof value!=='object')return null;
        const data=value as Record<string,unknown>;
        if(typeof data.platform!=='string'||typeof data.game_name!=='string'||typeof data.tag_line!=='string')return null;
        return parsePlayerQuery(`${data.game_name}#${data.tag_line}`,data.platform);
    }catch{return null}
}
export const playerKey=(player:PlayerRequest)=>JSON.stringify([player.platform,player.game_name.toLowerCase(),player.tag_line.toLowerCase()]);
export const playerIdentity=(player:PlayerRequest):PlayerRequest=>({platform:player.platform,game_name:player.game_name,tag_line:player.tag_line});
const knownErrors:PlayerError[]=['not_configured','invalid_configuration','invalid_request','unauthorized','not_found','unavailable','riot_busy','rate_limited','invalid_response','desktop_required'];
const cleanError=(error:unknown):PlayerError=>knownErrors.includes(error as PlayerError)?error as PlayerError:'unavailable';
export interface PlayerTransport {profile:(request:PlayerRequest)=>Promise<PlayerProfile>;matches:(request:PlayerHistoryRequest)=>Promise<PlayerHistory>}
export interface PlayerEntry {identity:PlayerRequest;profile:PlayerProfile|null;loading:boolean;error:PlayerError|null;matches:PlayerMatchView[];historyLoading:boolean;historyRefresh:boolean;historyError:PlayerError|null;next:number|null;omitted:number;historyFetchedAt:number|null;historySource:'lcu'|'api'|null;scrollTop:number}
export interface PlayersState {connected:boolean;active:PlayerRequest|null;home:PlayerRequest|null;viewed:PlayerRequest|null;entries:Record<string,PlayerEntry>;storageFailed:boolean;query:string;platform:string}
const emptyEntry=(identity:PlayerRequest):PlayerEntry=>({identity,profile:null,loading:true,error:null,matches:[],historyLoading:false,historyRefresh:false,historyError:null,next:0,omitted:0,historyFetchedAt:null,historySource:null,scrollTop:0});

/** Deux profils maximum en mémoire : compte d'accueil et joueur consulté. Aucun polling. */
export function createPlayerStore(transport:PlayerTransport,home:PlayerRequest|null,persist:(value:PlayerRequest|null)=>void){
    let state:PlayersState={connected:false,active:null,home,viewed:null,entries:{},storageFailed:false,query:'',platform:home?.platform??'EUW1'};
    let lastPhase:string|null=null;
    const listeners=new Set<()=>void>(),versions=new Map<string,number>();
    const publish=(patch:Partial<PlayersState>)=>{state={...state,...patch};listeners.forEach(listener=>listener())};
    const update=(key:string,patch:Partial<PlayerEntry>)=>{if(state.entries[key])publish({entries:{...state.entries,[key]:{...state.entries[key],...patch}}})};
    const current=(key:string,version:number)=>!!state.entries[key]&&versions.get(key)===version;
    const retain=()=>{const keys=[state.home,state.viewed].filter((p):p is PlayerRequest=>!!p).map(playerKey);const entries={...state.entries};for(const key of Object.keys(entries))if(!keys.includes(key)){delete entries[key];versions.set(key,(versions.get(key)??0)+1)}publish({entries})};
    const writeHome=(value:PlayerRequest|null)=>{let storageFailed=false;try{persist(value)}catch{storageFailed=true}publish({home:value,storageFailed});retain()};
    const loadPage=async(key:string,version:number,replace=false)=>{
        const entry=state.entries[key];if(!entry?.profile||entry.loading||entry.historyLoading||entry.next===null&&!replace)return;
        const request={player:playerIdentity(entry.profile),start:replace?0:entry.next!,count:10,source:entry.profile.source};
        update(key,{historyLoading:true,historyRefresh:replace,historyError:null});
        try{
            const page=await transport.matches(request);if(!current(key,version))return;
            if(page.source!==request.source||playerKey(page)!==key||page.start!==request.start||page.count!==request.count||page.matches.length+page.omitted_matches>request.count||page.next_start!==null&&(page.next_start!==request.start+request.count||page.next_start>10000))throw 'invalid_response';
            // L'arrivée d'une partie entre deux pages peut décaler les résultats de l'API.
            const latest=state.entries[key];if(!latest)return;const previous=replace?[]:latest.matches;const ids=new Set(previous.map(match=>match.match_id));
            const added=page.matches.filter(match=>{if(ids.has(match.match_id))return false;ids.add(match.match_id);return true});
            update(key,{matches:[...previous,...added],historyLoading:false,historyRefresh:false,next:page.next_start,omitted:(replace?0:latest.omitted)+page.omitted_matches,historyFetchedAt:page.fetched_at,historySource:page.source});
        }catch(error){if(current(key,version))update(key,{historyLoading:false,historyError:cleanError(error)})}
    };
    const load=async(identity:PlayerRequest,force=false,preserve=false)=>{
        const key=playerKey(identity);if(state.entries[key]&&!force)return;
        const version=(versions.get(key)??0)+1;versions.set(key,version);
        const existing=state.entries[key];
        const keep=preserve&&!!existing?.profile;
        publish({entries:{...state.entries,[key]:keep?{...existing,loading:true,error:null,historyLoading:false}:emptyEntry(identity)}});
        try{
            const profile=await transport.profile(identity);if(!current(key,version))return;
            if(playerKey(profile)!==key)throw 'invalid_response';
            update(key,{profile,loading:false});await loadPage(key,version,keep);
        }catch(error){if(current(key,version))update(key,{loading:false,error:cleanError(error)})}
    };
    return {
        getSnapshot:()=>state,
        subscribe:(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener)}},
        // L'identité locale ne dépend pas de la disponibilité du service de profils.
        syncAccount:(connected:boolean,account:PlayerRequest|null)=>{
            const active=connected&&account?parseHomePlayer(JSON.stringify(account)):null;
            if(state.connected===connected&&JSON.stringify(state.active)===JSON.stringify(active))return;
            const previous=state.active;
            publish({connected,active});
            if(!active)return;
            const changed=!state.home||JSON.stringify(state.home)!==JSON.stringify(active);
            if(changed)writeHome(playerIdentity(active));
            // Une reconnexion actualise aussi le même compte ; les ticks de draft ne rechargent rien.
            void load(active,!previous||playerKey(previous)!==playerKey(active),true);
        },
        syncPhase:(phase:string|null)=>{
            const completed=phase==='EndOfGame'&&lastPhase!=='EndOfGame';lastPhase=phase;
            if(completed&&state.active&&!state.entries[playerKey(state.active)]?.loading)void load(state.active,true,true);
        },
        start:()=>{if(state.home)void load(state.home)},
        select:(identity:PlayerRequest)=>{publish({viewed:identity,query:`${identity.game_name}#${identity.tag_line}`,platform:identity.platform});retain();void load(identity)},
        edit:(query:string,platform:string)=>publish({query,platform}),
        pin:()=>{const profile=state.viewed&&state.entries[playerKey(state.viewed)]?.profile;if(profile&&!state.connected)writeHome(playerIdentity(profile))},
        forget:()=>{if(!state.connected)writeHome(null)},
        refresh:(identity=state.viewed)=>{if(identity)void load(identity,true,true)},
        more:async(identity=state.viewed)=>{if(identity){const key=playerKey(identity);await loadPage(key,versions.get(key)??0,state.entries[key]?.historyRefresh??false)}},
        scroll:(scrollTop:number,identity=state.viewed)=>{if(identity)update(playerKey(identity),{scrollTop})},
    };
}
export type PlayerStore=ReturnType<typeof createPlayerStore>;
