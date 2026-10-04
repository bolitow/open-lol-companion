//! Cycle de navigation native ; `Loaded` indique une page chargée, jamais une lecture média.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaStatus {
    #[default]
    Idle,
    Loading,
    Loaded,
    Slow,
    Failed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub attempt: u64,
    pub status: MediaStatus,
}

#[derive(Default)]
pub struct MediaLifecycle {
    state: MediaState,
    url: Option<String>,
}

impl MediaLifecycle {
    /// Une même URL conserve son chargement, notamment lors d'un déplacement natif.
    pub fn begin(&mut self, url: &str) -> bool {
        if self.url.as_deref() == Some(url) {
            return false;
        }
        let Some(attempt) = self.state.attempt.checked_add(1) else {
            // Ne jamais réutiliser une génération après épuisement du compteur.
            self.url = None;
            self.state.status = MediaStatus::Idle;
            return false;
        };
        self.state = MediaState {
            attempt,
            status: MediaStatus::Loading,
        };
        self.url = Some(url.to_owned());
        true
    }

    /// Fermer ou réessayer invalide également les événements natifs déjà en vol.
    pub fn invalidate(&mut self) {
        self.state.attempt = self.state.attempt.saturating_add(1);
        self.state.status = MediaStatus::Idle;
        self.url = None;
    }

    pub fn update(&mut self, attempt: u64, status: MediaStatus) -> bool {
        if attempt != self.state.attempt {
            return false;
        }
        let allowed = matches!(
            (self.state.status, status),
            (
                MediaStatus::Loading,
                MediaStatus::Slow | MediaStatus::Loaded | MediaStatus::Failed
            ) | (MediaStatus::Slow, MediaStatus::Loaded | MediaStatus::Failed)
        );
        if allowed {
            self.state.status = status;
        }
        allowed
    }

    pub fn current(&self) -> MediaState {
        self.state.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etat_initial_et_contrat_json_sont_explicites() {
        let media = MediaLifecycle::default();
        assert_eq!(
            serde_json::to_value(media.current()).unwrap(),
            serde_json::json!({"attempt": 0, "status": "idle"})
        );
        for (status, value) in [
            (MediaStatus::Loading, "loading"),
            (MediaStatus::Loaded, "loaded"),
            (MediaStatus::Slow, "slow"),
            (MediaStatus::Failed, "failed"),
        ] {
            assert_eq!(serde_json::to_value(status).unwrap(), value);
        }
    }

    #[test]
    fn compteur_epuise_ne_reutilise_aucune_tentative() {
        let mut media = MediaLifecycle {
            state: MediaState {
                attempt: u64::MAX,
                status: MediaStatus::Loading,
            },
            url: Some("A".into()),
        };
        media.invalidate();
        assert!(!media.begin("A"));
        assert!(!media.update(u64::MAX, MediaStatus::Loaded));
        assert_eq!(media.current().status, MediaStatus::Idle);
    }

    #[test]
    fn retry_invalide_les_evenements_de_l_ancienne_tentative() {
        let mut media = MediaLifecycle::default();
        assert!(media.begin("A"));
        let previous = media.current().attempt;
        media.invalidate();
        assert!(media.begin("A"));
        assert!(!media.update(previous, MediaStatus::Loaded));
        assert_eq!(media.current().status, MediaStatus::Loading);
        assert!(media.current().attempt > previous);
    }

    #[test]
    fn aller_retour_sur_une_url_ne_reutilise_pas_la_generation() {
        let mut media = MediaLifecycle::default();
        media.begin("A");
        let first = media.current().attempt;
        media.begin("B");
        let second = media.current().attempt;
        media.begin("A");
        let third = media.current().attempt;
        assert!(first < second && second < third);
        assert!(!media.update(first, MediaStatus::Failed));
        assert!(!media.update(second, MediaStatus::Loaded));
    }

    #[test]
    fn detacher_et_repositionner_la_meme_url_preserve_la_navigation() {
        let mut media = MediaLifecycle::default();
        media.begin("A");
        let attempt = media.current().attempt;
        assert!(media.update(attempt, MediaStatus::Loaded));
        assert!(!media.begin("A"));
        assert_eq!(
            media.current(),
            MediaState {
                attempt,
                status: MediaStatus::Loaded
            }
        );
    }

    #[test]
    fn delai_lent_n_est_pas_un_echec_et_peut_finir_charge() {
        let mut media = MediaLifecycle::default();
        media.begin("A");
        let attempt = media.current().attempt;
        assert!(media.update(attempt, MediaStatus::Slow));
        assert!(!media.update(attempt, MediaStatus::Slow));
        assert!(media.update(attempt, MediaStatus::Loaded));
        assert!(!media.update(attempt, MediaStatus::Slow));
        assert!(!media.update(attempt, MediaStatus::Failed));
    }

    #[test]
    fn fermeture_ignore_tout_evenement_tardif() {
        let mut media = MediaLifecycle::default();
        media.begin("A");
        let attempt = media.current().attempt;
        media.invalidate();
        for status in [MediaStatus::Loaded, MediaStatus::Slow, MediaStatus::Failed] {
            assert!(!media.update(attempt, status));
            assert!(!media.update(media.current().attempt, status));
        }
        assert_eq!(media.current().status, MediaStatus::Idle);
    }

    #[test]
    fn echec_terminal_et_transitions_non_autorisees_sont_ignores() {
        let mut media = MediaLifecycle::default();
        media.begin("A");
        let attempt = media.current().attempt;
        assert!(!media.update(attempt, MediaStatus::Idle));
        assert!(!media.update(attempt, MediaStatus::Loading));
        assert!(media.update(attempt, MediaStatus::Failed));
        assert!(!media.update(attempt, MediaStatus::Loaded));
        media.invalidate();
        media.begin("A");
        let retry = media.current().attempt;
        assert!(media.update(retry, MediaStatus::Slow));
        assert!(media.update(retry, MediaStatus::Failed));
    }
}
