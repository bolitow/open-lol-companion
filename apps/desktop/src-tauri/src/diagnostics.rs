//! Export volontaire et local, sans messages libres ni données d'identité.
use olc_desktop_support::{
    diagnostics::{export_archive, summarize_log, DiagnosticCode, DiagnosticJournal},
    Locale,
};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

pub struct DiagnosticsState {
    journal: Mutex<DiagnosticJournal>,
    export_lock: Mutex<()>,
}
impl Default for DiagnosticsState {
    fn default() -> Self {
        let mut journal = DiagnosticJournal::default();
        journal.record(DiagnosticCode::Started);
        Self {
            journal: Mutex::new(journal),
            export_lock: Mutex::new(()),
        }
    }
}
pub fn record(app: &AppHandle, code: DiagnosticCode) {
    if let Some(state) = app.try_state::<DiagnosticsState>() {
        if let Ok(mut journal) = state.journal.lock() {
            journal.record(code);
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticExportResult {
    Exported,
    Cancelled,
}

#[tauri::command]
pub async fn export_diagnostics(
    app: AppHandle,
    include_league_summary: bool,
    locale: Locale,
) -> Result<DiagnosticExportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<DiagnosticsState>();
        let _guard = state.export_lock.try_lock().map_err(|_| "busy")?;
        let (pick, save) = match locale {
            Locale::Fr => (
                "Choisir des logs League à résumer",
                "Enregistrer le diagnostic local",
            ),
            Locale::En => ("Choose League logs to summarize", "Save local diagnostics"),
        };
        record(&app, DiagnosticCode::ExportRequested);
        let mut summaries = Vec::new();
        if include_league_summary {
            let Some(files) = app
                .dialog()
                .file()
                .set_title(pick)
                .add_filter("Logs", &["log", "txt"])
                .blocking_pick_files()
            else {
                return Ok(DiagnosticExportResult::Cancelled);
            };
            if files.len() > 5 {
                return Err("too_large".into());
            }
            for file in files {
                let path = file.into_path().map_err(|_| "unsupported")?;
                summaries.push(summarize_log(&path).map_err(|e| e.to_string())?);
            }
        }
        let Some(destination) = app
            .dialog()
            .file()
            .set_title(save)
            .set_file_name("open-lol-diagnostics.zip")
            .add_filter("ZIP", &["zip"])
            .blocking_save_file()
        else {
            return Ok(DiagnosticExportResult::Cancelled);
        };
        let path = destination.into_path().map_err(|_| "unsupported")?;
        // Le verrou du journal n'est jamais conservé pendant un dialogue utilisateur.
        let events = state.journal.lock().map_err(|_| "unavailable")?.snapshot();
        export_archive(
            &path,
            &events,
            &summaries,
            &app.package_info().version.to_string(),
        )
        .map_err(|e| e.to_string())?;
        record(&app, DiagnosticCode::ExportCompleted);
        Ok(DiagnosticExportResult::Exported)
    })
    .await
    .map_err(|_| "unavailable".to_owned())?
}
