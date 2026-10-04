//! Annonce des publications du serveur : état lisible par l'interface et événement à chaque changement.
use olc_build_client::publications::{PublicationEvent, PublicationState, ReconnectPolicy};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// Événement émis vers la fenêtre principale, miroir de `PUBLICATION_STATE_EVENT` dans @olc/shared.
const STATE_EVENT: &str = "publication-state";

#[derive(Default)]
pub struct PublicationsState(Mutex<PublicationState>);

/// État courant, pour une fenêtre qui s'ouvre après la dernière notification.
#[tauri::command]
pub fn publication_state(
    state: tauri::State<'_, PublicationsState>,
) -> Result<PublicationState, &'static str> {
    state
        .0
        .lock()
        .map(|state| state.clone())
        .map_err(|_| "state_unavailable")
}

fn apply(app: &tauri::AppHandle, event: PublicationEvent) {
    let state = app.state::<PublicationsState>();
    let Ok(mut current) = state.0.lock() else {
        return;
    };
    if current.apply(event) {
        let _ = app.emit_to("main", STATE_EVENT, current.clone());
    }
}

/// Lance l'écoute si le service est configuré ; sinon l'état reste `not_configured`.
pub fn setup(app: &tauri::AppHandle) {
    app.manage(PublicationsState::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let builds = app.state::<super::BuildState>();
        let Ok(client) = builds.inner() else {
            return;
        };
        client
            .watch_publications(ReconnectPolicy::default(), |event| apply(&app, event))
            .await;
    });
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
