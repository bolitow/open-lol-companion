use serde::{Deserialize, Serialize};

use crate::JsonApiEvent;

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

    /// Vrai pendant les écrans d'après-partie : le bilan en mémoire reste disponible.
    pub fn is_post_game(self) -> bool {
        matches!(
            self,
            Self::WaitingForStats | Self::PreEndOfGame | Self::EndOfGame
        )
    }

    /// Nouvelle phase portée par un événement WebSocket, s'il concerne cet endpoint.
    pub fn from_event(event: &JsonApiEvent) -> Option<Self> {
        if event.uri != Self::ENDPOINT {
            return None;
        }
        Self::deserialize(&event.data).ok()
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
        assert!(!p.is_post_game());
    }

    #[test]
    fn reconnait_les_phases_d_apres_partie() {
        for (name, expected) in [
            ("WaitingForStats", true),
            ("PreEndOfGame", true),
            ("EndOfGame", true),
            ("InProgress", false),
            ("Lobby", false),
            ("None", false),
        ] {
            let p: GameflowPhase = serde_json::from_str(&format!("\"{name}\"")).unwrap();
            assert_eq!(p.is_post_game(), expected, "{name}");
        }
    }

    #[test]
    fn lit_la_phase_d_un_evenement() {
        let event = |uri: &str, data| JsonApiEvent {
            uri: uri.into(),
            event_type: "Update".into(),
            data,
        };
        let phase = GameflowPhase::from_event(&event(GameflowPhase::ENDPOINT, "Lobby".into()));
        assert_eq!(phase, Some(GameflowPhase::Lobby));
        assert_eq!(
            GameflowPhase::from_event(&event("/lol-chat/v1/me", "Lobby".into())),
            None
        );
        // Suppression de la ressource : pas de donnée, pas de phase.
        assert_eq!(
            GameflowPhase::from_event(&event(GameflowPhase::ENDPOINT, serde_json::Value::Null)),
            None
        );
    }

    #[test]
    fn tolere_une_phase_inconnue() {
        let p: GameflowPhase = serde_json::from_str("\"SwarmLobby2030\"").unwrap();
        assert_eq!(p, GameflowPhase::Unknown);
    }
}
