//! Préférences natives sans dépendance à Tauri ni au client League.

pub mod diagnostics;

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

const MAX_PREFERENCES_BYTES: u64 = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    Fr,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePreferences {
    pub close_to_tray: bool,
    pub locale: Locale,
}

impl Default for NativePreferences {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            locale: Locale::Fr,
        }
    }
}

/// Codes fermés : aucun chemin ni détail système n'est exposé à l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PreferenceError {
    #[error("preferences_read_failed")]
    ReadFailed,
    #[error("preferences_invalid")]
    Invalid,
    #[error("preferences_too_large")]
    TooLarge,
    #[error("preferences_write_failed")]
    WriteFailed,
}

/// Lit une préférence bornée ; seul un fichier absent reprend les valeurs par défaut.
pub fn load_preferences(path: &Path) -> Result<NativePreferences, PreferenceError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(NativePreferences::default())
        }
        Err(_) => return Err(PreferenceError::ReadFailed),
    };
    let metadata = file.metadata().map_err(|_| PreferenceError::ReadFailed)?;
    if !metadata.is_file() {
        return Err(PreferenceError::ReadFailed);
    }
    if metadata.len() > MAX_PREFERENCES_BYTES {
        return Err(PreferenceError::TooLarge);
    }
    // La limite s'applique aussi si un autre processus agrandit le fichier après metadata().
    let mut bytes = Vec::new();
    file.take(MAX_PREFERENCES_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PreferenceError::ReadFailed)?;
    if bytes.len() as u64 > MAX_PREFERENCES_BYTES {
        return Err(PreferenceError::TooLarge);
    }
    serde_json::from_slice(&bytes).map_err(|_| PreferenceError::Invalid)
}

/// Remplace atomiquement les préférences sans tronquer le fichier existant en cas d’échec.
pub fn save_preferences(
    path: &Path,
    preferences: &NativePreferences,
) -> Result<(), PreferenceError> {
    let bytes = serde_json::to_vec(preferences).map_err(|_| PreferenceError::WriteFailed)?;
    write_atomic(path, |file| file.write_all(&bytes))
}

fn write_atomic(
    path: &Path,
    write: impl FnOnce(&mut File) -> io::Result<()>,
) -> Result<(), PreferenceError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|_| PreferenceError::WriteFailed)?;
    // Même dossier : remplacement atomique sur le même volume, sans troncature de l'ancien fichier.
    // tempfile::persist utilise les primitives natives adaptées à Windows et macOS.
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| PreferenceError::WriteFailed)?;
    write(temporary.as_file_mut()).map_err(|_| PreferenceError::WriteFailed)?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| PreferenceError::WriteFailed)?;
    temporary
        .persist(path)
        .map_err(|_| PreferenceError::WriteFailed)?;
    Ok(())
}

/// La fenêtre doit rester accessible si la barre système est indisponible.
pub fn should_hide_on_close(close_to_tray: bool, tray_available: bool) -> bool {
    close_to_tray && tray_available
}

/// Un lancement automatique ne se masque que lorsque le tray permet la réouverture.
pub fn should_start_hidden(from_autostart: bool, tray_available: bool) -> bool {
    from_autostart && tray_available
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn fichier_absent_reprend_les_valeurs_par_defaut() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            load_preferences(&directory.path().join("absent.json")),
            Ok(NativePreferences {
                close_to_tray: true,
                locale: Locale::Fr
            })
        );
    }

    #[test]
    fn restaure_les_choix_apres_redemarrage_et_remplacement() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("native.json");
        let preferences = NativePreferences {
            close_to_tray: false,
            locale: Locale::En,
        };
        save_preferences(&path, &preferences).unwrap();
        assert_eq!(load_preferences(&path), Ok(preferences));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&path).unwrap()).unwrap(),
            serde_json::json!({"close_to_tray":false,"locale":"en"})
        );
        save_preferences(&path, &NativePreferences::default()).unwrap();
        assert_eq!(load_preferences(&path), Ok(NativePreferences::default()));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn refuse_un_fichier_partiel_ou_un_schema_invalide_sans_le_modifier() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("native.json");
        for content in [
            "{",
            "null",
            "{}",
            r#"{"close_to_tray":true,"locale":"de"}"#,
            r#"{"close_to_tray":true,"locale":"fr","autostart":true}"#,
        ] {
            fs::write(&path, content).unwrap();
            assert_eq!(load_preferences(&path), Err(PreferenceError::Invalid));
            assert_eq!(fs::read_to_string(&path).unwrap(), content);
        }
    }

    #[test]
    fn refuse_un_fichier_trop_grand_meme_si_json_valide() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("native.json");
        let mut content = serde_json::to_vec(&NativePreferences::default()).unwrap();
        content.resize(MAX_PREFERENCES_BYTES as usize, b' ');
        fs::write(&path, &content).unwrap();
        assert!(load_preferences(&path).is_ok());
        content.push(b' ');
        fs::write(&path, &content).unwrap();
        assert_eq!(load_preferences(&path), Err(PreferenceError::TooLarge));
    }

    #[test]
    fn distingue_absence_et_erreur_de_lecture() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            load_preferences(directory.path()),
            Err(PreferenceError::ReadFailed)
        );
    }

    #[test]
    fn signale_un_parent_inutilisable_sans_modification() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("pas-un-dossier");
        fs::write(&parent, "existant").unwrap();
        assert_eq!(
            save_preferences(&parent.join("native.json"), &NativePreferences::default()),
            Err(PreferenceError::WriteFailed)
        );
        assert_eq!(fs::read_to_string(parent).unwrap(), "existant");
    }

    #[test]
    fn une_erreur_apres_ecriture_partielle_preserve_le_fichier_precedent() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("native.json");
        let previous = br#"{"close_to_tray":false,"locale":"en"}"#;
        fs::write(&path, previous).unwrap();
        let result = write_atomic(&path, |file| {
            file.write_all(b"{")?;
            Err(io::Error::other("panne simulee"))
        });
        assert_eq!(result, Err(PreferenceError::WriteFailed));
        assert_eq!(fs::read(&path).unwrap(), previous);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn lectures_et_ecritures_concurrentes_ne_voient_jamais_un_fichier_partiel() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("native.json");
        let old = NativePreferences::default();
        let new = NativePreferences {
            close_to_tray: false,
            locale: Locale::En,
        };
        save_preferences(&path, &old).unwrap();
        let successes = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for preferences in [old, new] {
                let path = &path;
                let successes = &successes;
                scope.spawn(move || {
                    for _ in 0..30 {
                        // L'OS peut refuser deux remplacements concurrents ; l'intégrité reste obligatoire.
                        match save_preferences(path, &preferences) {
                            Ok(()) => {
                                successes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                            Err(PreferenceError::WriteFailed) => {}
                            result => panic!("résultat inattendu : {result:?}"),
                        }
                    }
                });
            }
            for _ in 0..100 {
                let observed = load_preferences(&path).unwrap();
                assert!(observed == old || observed == new);
            }
        });
        assert!(successes.load(std::sync::atomic::Ordering::Relaxed) > 0);
        let final_value = load_preferences(&path).unwrap();
        assert!(final_value == old || final_value == new);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn ne_cache_jamais_la_fenetre_sans_tray_disponible() {
        for close_to_tray in [false, true] {
            let preferences = NativePreferences {
                close_to_tray,
                ..NativePreferences::default()
            };
            assert!(!should_hide_on_close(preferences.close_to_tray, false));
            assert_eq!(
                should_hide_on_close(preferences.close_to_tray, true),
                close_to_tray
            );
        }
        for from_autostart in [false, true] {
            assert!(!should_start_hidden(from_autostart, false));
            assert_eq!(should_start_hidden(from_autostart, true), from_autostart);
        }
    }
}
