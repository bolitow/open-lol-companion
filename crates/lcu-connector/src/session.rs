use crate::{DraftSession, GameflowPhase, LcuEvent, RunePage};
use serde::Serialize;

/// État public de session : aucun port ni identifiant d'authentification.
/// Miroir exact de `LcuSession` dans @olc/shared.
#[derive(Clone, Default, Serialize)]
pub struct LcuSession {
    pub revision: u32,
    #[serde(rename = "draftId", skip_serializing_if = "Option::is_none")]
    pub draft_id: Option<String>,
    #[serde(skip)]
    draft_generation: u64,
    pub connected: bool,
    pub account: Option<crate::LcuAccount>,
    pub phase: Option<GameflowPhase>,
    pub draft: Option<DraftSession>,
    #[serde(rename = "runePage")]
    pub rune_page: Option<RunePage>,
}

impl LcuSession {
    fn start_draft(&mut self) {
        self.draft_generation = self.draft_generation.saturating_add(1);
        self.draft_id = Some(format!("draft-{}", self.draft_generation));
    }

    pub fn apply(&mut self, event: LcuEvent) {
        self.revision = self.revision.saturating_add(1);
        match event {
            LcuEvent::Connected { .. } => {
                self.connected = true;
                self.account = None;
                self.phase = None;
                self.draft_id = None;
                self.draft = None;
                self.rune_page = None;
            }
            LcuEvent::Disconnected => {
                self.connected = false;
                self.account = None;
                self.phase = None;
                self.draft_id = None;
                self.draft = None;
                self.rune_page = None;
            }
            LcuEvent::AccountChanged { account } => {
                if self.connected {
                    self.account = account;
                }
            }
            LcuEvent::RunePageChanged { page } => {
                if self.connected {
                    self.rune_page = page;
                }
            }
            LcuEvent::DraftChanged { draft } => {
                if self.connected && self.phase == Some(GameflowPhase::ChampSelect) {
                    if draft.is_some() && self.draft_id.is_none() {
                        self.start_draft();
                    }
                    // Une lecture manquante ne marque pas une nouvelle draft.
                    // L'identité expire seulement à la sortie de phase/connexion.
                    self.draft = draft;
                }
            }
            LcuEvent::PhaseChanged { phase } => {
                if self.connected
                    && phase == GameflowPhase::ChampSelect
                    && self.phase != Some(phase)
                {
                    self.start_draft();
                }
                self.phase = Some(phase);
                if phase != GameflowPhase::ChampSelect {
                    self.draft = None;
                    self.draft_id = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifie_la_draft_avant_game_id_sans_changer_aux_evenements_repetes() {
        let mut session = LcuSession::default();
        session.apply(LcuEvent::Connected { port: 1 });
        session.apply(LcuEvent::PhaseChanged {
            phase: GameflowPhase::ChampSelect,
        });
        let before = serde_json::to_value(&session).unwrap()["draftId"].clone();
        assert!(before.as_str().is_some_and(|id| !id.is_empty()));
        session.apply(LcuEvent::PhaseChanged {
            phase: GameflowPhase::ChampSelect,
        });
        assert_eq!(serde_json::to_value(&session).unwrap()["draftId"], before);
        session.apply(LcuEvent::PhaseChanged {
            phase: GameflowPhase::Lobby,
        });
        assert!(serde_json::to_value(&session)
            .unwrap()
            .get("draftId")
            .is_none());
        session.apply(LcuEvent::PhaseChanged {
            phase: GameflowPhase::ChampSelect,
        });
        assert_ne!(serde_json::to_value(&session).unwrap()["draftId"], before);
    }
    #[test]
    fn une_lecture_de_draft_absente_ne_rearme_pas_les_imports() {
        let mut session = LcuSession::default();
        session.apply(LcuEvent::Connected { port: 1 });
        session.apply(LcuEvent::PhaseChanged {
            phase: GameflowPhase::ChampSelect,
        });
        let draft = DraftSession::parse(
            serde_json::from_str(include_str!("../tests/fixtures/champ-select-public.json"))
                .unwrap(),
        );
        session.apply(LcuEvent::DraftChanged {
            draft: draft.clone(),
        });
        let id = session.draft_id.clone();
        session.apply(LcuEvent::DraftChanged { draft: None });
        assert!(session.draft.is_none());
        assert_eq!(session.draft_id, id);
        session.apply(LcuEvent::DraftChanged { draft });
        assert_eq!(session.draft_id, id);
    }
    #[test]
    fn compte_actif_efface_a_la_deconnexion_et_evenement_tardif_ignore() {
        let account = crate::LcuAccount {
            platform: "EUW1".into(),
            game_name: "Alpha".into(),
            tag_line: "TEST".into(),
            profile_icon_id: None,
        };
        let mut session = LcuSession::default();
        session.apply(LcuEvent::Connected { port: 1 });
        session.apply(LcuEvent::AccountChanged {
            account: Some(account.clone()),
        });
        assert_eq!(session.account, Some(account.clone()));
        session.apply(LcuEvent::Disconnected);
        assert!(session.account.is_none());
        session.apply(LcuEvent::AccountChanged {
            account: Some(account),
        });
        assert!(session.account.is_none());
    }

    #[test]
    fn efface_les_runes_a_la_deconnexion_et_ignore_un_evenement_tardif() {
        let page = RunePage::parse(serde_json::json!({"primaryStyleId":8000,"subStyleId":8200,
            "selectedPerkIds":[],"isValid":false,"isTemporary":false}))
        .unwrap();
        let mut state = LcuSession::default();
        state.apply(LcuEvent::Connected { port: 1 });
        state.apply(LcuEvent::RunePageChanged {
            page: Some(page.clone()),
        });
        assert!(state.rune_page.is_some());
        state.apply(LcuEvent::Disconnected);
        assert!(state.rune_page.is_none());
        state.apply(LcuEvent::RunePageChanged { page: Some(page) });
        assert!(state.rune_page.is_none());
    }
    #[test]
    fn efface_la_draft_en_sortie_de_selection_et_ignore_une_mise_a_jour_tardive() {
        let draft = crate::DraftSession::parse(
            serde_json::from_str(include_str!("../tests/fixtures/champ-select-public.json"))
                .unwrap(),
        )
        .unwrap();
        let mut state = LcuSession::default();
        state.apply(LcuEvent::Connected { port: 1 });
        state.apply(LcuEvent::PhaseChanged {
            phase: GameflowPhase::ChampSelect,
        });
        state.apply(LcuEvent::DraftChanged {
            draft: Some(draft.clone()),
        });
        assert!(state.draft.is_some());
        state.apply(LcuEvent::PhaseChanged {
            phase: GameflowPhase::GameStart,
        });
        assert!(state.draft.is_none());
        state.apply(LcuEvent::DraftChanged { draft: Some(draft) });
        assert!(state.draft.is_none());
    }
    #[test]
    fn suit_la_session_et_efface_la_phase_a_la_deconnexion() {
        let mut state = LcuSession::default();
        state.apply(LcuEvent::Connected { port: 123 });
        state.apply(LcuEvent::PhaseChanged {
            phase: GameflowPhase::ChampSelect,
        });
        assert_eq!(state.phase, Some(GameflowPhase::ChampSelect));
        state.apply(LcuEvent::Disconnected);
        assert!(!state.connected);
        assert_eq!(state.phase, None);
        assert_eq!(state.revision, 3);
    }
    #[test]
    fn efface_la_draft_a_la_deconnexion_et_a_une_nouvelle_connexion() {
        let draft = crate::DraftSession::parse(
            serde_json::from_str(include_str!("../tests/fixtures/champ-select-public.json"))
                .unwrap(),
        )
        .unwrap();
        for event in [LcuEvent::Disconnected, LcuEvent::Connected { port: 2 }] {
            let mut state = LcuSession {
                connected: true,
                phase: Some(GameflowPhase::ChampSelect),
                draft: Some(draft.clone()),
                ..Default::default()
            };
            state.apply(event);
            assert!(state.draft.is_none());
            assert!(state.phase.is_none());
        }
    }
    #[test]
    fn serialise_uniquement_le_contrat_public() {
        let mut state = LcuSession::default();
        state.apply(LcuEvent::Connected { port: 123 });
        assert_eq!(
            serde_json::to_value(state).unwrap(),
            serde_json::json!({"revision":1,"connected":true,"account":null,"phase":null,"draft":null,"runePage":null})
        );
    }
}
