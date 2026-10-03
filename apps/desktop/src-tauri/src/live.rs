use lcu_connector::{
    live::{LiveInput, LiveSession},
    LcuSession,
};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tokio::sync::watch;

type Shared = Arc<Mutex<LiveSession>>;
pub struct Control(watch::Sender<LiveInput>);
#[tauri::command]
pub fn live_session(state: tauri::State<'_, Shared>) -> Result<LiveSession, &'static str> {
    state.lock().map(|s| s.clone()).map_err(|_| "unavailable")
}
#[tauri::command]
pub fn live_custom_role(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Control>,
    locks: tauri::State<'_, crate::imports::ImportLocks>,
    role: Option<String>,
) -> Result<(), &'static str> {
    if window.label() != "main"
        || role
            .as_deref()
            .is_some_and(|r| !lcu_connector::live::valid_role(r))
    {
        return Err("unavailable");
    }
    locks.set_custom_role(role.clone())?;
    state.0.send_if_modified(|input| {
        if input.custom_role == role {
            false
        } else {
            input.custom_role = role;
            true
        }
    });
    Ok(())
}
pub fn setup(app: &tauri::AppHandle) {
    let (tx, rx) = watch::channel(LiveInput::default());
    app.manage(Control(tx));
    let state = Arc::new(Mutex::new(LiveSession::default()));
    app.manage(state.clone());
    let (output, mut receive) = tokio::sync::mpsc::channel::<LiveSession>(16);
    tauri::async_runtime::spawn(lcu_connector::live::watch(rx, output));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(snapshot) = receive.recv().await {
            if let Ok(mut current) = state.lock() {
                *current = snapshot.clone();
            } else {
                break;
            }
            super::overlay::live_changed(&app, &snapshot);
            let _ = app.emit("live-session", snapshot);
        }
    });
}
pub fn lcu_changed(app: &tauri::AppHandle, session: &LcuSession) {
    super::overlay::lcu_changed(app, session);
    app.state::<Control>()
        .0
        .send_modify(|input| input.set_session(session));
}
