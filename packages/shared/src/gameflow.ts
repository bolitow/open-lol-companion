import type {DraftSession} from "./draft";
/** Phases renvoyées par `GET /lol-gameflow/v1/gameflow-phase` (miroir de `GameflowPhase` côté Rust). */
export const GAMEFLOW_PHASES = [
  "None",
  "Lobby",
  "Matchmaking",
  "CheckedIntoTournament",
  "ReadyCheck",
  "ChampSelect",
  "GameStart",
  "FailedToLaunch",
  "InProgress",
  "Reconnect",
  "WaitingForStats",
  "PreEndOfGame",
  "EndOfGame",
  "TerminatedInError",
  "Unknown",
] as const;

export type GameflowPhase = (typeof GAMEFLOW_PHASES)[number];

/** Écran principal de l'app à afficher pour une phase donnée (section 4.3 du cahier des charges). */
export type AppScreen = "dashboard" | "champ-select" | "in-game" | "post-game";

export function screenForPhase(phase: GameflowPhase): AppScreen {
  switch (phase) {
    case "ChampSelect":
      return "champ-select";
    case "GameStart":
    case "InProgress":
    case "Reconnect":
      return "in-game";
    case "WaitingForStats":
    case "PreEndOfGame":
    case "EndOfGame":
      return "post-game";
    default:
      return "dashboard";
  }
}

/** État de connexion au client exposé par la commande Tauri `lcu_status`. */
export interface LcuStatus {
  connected: boolean;
  port: number | null;
  message: string;
}

/** Page équipée renvoyée par le client ; les fragments peuvent répéter un identifiant. */
export interface RunePage {
  primaryStyleId: number;
  subStyleId: number;
  selectedPerkIds: number[];
  isValid: boolean;
  isTemporary: boolean;
  autoModifiedSelections: number[];
}

/** Identité publique locale ; aucun PUUID, identifiant de session ou secret. */
export interface LcuAccount { platform:string; game_name:string; tag_line:string }

/** État courant versionné, miroir de `lcu_connector::LcuSession`. */
export interface LcuSession {
  /** Identité locale de draft, disponible avant le gameId Riot. */
  draftId?: string;
  revision: number;
  account: LcuAccount | null;
  connected: boolean;
  phase: GameflowPhase | null;
  draft: DraftSession | null;
  runePage: RunePage | null;
}
