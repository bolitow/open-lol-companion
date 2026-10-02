/** Identité saisie explicitement ; ne prouve pas la propriété du compte. */
export interface PlayerRequest {platform:string;game_name:string;tag_line:string}
/** Commande Tauri ; les routes et l'autorisation restent en Rust. */
export interface PlayerMatchesRequest {player:PlayerRequest;start:number;count:number}
/** Miroir Rust PlayerError ; desktop_required est propre à l'aperçu navigateur. */
export type PlayerError='not_configured'|'invalid_configuration'|'invalid_request'|'unauthorized'|'not_found'|'unavailable'|'rate_limited'|'invalid_response'|'desktop_required';
