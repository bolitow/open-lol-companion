//! Annonce des publications du serveur : état lisible par l'interface et événement à chaque changement.
use olc_build_client::{
    publications::{PublicationEvent, PublicationState, PublicationStatus, ReconnectPolicy},
    BuildClient,
};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

/// Événement émis vers la fenêtre principale, miroir de `PUBLICATION_STATE_EVENT` dans @olc/shared.
const STATE_EVENT: &str = "publication-state";

/// Génération du flux en cours et état visible : un flux remplacé ne modifie plus l'état.
#[derive(Default)]
pub struct PublicationsState(Mutex<(u64, PublicationState)>);

/// État courant, pour une fenêtre qui s'ouvre après la dernière notification.
#[tauri::command]
pub fn publication_state(
    state: tauri::State<'_, PublicationsState>,
) -> Result<PublicationState, &'static str> {
    state
        .0
        .lock()
        .map(|state| state.1.clone())
        .map_err(|_| "state_unavailable")
}

/// Applique un événement du flux `generation` ; renvoie `true` si l'état visible a changé.
fn accept(current: &mut (u64, PublicationState), generation: u64, event: PublicationEvent) -> bool {
    current.0 == generation && current.1.apply(event)
}

/// Ouvre une nouvelle génération ; `revision` reste croissante pour que l'interface relise
/// ses builds à la prochaine publication du nouveau serveur. Renvoie la génération et si
/// l'état visible a changé.
fn reset(current: &mut (u64, PublicationState)) -> (u64, bool) {
    current.0 = current.0.wrapping_add(1);
    let changed =
        current.1.status != PublicationStatus::NotConfigured || current.1.publication.is_some();
    current.1.status = PublicationStatus::NotConfigured;
    current.1.publication = None;
    (current.0, changed)
}

fn apply(app: &tauri::AppHandle, generation: u64, event: PublicationEvent) {
    let state = app.state::<PublicationsState>();
    let Ok(mut current) = state.0.lock() else {
        return;
    };
    if accept(&mut current, generation, event) {
        let _ = app.emit_to("main", STATE_EVENT, current.1.clone());
    }
}

/// L'état reste `not_configured` jusqu'au chargement de l'accès API (`api_access::setup`).
pub fn setup(app: &tauri::AppHandle) {
    app.manage(PublicationsState::default());
}

/// Repart d'une connexion vierge pour le client donné ; l'ancien flux ne modifie plus l'état.
pub fn restart(
    app: &tauri::AppHandle,
    client: Option<Arc<BuildClient>>,
) -> Option<tauri::async_runtime::JoinHandle<()>> {
    let generation = {
        let state = app.state::<PublicationsState>();
        let mut current = state.0.lock().ok()?;
        let (generation, changed) = reset(&mut current);
        if changed {
            let _ = app.emit_to("main", STATE_EVENT, current.1.clone());
        }
        generation
    };
    let client = client?;
    let app = app.clone();
    Some(tauri::async_runtime::spawn(async move {
        client
            .watch_publications(ReconnectPolicy::default(), |event| {
                apply(&app, generation, event)
            })
            .await;
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn la_commande_et_l_evenement_sont_declares_comme_dans_le_miroir_typescript() {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
        let overlay: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/overlay.json")).unwrap();
        let permission = serde_json::json!("allow-publication-state");
        assert!(capability["permissions"]
            .as_array()
            .unwrap()
            .contains(&permission));
        assert!(!overlay["permissions"]
            .as_array()
            .unwrap()
            .contains(&permission));
        assert!(include_str!("../build.rs").contains("\"publication_state\""));
        assert!(
            include_str!("../../../../packages/shared/src/publications.ts")
                .contains(&format!("'{}'", super::STATE_EVENT))
        );
    }
}

#[cfg(test)]
mod restart_tests {
    use super::*;
    use olc_build_client::publications::Publication;

    fn publication(stats: &str) -> PublicationEvent {
        PublicationEvent::Received(Publication {
            kind: "publication".into(),
            stats_version: Some(stats.into()),
            static_version: None,
            available: true,
        })
    }

    #[test]
    fn un_flux_remplace_ne_modifie_plus_l_etat() {
        let mut current = (0, PublicationState::default());
        let (old, _) = reset(&mut current);
        assert!(accept(&mut current, old, publication("v1")));
        let (new, changed) = reset(&mut current);
        assert!(changed);
        assert!(!accept(&mut current, old, publication("v2")));
        assert_eq!(current.1.status, PublicationStatus::NotConfigured);
        assert!(accept(&mut current, new, publication("v2")));
    }

    #[test]
    fn la_remise_a_zero_garde_la_revision_croissante() {
        let mut current = (0, PublicationState::default());
        let (generation, changed) = reset(&mut current);
        assert!(!changed);
        accept(&mut current, generation, publication("v1"));
        assert_eq!(current.1.revision, 1);
        let (next, changed) = reset(&mut current);
        assert!(changed);
        assert_eq!(next, generation + 1);
        assert_eq!(current.1.revision, 1);
        assert_eq!(current.1.publication, None);
        assert_eq!(current.1.status, PublicationStatus::NotConfigured);
        accept(&mut current, next, publication("v1"));
        assert_eq!(current.1.revision, 2);
    }
}
