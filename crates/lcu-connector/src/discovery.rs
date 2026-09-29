use std::path::PathBuf;
use std::process::Command;

use thiserror::Error;

use crate::{Credentials, ParseError};

/// Variable d'environnement pour forcer le chemin du lockfile (installation non standard, tests).
pub const LOCKFILE_ENV: &str = "OLC_LOL_LOCKFILE";

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("client League of Legends introuvable (ni lockfile, ni processus LeagueClientUx)")]
    NotRunning,
    #[error("lockfile {path} illisible : {source}")]
    Parse { path: PathBuf, source: ParseError },
}

/// Emplacements standard du lockfile sur la plateforme courante.
pub fn default_lockfile_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if cfg!(target_os = "windows") {
        for drive in ["C", "D", "E"] {
            paths.push(PathBuf::from(format!(
                r"{drive}:\Riot Games\League of Legends\lockfile"
            )));
        }
    } else if cfg!(target_os = "macos") {
        paths.push(PathBuf::from(
            "/Applications/League of Legends.app/Contents/LoL/lockfile",
        ));
    }
    paths
}

/// Cherche le client : variable d'environnement, emplacements standard, puis
/// arguments du processus `LeagueClientUx` (installation dans un dossier personnalisé).
pub fn discover() -> Result<Credentials, DiscoveryError> {
    let mut candidates = Vec::new();
    if let Ok(p) = std::env::var(LOCKFILE_ENV) {
        candidates.push(PathBuf::from(p));
    }
    candidates.extend(default_lockfile_paths());

    for path in candidates {
        if let Ok(content) = std::fs::read_to_string(&path) {
            return Credentials::from_lockfile(&content)
                .map_err(|source| DiscoveryError::Parse { path, source });
        }
    }

    client_cmdline()
        .and_then(|cmd| Credentials::from_process_args(&cmd).ok())
        .ok_or(DiscoveryError::NotRunning)
}

/// Ligne de commande de `LeagueClientUx`, si le processus tourne.
fn client_cmdline() -> Option<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_Process -Filter \"Name='LeagueClientUx.exe'\").CommandLine",
            ])
            .output()
            .ok()?
    } else {
        Command::new("ps")
            .args(["-A", "-o", "args="])
            .output()
            .ok()?
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find(|l| l.contains("LeagueClientUx") && l.contains("--app-port="))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_variable_d_environnement_est_prioritaire() {
        let dir = std::env::temp_dir().join(format!("olc-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let lockfile = dir.join("lockfile");
        std::fs::write(&lockfile, "LeagueClient:7:4242:pw:https").unwrap();

        std::env::set_var(LOCKFILE_ENV, &lockfile);
        let creds = discover().unwrap();
        std::env::remove_var(LOCKFILE_ENV);
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(creds.port, 4242);
    }
}
