use lcu_connector::{
    imports::{AutoImportContext, AutoImportReceipt},
    GameflowPhase, LcuSession,
};

pub(super) fn matches_selection(
    session: &LcuSession,
    context: &AutoImportContext,
    custom_role: Option<&str>,
) -> bool {
    if !session.connected
        || session.phase != Some(GameflowPhase::ChampSelect)
        || session.draft_id.as_deref() != Some(&context.draft_id)
        || !lcu_connector::live::valid_role(&context.role)
    {
        return false;
    }
    let Some(draft) = &session.draft else {
        return false;
    };
    let mut own = draft.allies.iter().filter(|p| p.local);
    let matches = own.next().is_some_and(|p| {
        p.champion_id == Some(context.champion_id)
            && match p.position.as_deref() {
                Some(role) => role.eq_ignore_ascii_case(&context.role),
                None => draft.custom_game && custom_role == Some(context.role.as_str()),
            }
    });
    draft.supported
        && draft.queue_id == Some(context.queue_id)
        && matches
        && own.next().is_none()
        && context
            .game_id
            .as_ref()
            .map_or(true, |id| draft.game_id.as_ref() == Some(id))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Runes,
    Items,
    Spells,
}

type Selection = (Option<u32>, bool, bool, Vec<(Option<u32>, Option<String>)>);
#[derive(Default)]
pub(super) struct History {
    pub epoch: u64,
    pub receipts: Vec<(AutoImportContext, Kind, AutoImportReceipt)>,
    pub custom_role: Option<String>,
    draft_id: Option<String>,
    selection: Option<Selection>,
}
impl History {
    pub fn receipt(&self, context: &AutoImportContext, kind: Kind) -> Option<AutoImportReceipt> {
        self.receipts
            .iter()
            .find(|(c, k, _)| c.same_selection(context) && *k == kind)
            .map(|(_, _, r)| *r)
    }
    pub fn record(&mut self, context: AutoImportContext, kind: Kind, receipt: AutoImportReceipt) {
        self.receipts.retain(|(_, k, _)| *k != kind);
        self.receipts.push((context, kind, receipt));
    }

    pub fn set_custom_role(&mut self, role: Option<String>) {
        if self.custom_role != role {
            self.custom_role = role;
            if self
                .selection
                .as_ref()
                .is_some_and(|(_, custom, _, players)| {
                    *custom && players.iter().any(|(_, role)| role.is_none())
                })
            {
                self.invalidate();
            }
        }
    }
    fn invalidate(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        self.receipts.clear();
    }
    pub fn observe(&mut self, session: &LcuSession) {
        let id = session
            .draft_id
            .clone()
            .filter(|_| session.connected && session.phase == Some(GameflowPhase::ChampSelect));
        if self.draft_id != id {
            self.invalidate();
            self.draft_id = id;
            self.selection = None;
        }
        if self.draft_id.is_none() {
            return;
        }
        // Une lecture manquante ne prouve pas une nouvelle sélection ; elle bloque seulement l'écriture.
        let Some(draft) = &session.draft else {
            return;
        };
        let selection = (
            draft.queue_id,
            draft.custom_game,
            draft.supported,
            draft
                .allies
                .iter()
                .filter(|p| p.local)
                .map(|p| {
                    (
                        p.champion_id,
                        p.position.as_ref().map(|r| r.to_ascii_uppercase()),
                    )
                })
                .collect(),
        );
        if self.selection.as_ref() != Some(&selection) {
            self.invalidate();
            self.selection = Some(selection);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lcu_connector::{DraftSession, LcuEvent};
    use serde_json::json;
    fn session() -> LcuSession {
        let mut session = LcuSession::default();
        session.apply(LcuEvent::Connected { port: 1 });
        session.apply(LcuEvent::PhaseChanged {
            phase: GameflowPhase::ChampSelect,
        });
        let draft = DraftSession::parse(json!({"gameId":123,"localPlayerCellId":0,
          "myTeam":[{"cellId":0,"championId":432,"assignedPosition":"utility"}],
          "theirTeam":[],"actions":[]}))
        .unwrap();
        session.draft = Some(DraftSession {
            supported: true,
            queue_id: Some(420),
            ..draft
        });
        session
    }
    fn context(s: &LcuSession) -> AutoImportContext {
        AutoImportContext {
            draft_id: s.draft_id.clone().unwrap(),
            game_id: Some("123".into()),
            champion_id: 432,
            role: "UTILITY".into(),
            queue_id: 420,
        }
    }
    #[test]
    fn le_cache_d_objets_ne_masque_pas_les_sorts_et_chaque_recu_reste_independant() {
        let s = session();
        let c = context(&s);
        let mut h = History::default();
        h.observe(&s);
        h.record(
            c.clone(),
            Kind::Items,
            AutoImportReceipt { confirmed: true },
        );
        assert_eq!(h.receipt(&c, Kind::Spells), None);
        h.record(
            c.clone(),
            Kind::Spells,
            AutoImportReceipt { confirmed: false },
        );
        h.record(
            c.clone(),
            Kind::Runes,
            AutoImportReceipt { confirmed: true },
        );
        assert_eq!(
            h.receipt(&c, Kind::Items),
            Some(AutoImportReceipt { confirmed: true })
        );
        assert_eq!(
            h.receipt(&c, Kind::Spells),
            Some(AutoImportReceipt { confirmed: false })
        );
        h.record(
            c.clone(),
            Kind::Spells,
            AutoImportReceipt { confirmed: true },
        );
        assert_eq!(h.receipts.len(), 3);
        assert_eq!(
            h.receipt(&c, Kind::Spells),
            Some(AutoImportReceipt { confirmed: true })
        );
    }
    #[test]
    fn le_poste_manuel_invalide_le_cache_et_les_imports_en_vol() {
        let mut s = session();
        s.draft.as_mut().unwrap().custom_game = true;
        s.draft.as_mut().unwrap().allies[0].position = None;
        let c = context(&s);
        let mut h = History::default();
        h.observe(&s);
        h.set_custom_role(Some("UTILITY".into()));
        let before = h.epoch;
        h.receipts.push((
            c.clone(),
            Kind::Runes,
            AutoImportReceipt { confirmed: true },
        ));
        h.set_custom_role(Some("MIDDLE".into()));
        assert!(h.receipts.is_empty());
        assert!(h.epoch > before);
        assert!(!matches_selection(&s, &c, h.custom_role.as_deref()));
        h.set_custom_role(Some("UTILITY".into()));
        assert!(h.receipts.is_empty());
        assert!(matches_selection(&s, &c, h.custom_role.as_deref()));
    }
    #[test]
    fn une_reponse_http_ancienne_ne_survit_pas_au_champion_role_ou_mode_courant() {
        let s = session();
        let c = context(&s);
        assert!(matches_selection(&s, &c, None));
        for mut changed in [s.clone(), s.clone(), s.clone(), s.clone(), s.clone()]
            .into_iter()
            .enumerate()
        {
            let d = changed.1.draft.as_mut().unwrap();
            match changed.0 {
                0 => d.allies[0].champion_id = Some(103),
                1 => d.allies[0].position = Some("MIDDLE".into()),
                2 => d.queue_id = Some(440),
                3 => d.supported = false,
                _ => d.game_id = Some("456".into()),
            };
            assert!(!matches_selection(&changed.1, &c, None));
        }
    }
    #[test]
    fn passage_par_un_autre_champion_sans_import_rearme_le_premier_et_invalide_les_reponses_tardives(
    ) {
        let mut s = session();
        let c = context(&s);
        let mut history = History::default();
        history.observe(&s);
        let before = history.epoch;
        history
            .receipts
            .push((c, Kind::Runes, AutoImportReceipt { confirmed: true }));
        s.draft.as_mut().unwrap().allies[0].champion_id = Some(103);
        history.observe(&s);
        s.draft.as_mut().unwrap().allies[0].champion_id = Some(432);
        history.observe(&s);
        assert!(history.receipts.is_empty());
        assert!(history.epoch > before);
    }
    #[test]
    fn lecture_transitoire_absente_et_verrouillage_ne_rearment_pas() {
        let mut s = session();
        let c = context(&s);
        let mut history = History::default();
        history.observe(&s);
        let before = history.epoch;
        history
            .receipts
            .push((c, Kind::Runes, AutoImportReceipt { confirmed: true }));
        let draft = s.draft.take();
        history.observe(&s);
        s.draft = draft;
        s.draft.as_mut().unwrap().allies[0].locked = true;
        history.observe(&s);
        assert_eq!(history.epoch, before);
        assert_eq!(history.receipts.len(), 1);
        s.apply(LcuEvent::Disconnected);
        history.observe(&s);
        assert!(history.receipts.is_empty());
    }
}
