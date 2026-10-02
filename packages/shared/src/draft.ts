/** Projection Rust de la sélection, sans identifiants ni intention adverse. */
export interface DraftPlayer {
 cellId:number; championId:number|null; locked:boolean; local:boolean;
 position:'top'|'jungle'|'middle'|'bottom'|'utility'|null; acting:boolean;
}
export interface DraftTimer {remainingMs:number;observedAtMs:number}
export interface DraftSession {
 supported:boolean;allySide:'blue'|'red'|null;
 allies:DraftPlayer[];enemies:DraftPlayer[];allyBans:number[];enemyBans:number[];
 timer:DraftTimer|null;
 /** Emplacements D puis F, uniquement pour le joueur local. */
 localSpells:[number,number]|null;
}
