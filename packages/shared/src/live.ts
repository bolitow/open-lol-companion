import type {Role} from './api';

/**
 * Projection locale de la Live Client Data API par liste blanche (#102) : aucune identité,
 * aucune donnée adverse cachée (or, sorts, positions), aucun nom de joueur dans les événements.
 */
export interface LivePlayer {
  championKey: string;
  level: number;
  kills: number;
  deaths: number;
  assists: number;
  creepScore: number;
  items: number[];
  /** Champs du joueur local seul ; `null` si la source ne les fournit pas ou s'ils sont invalides. */
  currentGold: number | null;
  wardScore: number | null;
  isDead: boolean | null;
  respawnTimer: number | null;
  abilityLevels: LiveAbilityLevels | null;
  team: LiveSide | null;
  position: Role | null;
}
/** Niveaux appris des compétences Q, W, E et R du joueur local. */
export interface LiveAbilityLevels {
  q: number;
  w: number;
  e: number;
  r: number;
}
/** Côté de la carte : `order` (bleu) ou `chaos` (rouge). */
export type LiveSide = 'order' | 'chaos';
/** Totaux visibles au tableau des scores, sans or, sorts ni vision adverse. */
export interface LiveTeamTotals {
  kills: number;
  deaths: number;
  assists: number;
  creepScore: number;
}
/** Équipes relatives au joueur local ; `null` si une donnée du tableau des scores manque. */
export interface LiveTeams {
  allies: LiveTeamTotals;
  enemies: LiveTeamTotals;
}
export type LiveEventName =
  | 'GameStart'
  | 'MinionsSpawning'
  | 'GameEnd'
  | 'ChampionKill'
  | 'FirstBlood'
  | 'DragonKill'
  | 'HeraldKill'
  | 'BaronKill'
  | 'TurretKilled'
  | 'InhibKilled'
  | 'Ace';
/**
 * Événement public annoncé à tous. `ally` : camp de l'auteur relatif au joueur local, `null`
 * si l'auteur n'est pas un joueur identifiable ; `involvesLocalPlayer` : auteur, assistant ou victime.
 */
export interface LiveEvent {
  id: number;
  name: LiveEventName;
  time: number;
  ally: boolean | null;
  involvesLocalPlayer: boolean;
}
export interface LiveGame {
  gameTime: number;
  gameMode: string;
  mapNumber: number;
  player: LivePlayer;
  teams: LiveTeams | null;
  events: LiveEvent[];
}
/** Contexte de la dernière draft réelle, sans champion consulté dans l'interface. */
export interface LiveContext {
  championId: number;
  role: Role | null;
  platform: string | null;
  queue: number | null;
  customGame: boolean;
}
export interface LiveSession {
  revision: number;
  generation: number;
  status: 'idle' | 'waiting' | 'ready' | 'unavailable' | 'invalid';
  context: LiveContext | null;
  game: LiveGame | null;
  /**
   * Dernière lecture valide de la partie terminée, conservée en mémoire jusqu'au bilan puis
   * purgée (nouvelle partie ou sortie des écrans d'après-partie).
   */
  postgame: LiveGame | null;
}

/** Réglages du panneau minimal. Le cadre relatif se rapporte à l'écran choisi. */
export interface OverlayPreferences {
  enabled: boolean;
  exclusiveFullscreen: boolean;
  monitor: number;
  x: number;
  y: number;
  width: number;
  opacity: number;
  locale: 'fr' | 'en';
}
export interface OverlayState {
  material: 'solid' | 'vibrancy' | 'liquidGlass';
  revision: number;
  preferences: OverlayPreferences;
  available: boolean;
  visible: boolean;
  preview: boolean;
  error: 'unavailable' | 'storage' | null;
}
