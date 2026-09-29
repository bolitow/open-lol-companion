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
