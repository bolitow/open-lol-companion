use serde::{Deserialize, Serialize};

/// Phase de jeu renvoyée par `GET /lol-gameflow/v1/gameflow-phase`.
/// L'app change d'écran selon cette valeur (section 4.3 du cahier des charges).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameflowPhase {
    None,
    Lobby,
    Matchmaking,
    CheckedIntoTournament,
    ReadyCheck,
    ChampSelect,
    GameStart,
    FailedToLaunch,
    InProgress,
    Reconnect,
    WaitingForStats,
    PreEndOfGame,
    EndOfGame,
    TerminatedInError,
    /// Valeur inconnue (nouvelle phase ajoutée par Riot) : on ne plante pas.
    #[serde(other)]
    Unknown,
}

impl GameflowPhase {
    /// Endpoint LCU qui renvoie la phase courante.
    pub const ENDPOINT: &'static str = "/lol-gameflow/v1/gameflow-phase";

    /// Vrai pendant la sélection des champions (écran de draft).
    pub fn is_champ_select(self) -> bool {
        self == Self::ChampSelect
    }

    /// Vrai quand une partie tourne (overlays et enregistrement actifs).
    pub fn is_in_game(self) -> bool {
        matches!(self, Self::GameStart | Self::InProgress | Self::Reconnect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialise_la_reponse_du_client() {
        let p: GameflowPhase = serde_json::from_str("\"ChampSelect\"").unwrap();
        assert!(p.is_champ_select());
        let p: GameflowPhase = serde_json::from_str("\"InProgress\"").unwrap();
        assert!(p.is_in_game());
    }

    #[test]
    fn tolere_une_phase_inconnue() {
        let p: GameflowPhase = serde_json::from_str("\"SwarmLobby2030\"").unwrap();
        assert_eq!(p, GameflowPhase::Unknown);
    }
}
