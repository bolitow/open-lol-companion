mod imports;
use lcu_connector::LcuSession;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

/// Réponse de `lcu_status`, miroir du type `LcuStatus` de `@olc/shared`.
/// Le mot de passe du client ne quitte jamais le cœur Rust.
#[derive(Serialize)]
struct LcuStatus {
    connected: bool,
    port: Option<u16>,
    message: String,
}

#[tauri::command]
fn lcu_status() -> LcuStatus {
    match lcu_connector::discover() {
        Ok(creds) => LcuStatus {
            connected: true,
            port: Some(creds.port),
            message: format!("API locale sur {}", creds.base_url()),
        },
        Err(e) => LcuStatus {
            connected: false,
            port: None,
            message: e.to_string(),
        },
    }
}

type SessionState = Arc<Mutex<LcuSession>>;

type BuildState = Result<olc_build_client::BuildClient, olc_build_client::BuildError>;

#[tauri::command]
async fn community_builds(
    request: olc_build_client::BuildRequest,
    state: tauri::State<'_, BuildState>,
) -> Result<olc_build_client::BuildReport, olc_build_client::BuildError> {
    match state.inner() {
        Ok(client) => client.builds(request).await,
        Err(error) => Err(*error),
    }
}

#[tauri::command]
fn lcu_session(state: tauri::State<'_, SessionState>) -> Result<LcuSession, &'static str> {
    state
        .lock()
        .map(|session| session.clone())
        .map_err(|_| "session_unavailable")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(LcuSession::default())))
        .manage(olc_build_client::BuildClient::from_env())
        .manage(imports::ImportLocks::default())
        .setup(|app| {
            let state = app.state::<SessionState>().inner().clone();
            let handle = app.handle().clone();
            let (tx, mut rx) = tokio::sync::mpsc::channel(32);
            tauri::async_runtime::spawn(lcu_connector::watch(tx));
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let snapshot = match state.lock() {
                        Ok(mut session) => {
                            session.apply(event);
                            session.clone()
                        }
                        Err(_) => break,
                    };
                    // L'état courant reste lisible si aucune fenêtre n'écoute encore.
                    let _ = handle.emit("lcu-session", snapshot);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            lcu_status,
            lcu_session,
            community_builds,
            imports::import_runes,
            imports::import_draft_runes,
            imports::import_spells,
            imports::import_draft_spells,
            imports::import_items
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application");
}
