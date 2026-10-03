//! État partagé du lecteur et autorisation de ses transitions.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotlightSegment {
    pub kind: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotlightVideo {
    pub skin_id: u32,
    pub champion_id: u32,
    pub video_id: String,
    pub name: String,
    #[serde(default)]
    pub segments: Vec<SpotlightSegment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotlightState {
    pub revision: u64,
    pub video: Option<SpotlightVideo>,
    pub selected: String,
    pub detached: bool,
    pub locale: String,
}

impl Default for SpotlightState {
    fn default() -> Self {
        Self {
            revision: 0,
            video: None,
            selected: "full".into(),
            detached: false,
            locale: "fr".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SpotlightAction {
    Open {
        skin_id: u32,
        champion_id: u32,
        locale: String,
        kind: Option<String>,
    },
    Select {
        kind: String,
    },
    Step {
        direction: i8,
    },
    Detach,
    Attach,
    Close,
    Retry,
    External,
}

const SEGMENT_ORDER: [&str; 10] = [
    "passive", "q", "w", "e", "r", "emotes", "recall", "attack", "movement", "death",
];

fn catalog_video(skin_id: u32, champion_id: u32) -> Result<SpotlightVideo, String> {
    #[derive(Deserialize)]
    struct Catalog {
        entries: Vec<SpotlightVideo>,
    }
    let catalog: Catalog =
        serde_json::from_str(include_str!("../../public/game-data/skin-spotlights.json"))
            .map_err(|_| "spotlight_catalog_invalid")?;
    let mut video = catalog
        .entries
        .into_iter()
        .find(|entry| entry.skin_id == skin_id && entry.champion_id == champion_id)
        .ok_or("spotlight_video_unavailable")?;
    // Les données restent issues du catalogue embarqué ; une entrée mal formée
    // ne doit jamais devenir une URL ou un passage fourni par l'interface.
    if video.video_id.len() != 11
        || !video
            .video_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        || video.segments.len() > SEGMENT_ORDER.len()
    {
        return Err("spotlight_catalog_invalid".into());
    }
    let mut kinds = std::collections::HashSet::new();
    for segment in &video.segments {
        if !SEGMENT_ORDER.contains(&segment.kind.as_str())
            || !kinds.insert(segment.kind.clone())
            || segment.start >= segment.end
            || segment.end > 7200
        {
            return Err("spotlight_catalog_invalid".into());
        }
    }
    video
        .segments
        .sort_by_key(|segment| SEGMENT_ORDER.iter().position(|kind| *kind == segment.kind));
    Ok(video)
}

fn selection_exists(video: &SpotlightVideo, kind: &str) -> bool {
    kind == "full" || video.segments.iter().any(|segment| segment.kind == kind)
}

pub fn transition(
    current: &SpotlightState,
    action: SpotlightAction,
    caller: &str,
    expected_revision: u64,
) -> Result<SpotlightState, String> {
    let owner = if current.detached {
        "skin-spotlight-controls"
    } else {
        "main"
    };
    let authorized = if matches!(&action, SpotlightAction::Open { .. }) {
        caller == "main"
    } else {
        caller == owner
    };
    if !authorized {
        return Err("spotlight_forbidden".into());
    }
    if expected_revision != current.revision {
        return Err("spotlight_stale_revision".into());
    }
    if current.video.is_none() && !matches!(&action, SpotlightAction::Open { .. }) {
        return Err("spotlight_not_open".into());
    }
    let mut next = current.clone();
    next.revision = current
        .revision
        .checked_add(1)
        .ok_or("spotlight_revision_overflow")?;
    match action {
        SpotlightAction::Open {
            skin_id,
            champion_id,
            locale,
            kind,
        } => {
            if locale != "fr" && locale != "en" {
                return Err("spotlight_invalid_locale".into());
            }
            let video = catalog_video(skin_id, champion_id)?;
            let selected = kind.unwrap_or_else(|| "full".into());
            if !selection_exists(&video, &selected) {
                return Err("spotlight_invalid_segment".into());
            }
            next.video = Some(video);
            next.selected = selected;
            next.detached = false;
            next.locale = locale;
        }
        SpotlightAction::Select { kind } => {
            let video = current.video.as_ref().ok_or("spotlight_not_open")?;
            if !selection_exists(video, &kind) {
                return Err("spotlight_invalid_segment".into());
            }
            next.selected = kind;
        }
        SpotlightAction::Step { direction } => {
            if direction != -1 && direction != 1 {
                return Err("spotlight_invalid_direction".into());
            }
            let video = current.video.as_ref().ok_or("spotlight_not_open")?;
            let choices: Vec<&str> = std::iter::once("full")
                .chain(video.segments.iter().map(|segment| segment.kind.as_str()))
                .collect();
            let index = choices
                .iter()
                .position(|kind| *kind == current.selected)
                .ok_or("spotlight_invalid_segment")?;
            let offset = if direction > 0 { 1 } else { choices.len() - 1 };
            next.selected = choices[(index + offset) % choices.len()].into();
        }
        SpotlightAction::Detach => next.detached = true,
        SpotlightAction::Attach => next.detached = false,
        SpotlightAction::Close | SpotlightAction::External => {
            next.video = None;
            next.selected = "full".into();
            next.detached = false;
        }
        SpotlightAction::Retry => {}
    }
    Ok(next)
}

pub fn selected_segment(state: &SpotlightState) -> Option<&SpotlightSegment> {
    state
        .video
        .as_ref()?
        .segments
        .iter()
        .find(|segment| segment.kind == state.selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened() -> SpotlightState {
        transition(
            &SpotlightState::default(),
            SpotlightAction::Open {
                skin_id: 103007,
                champion_id: 103,
                locale: "fr".into(),
                kind: None,
            },
            "main",
            0,
        )
        .unwrap()
    }

    fn apply(state: &SpotlightState, action: SpotlightAction, caller: &str) -> SpotlightState {
        transition(state, action, caller, state.revision).unwrap()
    }

    #[test]
    fn ouverture_utilise_le_catalogue_et_le_contrat_camel_case() {
        let state = opened();
        let video = state.video.as_ref().unwrap();
        assert_eq!(video.video_id, "IPU9_WRcsj4");
        assert_eq!(state.selected, "full");
        assert_eq!(state.revision, 1);
        assert_eq!(video.segments[0].kind, "passive");
        let encoded = serde_json::to_value(&state).unwrap();
        assert_eq!(encoded["video"]["skinId"], 103007);
        let action: SpotlightAction = serde_json::from_value(serde_json::json!({
            "type": "open", "skinId": 103007, "championId": 103, "locale": "en", "kind": "q"
        }))
        .unwrap();
        let next = transition(&state, action, "main", 1).unwrap();
        assert_eq!(next.locale, "en");
        assert_eq!(selected_segment(&next).unwrap().start, 88);
    }

    #[test]
    fn refuse_revision_perimee_et_ne_modifie_pas_l_etat() {
        let state = opened();
        assert_eq!(
            transition(&state, SpotlightAction::Close, "main", 0),
            Err("spotlight_stale_revision".into())
        );
        assert!(state.video.is_some());
        assert_eq!(state.revision, 1);
    }

    #[test]
    fn refuse_toujours_le_contenu_distant_et_limite_au_proprietaire() {
        let state = opened();
        for caller in [
            "skin-spotlight",
            "skin-spotlight-media",
            "unknown",
            "skin-spotlight-controls",
        ] {
            assert_eq!(
                transition(&state, SpotlightAction::Retry, caller, 1),
                Err("spotlight_forbidden".into())
            );
        }
        let detached = apply(&state, SpotlightAction::Detach, "main");
        assert_eq!(
            transition(&detached, SpotlightAction::Close, "main", 2),
            Err("spotlight_forbidden".into())
        );
        let retry = apply(&detached, SpotlightAction::Retry, "skin-spotlight-controls");
        assert_eq!(retry.revision, 3);
        assert_eq!(
            transition(
                &detached,
                SpotlightAction::Open {
                    skin_id: 103007,
                    champion_id: 103,
                    locale: "fr".into(),
                    kind: None
                },
                "skin-spotlight-controls",
                2
            ),
            Err("spotlight_forbidden".into())
        );
    }

    #[test]
    fn refuse_skin_inconnu_champion_inexact_locale_et_selection_invalides() {
        for (skin_id, champion_id, locale, kind) in [
            (9999999, 103, "fr", None),
            (103007, 1, "fr", None),
            (103007, 103, "de", None),
            (103007, 103, "fr", Some("missing")),
        ] {
            assert!(transition(
                &SpotlightState::default(),
                SpotlightAction::Open {
                    skin_id,
                    champion_id,
                    locale: locale.into(),
                    kind: kind.map(str::to_string),
                },
                "main",
                0
            )
            .is_err());
        }
        let state = opened();
        assert!(transition(
            &state,
            SpotlightAction::Select {
                kind: "missing".into()
            },
            "main",
            1
        )
        .is_err());
        assert!(transition(&state, SpotlightAction::Step { direction: 2 }, "main", 1).is_err());
        assert!(transition(&state, SpotlightAction::Step { direction: 0 }, "main", 1).is_err());
    }

    #[test]
    fn navigation_boucle_dans_l_ordre_interface_et_non_chronologique() {
        let mut state = opened();
        let expected = [
            "passive", "q", "w", "e", "r", "emotes", "recall", "attack", "death", "full",
        ];
        for kind in expected {
            state = apply(&state, SpotlightAction::Step { direction: 1 }, "main");
            assert_eq!(state.selected, kind);
        }
        state = apply(&state, SpotlightAction::Step { direction: -1 }, "main");
        assert_eq!(state.selected, "death");
    }

    #[test]
    fn detacher_rattacher_preserve_le_passage_et_fermer_reinitialise() {
        let selected = apply(
            &opened(),
            SpotlightAction::Select { kind: "q".into() },
            "main",
        );
        let detached = apply(&selected, SpotlightAction::Detach, "main");
        assert!(detached.detached);
        assert_eq!(selected_segment(&detached).unwrap().start, 88);
        let attached = apply(
            &detached,
            SpotlightAction::Attach,
            "skin-spotlight-controls",
        );
        assert!(!attached.detached);
        assert_eq!(attached.video, selected.video);
        assert_eq!(attached.selected, "q");
        let closed = apply(&attached, SpotlightAction::Close, "main");
        assert!(closed.video.is_none());
        assert!(!closed.detached);
        assert_eq!(closed.selected, "full");
        assert_eq!(closed.revision, attached.revision + 1);
        assert_eq!(selected_segment(&closed), None);
        assert!(transition(&closed, SpotlightAction::Retry, "main", closed.revision).is_err());
    }

    #[test]
    fn ouvrir_depuis_main_reprend_la_propriete_et_reinitialise_le_passage() {
        let detached = apply(&opened(), SpotlightAction::Detach, "main");
        let next = apply(
            &detached,
            SpotlightAction::Open {
                skin_id: 103007,
                champion_id: 103,
                locale: "en".into(),
                kind: None,
            },
            "main",
        );
        assert!(!next.detached);
        assert_eq!(next.selected, "full");
        assert_eq!(next.locale, "en");
    }
    #[test]
    fn catalogue_sans_passages_ne_cree_aucun_chapitre() {
        #[derive(Deserialize)]
        struct Catalog {
            entries: Vec<SpotlightVideo>,
        }
        let catalog: Catalog =
            serde_json::from_str(include_str!("../../public/game-data/skin-spotlights.json"))
                .unwrap();
        let source = catalog
            .entries
            .iter()
            .find(|video| video.segments.is_empty())
            .unwrap();
        let state = transition(
            &SpotlightState::default(),
            SpotlightAction::Open {
                skin_id: source.skin_id,
                champion_id: source.champion_id,
                locale: "en".into(),
                kind: None,
            },
            "main",
            0,
        )
        .unwrap();
        assert_eq!(state.video.as_ref().unwrap().segments.len(), 0);
        for direction in [-1, 1] {
            let stepped = apply(&state, SpotlightAction::Step { direction }, "main");
            assert_eq!(stepped.selected, "full");
            assert!(selected_segment(&stepped).is_none());
        }
        assert!(transition(
            &state,
            SpotlightAction::Select { kind: "q".into() },
            "main",
            state.revision
        )
        .is_err());
    }

    #[test]
    fn ouvrir_exige_aussi_la_revision_courante() {
        let state = opened();
        assert_eq!(
            transition(
                &state,
                SpotlightAction::Open {
                    skin_id: 103007,
                    champion_id: 103,
                    locale: "en".into(),
                    kind: None,
                },
                "main",
                0
            ),
            Err("spotlight_stale_revision".into())
        );
    }
    #[test]
    fn repli_externe_exige_une_session_et_son_proprietaire() {
        let external: SpotlightAction =
            serde_json::from_value(serde_json::json!({"type": "external"})).unwrap();
        let empty = SpotlightState::default();
        assert_eq!(
            transition(&empty, external.clone(), "main", 0),
            Err("spotlight_not_open".into())
        );
        let selected = apply(
            &opened(),
            SpotlightAction::Select { kind: "q".into() },
            "main",
        );
        let attached = apply(&selected, external.clone(), "main");
        assert!(attached.video.is_none());
        assert_eq!(attached.selected, "full");
        assert_eq!(attached.revision, selected.revision + 1);
        let detached = apply(&selected, SpotlightAction::Detach, "main");
        let exported = apply(&detached, external.clone(), "skin-spotlight-controls");
        assert!(exported.video.is_none());
        assert_eq!(exported.selected, "full");
        assert!(!exported.detached);
        for caller in ["main", "skin-spotlight-media", "skin-spotlight", "unknown"] {
            assert_eq!(
                transition(&detached, external.clone(), caller, detached.revision),
                Err("spotlight_forbidden".into())
            );
        }
    }
}
