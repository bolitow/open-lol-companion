import type {Role} from './api';

/** Projection locale de la Live Client Data API ; aucune identité ni donnée adverse. */
export interface LivePlayer {
  championKey: string;
  level: number;
  kills: number;
  deaths: number;
  assists: number;
  creepScore: number;
  items: number[];
}
export interface LiveEvent {
  id: number;
  name: 'GameStart' | 'MinionsSpawning' | 'GameEnd';
  time: number;
}
export interface LiveGame {
  gameTime: number;
  gameMode: string;
  mapNumber: number;
  player: LivePlayer;
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
  height: number;
  style: 'dark' | 'solid';
  locale: 'fr' | 'en';
}
export interface OverlayState {
  material: 'solid' | 'vibrancy' | 'liquidGlass';
  revision: number;
  preferences: OverlayPreferences;
  available: boolean;
  visible: boolean;
  preview: boolean;
  editSession: number | null;
  error: 'unavailable' | 'storage' | null;
}

export type OverlayEditAction = {type:'start'} | {type:'begin';session:number;kind:'move'|'resize'} | {type:'move'|'end'|'commit'|'cancel';session:number};
