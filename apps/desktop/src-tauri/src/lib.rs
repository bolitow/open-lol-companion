mod api_access;
mod desktop;
mod diagnostics;
mod friends;
mod imports;
mod live;
mod overlay;
mod players;
mod publications;
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

#[tauri::command]
async fn community_builds(
    request: olc_build_client::BuildRequest,
    state: tauri::State<'_, api_access::ApiState>,
) -> Result<olc_build_client::BuildReport, olc_build_client::BuildError> {
    state.client().await?.builds(request).await
}

#[tauri::command]
fn lcu_session(state: tauri::State<'_, SessionState>) -> Result<LcuSession, &'static str> {
    state
        .lock()
        .map(|session| session.clone())
        .map_err(|_| "session_unavailable")
}

/// Version du jeu installée, lue à la demande dans le client local (#93).
#[tauri::command]
async fn client_patch() -> Result<lcu_connector::ClientPatch, lcu_connector::ClientPatchError> {
    let client = players::client()
        .await
        .map_err(|_| lcu_connector::ClientPatchError::Unavailable)?;
    lcu_connector::read_client_patch(&client).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg("--autostart")
                .build(),
        )
        .plugin(tauri_plugin_dialog::init());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    builder
        .manage(diagnostics::DiagnosticsState::default())
        .manage(Arc::new(Mutex::new(LcuSession::default())))
        .manage(api_access::ApiState::default())
        .manage(imports::ImportLocks::default())
        .manage(players::LocalState::default())
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    let state = window.state::<desktop::DesktopState>();
                    if olc_desktop_support::should_hide_on_close(
                        state
                            .close_to_tray
                            .load(std::sync::atomic::Ordering::Relaxed),
                        state
                            .tray_available
                            .load(std::sync::atomic::Ordering::Relaxed),
                    ) && window.hide().is_ok()
                    {
                        api.prevent_close();
                    }
                }
                tauri::WindowEvent::Destroyed => window.app_handle().exit(0),
                _ => {}
            }
        })
        .setup(|app| {
            desktop::setup(app.handle())?;
            overlay::setup(app.handle());
            live::setup(app.handle());
            friends::setup(app.handle());
            publications::setup(app.handle());
            api_access::setup(app.handle());
            let state = app.state::<SessionState>().inner().clone();
            let handle = app.handle().clone();
            let (tx, mut rx) = tokio::sync::mpsc::channel(32);
            tauri::async_runtime::spawn(lcu_connector::watch(tx));
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    use olc_desktop_support::diagnostics::DiagnosticCode;
                    let code = match &event {
                        lcu_connector::LcuEvent::Connected { .. } => {
                            Some(DiagnosticCode::Connected)
                        }
                        lcu_connector::LcuEvent::Disconnected => Some(DiagnosticCode::Disconnected),
                        lcu_connector::LcuEvent::PhaseChanged { .. } => {
                            Some(DiagnosticCode::PhaseChanged)
                        }
                        _ => None,
                    };
                    if let Some(code) = code {
                        diagnostics::record(&handle, code);
                    }
                    let snapshot = match state.lock() {
                        Ok(mut session) => {
                            session.apply(event);
                            handle.state::<imports::ImportLocks>().observe(&session);
                            players::lcu_changed(&handle, &session);
                            session.clone()
                        }
                        Err(_) => break,
                    };
                    // L'état courant reste lisible si aucune fenêtre n'écoute encore.
                    live::lcu_changed(&handle, &snapshot);
                    friends::lcu_changed(&handle, &snapshot);
                    let _ = handle.emit("lcu-session", snapshot);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            friends::friends_state,
            live::live_session,
            live::live_custom_role,
            overlay::overlay_state,
            overlay::overlay_content_height,
            overlay::overlay_locale,
            overlay::overlay_configure,
            overlay::overlay_preview,
            diagnostics::export_diagnostics,
            desktop::desktop_settings,
            desktop::set_desktop_setting,
            desktop::set_desktop_locale,
            lcu_status,
            lcu_session,
            client_patch,
            community_builds,
            publications::publication_state,
            api_access::api_access_status,
            api_access::save_api_access,
            api_access::clear_api_access,
            players::player_profile,
            players::player_matches,
            imports::import_runes,
            imports::import_draft_runes,
            imports::import_spells,
            imports::import_draft_spells,
            imports::import_items,
            imports::import_selected_build
        ])
        .build(tauri::generate_context!())
        .expect("erreur au lancement de l'application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                desktop::show_main(app);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}

#[cfg(test)]
mod integration_permissions_tests {
    #[test]
    fn les_commandes_systeme_sont_accessibles_uniquement_depuis_la_fenetre_principale() {
        let main: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
        let overlay: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/overlay.json")).unwrap();
        let manifest = include_str!("../build.rs");
        assert_eq!(main["windows"], serde_json::json!(["main"]));
        assert_eq!(overlay["windows"], serde_json::json!(["game-overlay"]));
        for command in [
            "desktop_settings",
            "set_desktop_setting",
            "set_desktop_locale",
            "export_diagnostics",
            "api_access_status",
            "save_api_access",
            "clear_api_access",
            "client_patch",
        ] {
            let permission = serde_json::json!(format!("allow-{}", command.replace('_', "-")));
            assert!(
                main["permissions"]
                    .as_array()
                    .unwrap()
                    .contains(&permission),
                "permission principale absente : {command}"
            );
            assert!(
                !overlay["permissions"]
                    .as_array()
                    .unwrap()
                    .contains(&permission),
                "commande système exposée à l'overlay : {command}"
            );
            assert!(
                manifest.contains(&format!("\"{command}\"")),
                "commande absente du manifeste : {command}"
            );
        }
    }
}
