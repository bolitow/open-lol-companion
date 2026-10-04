/** Passages publiés et vérifiés du catalogue SkinSpotlights. */
export type SpotlightSegmentKind =
  | 'passive'
  | 'q'
  | 'w'
  | 'e'
  | 'r'
  | 'emotes'
  | 'recall'
  | 'attack'
  | 'movement'
  | 'death';

export type SpotlightSelection = 'full' | SpotlightSegmentKind;

export interface SpotlightSegment {
  kind: SpotlightSegmentKind;
  start: number;
  end: number;
}

export interface SpotlightVideo {
  skinId: number;
  championId: number;
  videoId: string;
  name: string;
  segments: SpotlightSegment[];
}

/** État autoritaire Rust ; chaque action exige sa révision courante. */
export interface SpotlightState {
  revision: number;
  video: SpotlightVideo | null;
  selected: SpotlightSelection;
  detached: boolean;
  locale: 'fr' | 'en';
}

export type SpotlightAction =
  | {
      type: 'open';
      skinId: number;
      championId: number;
      locale: 'fr' | 'en';
      kind?: SpotlightSelection | null;
    }
  | { type: 'select'; kind: SpotlightSelection }
  | { type: 'step'; direction: -1 | 1 }
  | { type: 'detach' }
  | { type: 'attach' }
  | { type: 'close' }
  | { type: 'retry' }
  | { type: 'external' };

/** Chargement du document natif uniquement ; « loaded » ne prouve pas la lecture vidéo. */
export interface SpotlightMediaState {
  attempt: number;
  status: 'idle' | 'loading' | 'loaded' | 'slow' | 'failed';
}

/** Rectangle DOM du média natif ; null masque la Webview sans interrompre sa session. */
export interface SpotlightVideoBounds {x:number;y:number;width:number;height:number}
export interface SpotlightLayoutRequest {revision:number;bounds:SpotlightVideoBounds|null}
