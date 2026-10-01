use serde::Serialize;

mod imports;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(imports::ImportLocks::default())
        .invoke_handler(tauri::generate_handler![lcu_status, imports::import_runes,])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application");
}
