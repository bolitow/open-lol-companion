//! Diagnostics à schéma fermé : aucune ligne de log brute ne sort du lecteur.
use serde::Serialize;
use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    time::Instant,
};

const MAX_EVENTS: usize = 200;
const MAX_LOG_BYTES: u64 = 8 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    Started,
    Connected,
    Disconnected,
    PhaseChanged,
    SettingChanged,
    ExportRequested,
    ExportCompleted,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEvent {
    elapsed_ms: u64,
    code: DiagnosticCode,
}
pub struct DiagnosticJournal {
    start: Instant,
    events: VecDeque<DiagnosticEvent>,
}
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DiagnosticError {
    #[error("read_failed")]
    ReadFailed,
    #[error("write_failed")]
    WriteFailed,
    #[error("too_large")]
    TooLarge,
    #[error("unsupported")]
    Unsupported,
}
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeagueSummary {
    pub lines: u64,
    pub error_markers: u64,
    pub warning_markers: u64,
    pub info_markers: u64,
}
impl Default for DiagnosticJournal {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            events: VecDeque::new(),
        }
    }
}
impl DiagnosticJournal {
    /// Conserve les 200 derniers codes, sans message libre ni payload du client.
    pub fn record(&mut self, code: DiagnosticCode) {
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(DiagnosticEvent {
            elapsed_ms: self.start.elapsed().as_millis().min(u64::MAX as u128) as u64,
            code,
        });
    }
    /// Copie bornée pour exporter sans conserver le verrou pendant le dialogue natif.
    pub fn snapshot(&self) -> Vec<DiagnosticEvent> {
        self.events.iter().cloned().collect()
    }
}
/// Résume un fichier explicite, UTF-8, régulier et borné ; aucun nom ni contenu brut n'est retourné.
pub fn summarize_log(path: &Path) -> Result<LeagueSummary, DiagnosticError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| DiagnosticError::ReadFailed)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(DiagnosticError::Unsupported);
    }
    if metadata.len() > MAX_LOG_BYTES {
        return Err(DiagnosticError::TooLarge);
    }
    let file = File::open(path).map_err(|_| DiagnosticError::ReadFailed)?;
    if !file
        .metadata()
        .map_err(|_| DiagnosticError::ReadFailed)?
        .is_file()
    {
        return Err(DiagnosticError::Unsupported);
    }
    let mut bytes = Vec::new();
    file.take(MAX_LOG_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| DiagnosticError::ReadFailed)?;
    if bytes.len() as u64 > MAX_LOG_BYTES {
        return Err(DiagnosticError::TooLarge);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| DiagnosticError::Unsupported)?;
    if text.contains('\0') {
        return Err(DiagnosticError::Unsupported);
    }
    let mut summary = LeagueSummary::default();
    for line in text.lines() {
        summary.lines += 1;
        // Compteurs de marqueurs lexicaux, pas interprétation de causes ou classement du joueur.
        let mut error = false;
        let mut warning = false;
        let mut info = false;
        for word in line.split(|character: char| !character.is_ascii_alphabetic()) {
            error |= word.eq_ignore_ascii_case("error");
            warning |= word.eq_ignore_ascii_case("warn") || word.eq_ignore_ascii_case("warning");
            info |= word.eq_ignore_ascii_case("info");
        }
        summary.error_markers += u64::from(error);
        summary.warning_markers += u64::from(warning);
        summary.info_markers += u64::from(info);
    }
    Ok(summary)
}
/// Écrit uniquement des entrées ZIP à noms fixes, atomiquement dans le dossier choisi.
pub fn export_archive(
    path: &Path,
    events: &[DiagnosticEvent],
    league: &[LeagueSummary],
    app_version: &str,
) -> Result<(), DiagnosticError> {
    if events.len() > MAX_EVENTS || league.len() > 5 {
        return Err(DiagnosticError::TooLarge);
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| DiagnosticError::WriteFailed)?;
    let manifest = serde_json::json!({
        "schemaVersion": 1, "appVersion": app_version,
        "os": std::env::consts::OS, "architecture": std::env::consts::ARCH,
        "sessionOnly": true, "eventLimit": MAX_EVENTS, "leagueFiles": league.len(),
        "leagueContent": "marker_counts_only", "rawLogsIncluded": false,
        "excluded": ["account_identity", "credentials", "paths", "raw_messages", "environment", "network_payloads"]
    });
    {
        let mut archive = zip::ZipWriter::new(temporary.as_file_mut());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let entries = [
            ("manifest.json", serde_json::to_vec_pretty(&manifest)),
            ("app-events.json", serde_json::to_vec_pretty(events)),
            ("league-summary.json", serde_json::to_vec_pretty(league)),
        ];
        for (name, bytes) in entries {
            archive
                .start_file(name, options)
                .map_err(|_| DiagnosticError::WriteFailed)?;
            archive
                .write_all(&bytes.map_err(|_| DiagnosticError::WriteFailed)?)
                .map_err(|_| DiagnosticError::WriteFailed)?;
        }
        archive.finish().map_err(|_| DiagnosticError::WriteFailed)?;
    }
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| DiagnosticError::WriteFailed)?;
    temporary
        .persist(path)
        .map_err(|_| DiagnosticError::WriteFailed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Read};
    #[test]
    fn journal_borne_et_codes_fermes() {
        let mut journal = DiagnosticJournal::default();
        for _ in 0..250 {
            journal.record(DiagnosticCode::PhaseChanged);
        }
        journal.record(DiagnosticCode::Disconnected);
        let events = journal.snapshot();
        assert_eq!(events.len(), MAX_EVENTS);
        let json = serde_json::to_value(events).unwrap();
        assert_eq!(json[199]["code"], "disconnected");
        assert_eq!(json[0].as_object().unwrap().len(), 2);
    }
    #[test]
    fn resume_sans_conserver_texte_identites_ou_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("private-player.log");
        fs::write(&p,"INFO harmless\nERROR token=DO_NOT_EXPORT\nWARN player=PRIVATE_PLAYER\nunknown C:\\private\\path\n").unwrap();
        let summary = summarize_log(&p).unwrap();
        assert_eq!(summary.lines, 4);
        assert_eq!(summary.error_markers, 1);
        assert_eq!(summary.warning_markers, 1);
        assert_eq!(summary.info_markers, 1);
        let json = serde_json::to_string(&summary).unwrap();
        for secret in ["DO_NOT_EXPORT", "PRIVATE_PLAYER", "private", "token"] {
            assert!(!json.contains(secret));
        }
    }
    #[test]
    fn refuse_binaire_trop_gros_et_dossier() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("source.log");
        fs::write(&p, [0, 255]).unwrap();
        assert_eq!(summarize_log(&p).unwrap_err(), DiagnosticError::Unsupported);
        fs::write(&p, b"INFO\0secret").unwrap();
        assert_eq!(summarize_log(&p).unwrap_err(), DiagnosticError::Unsupported);
        fs::File::create(&p)
            .unwrap()
            .set_len(MAX_LOG_BYTES + 1)
            .unwrap();
        assert_eq!(summarize_log(&p).unwrap_err(), DiagnosticError::TooLarge);
        assert_eq!(
            summarize_log(dir.path()).unwrap_err(),
            DiagnosticError::Unsupported
        );
    }
    #[cfg(unix)]
    #[test]
    fn refuse_lien_symbolique() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let link = dir.path().join("link");
        fs::write(&source, "INFO").unwrap();
        std::os::unix::fs::symlink(source, &link).unwrap();
        assert_eq!(
            summarize_log(&link).unwrap_err(),
            DiagnosticError::Unsupported
        );
    }
    #[test]
    fn archive_avec_noms_fixes_contenu_borne_et_remplacement_atomique() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("report.zip");
        fs::write(&p, "ancien").unwrap();
        let mut journal = DiagnosticJournal::default();
        journal.record(DiagnosticCode::Started);
        export_archive(
            &p,
            &journal.snapshot(),
            &[LeagueSummary {
                lines: 10,
                ..Default::default()
            }],
            "9.8.7",
        )
        .unwrap();
        let mut zip = zip::ZipArchive::new(fs::File::open(&p).unwrap()).unwrap();
        assert_eq!(zip.len(), 3);
        for name in ["manifest.json", "app-events.json", "league-summary.json"] {
            let mut data = String::new();
            zip.by_name(name)
                .unwrap()
                .read_to_string(&mut data)
                .unwrap();
            let value = serde_json::from_str::<serde_json::Value>(&data).unwrap();
            if name == "manifest.json" {
                assert_eq!(value["appVersion"], "9.8.7");
            }
        }
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[test]
    fn trop_de_sources_ou_evenements_ne_remplace_pas_ancien_export() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("report.zip");
        fs::write(&p, "ancien").unwrap();
        let events = vec![
            DiagnosticEvent {
                elapsed_ms: 0,
                code: DiagnosticCode::Started
            };
            201
        ];
        assert_eq!(
            export_archive(&p, &events, &[], "9.8.7"),
            Err(DiagnosticError::TooLarge)
        );
        let logs = (0..6).map(|_| LeagueSummary::default()).collect::<Vec<_>>();
        assert_eq!(
            export_archive(&p, &[], &logs, "9.8.7"),
            Err(DiagnosticError::TooLarge)
        );
        assert_eq!(fs::read_to_string(p).unwrap(), "ancien");
    }
    #[test]
    fn erreur_destination_garde_le_fichier_existant_et_nettoie_temporaire() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("existing");
        fs::create_dir(&p).unwrap();
        fs::write(p.join("keep"), "keep").unwrap();
        assert_eq!(
            export_archive(&p, &[], &[], "9.8.7"),
            Err(DiagnosticError::WriteFailed)
        );
        assert_eq!(fs::read_to_string(p.join("keep")).unwrap(), "keep");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
