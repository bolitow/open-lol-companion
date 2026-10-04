use super::*;
use std::sync::Mutex;

/// Trousseau en mémoire : les tests ne touchent jamais au trousseau réel du système.
#[derive(Default)]
struct MemoryStore {
    secret: Mutex<Option<Vec<u8>>>,
    fail_read: bool,
    fail_write: bool,
}

impl SecretStore for MemoryStore {
    fn read(&self) -> Result<Option<Vec<u8>>, CredentialError> {
        if self.fail_read {
            return Err(CredentialError::ReadFailed);
        }
        Ok(self.secret.lock().unwrap().clone())
    }
    fn write(&self, secret: &[u8]) -> Result<(), CredentialError> {
        if self.fail_write {
            return Err(CredentialError::WriteFailed);
        }
        *self.secret.lock().unwrap() = Some(secret.to_vec());
        Ok(())
    }
    fn delete(&self) -> Result<(), CredentialError> {
        *self.secret.lock().unwrap() = None;
        Ok(())
    }
}

const URL: &str = "https://api.example.com";
const TOKEN: &str = "test-token";

#[test]
fn sans_variable_ni_trousseau_l_acces_n_est_pas_configure() {
    let loaded = load_access(None, None, &MemoryStore::default());
    assert!(matches!(loaded.client, Err(BuildError::NotConfigured)));
    assert_eq!(
        loaded.status,
        ApiAccessStatus {
            source: None,
            url: None,
            error: None
        }
    );
}

#[test]
fn le_jeton_enregistre_est_relu_depuis_le_trousseau() {
    let store = MemoryStore::default();
    save_access(&store, URL.into(), TOKEN.into()).unwrap();
    let loaded = load_access(None, None, &store);
    assert!(loaded.client.is_ok());
    assert_eq!(loaded.status.source, Some(ApiAccessSource::Keychain));
    assert_eq!(loaded.status.url.as_deref(), Some(URL));
    assert_eq!(loaded.status.error, None);
}

#[test]
fn les_variables_d_environnement_completes_restent_prioritaires() {
    let store = MemoryStore::default();
    save_access(&store, URL.into(), TOKEN.into()).unwrap();
    let loaded = load_access(
        Some("http://127.0.0.1:3030".into()),
        Some(TOKEN.into()),
        &store,
    );
    assert!(loaded.client.is_ok());
    assert_eq!(loaded.status.source, Some(ApiAccessSource::Environment));
    assert_eq!(loaded.status.url.as_deref(), Some("http://127.0.0.1:3030"));
    // Une seule variable ne suffit pas : le trousseau reprend la main.
    let partial = load_access(Some("http://127.0.0.1:3030".into()), None, &store);
    assert_eq!(partial.status.source, Some(ApiAccessSource::Keychain));
}

#[test]
fn une_configuration_invalide_n_est_jamais_ecrite() {
    let store = MemoryStore::default();
    for (url, token) in [
        ("http://example.com", TOKEN),
        ("https://example.com/?token=a", TOKEN),
        (URL, ""),
        (URL, "   "),
        ("", TOKEN),
    ] {
        assert_eq!(
            save_access(&store, url.into(), token.into()).err(),
            Some(CredentialError::InvalidConfiguration)
        );
    }
    assert_eq!(store.read().unwrap(), None);
}

#[test]
fn les_espaces_colles_autour_de_l_url_et_du_jeton_sont_retires() {
    let store = MemoryStore::default();
    save_access(&store, format!(" {URL}\n"), format!("{TOKEN}\n")).unwrap();
    let loaded = load_access(None, None, &store);
    assert_eq!(loaded.status.url.as_deref(), Some(URL));
    let raw = String::from_utf8(store.read().unwrap().unwrap()).unwrap();
    assert!(raw.contains(&format!("\"{TOKEN}\"")));
}

#[test]
fn le_secret_tient_dans_la_limite_du_gestionnaire_d_identifiants_windows() {
    let store = MemoryStore::default();
    let long = "a".repeat(MAX_STORED_BYTES);
    assert_eq!(
        save_access(&store, URL.into(), long).err(),
        Some(CredentialError::TooLong)
    );
    assert_eq!(store.read().unwrap(), None);
    let fits = "a".repeat(MAX_STORED_BYTES - 64);
    save_access(&store, URL.into(), fits).unwrap();
    assert!(store.read().unwrap().unwrap().len() <= MAX_STORED_BYTES);
}

#[test]
fn un_echec_de_lecture_se_distingue_d_un_acces_non_configure() {
    let store = MemoryStore {
        fail_read: true,
        ..MemoryStore::default()
    };
    let loaded = load_access(None, None, &store);
    assert!(matches!(loaded.client, Err(BuildError::NotConfigured)));
    assert_eq!(loaded.status.source, None);
    assert_eq!(loaded.status.error, Some(CredentialError::ReadFailed));
}

#[test]
fn un_contenu_illisible_du_trousseau_est_signale_sans_etre_expose() {
    let store = MemoryStore::default();
    store.write(b"not-json secret").unwrap();
    let loaded = load_access(None, None, &store);
    assert!(matches!(
        loaded.client,
        Err(BuildError::InvalidConfiguration)
    ));
    assert_eq!(loaded.status.source, Some(ApiAccessSource::Keychain));
    assert_eq!(loaded.status.url, None);
    assert_eq!(
        loaded.status.error,
        Some(CredentialError::InvalidConfiguration)
    );
}

#[test]
fn un_echec_d_ecriture_est_remonte() {
    let store = MemoryStore {
        fail_write: true,
        ..MemoryStore::default()
    };
    assert_eq!(
        save_access(&store, URL.into(), TOKEN.into()).err(),
        Some(CredentialError::WriteFailed)
    );
}

#[test]
fn l_effacement_retire_le_jeton_et_tolere_un_trousseau_vide() {
    let store = MemoryStore::default();
    clear_access(&store).unwrap();
    save_access(&store, URL.into(), TOKEN.into()).unwrap();
    clear_access(&store).unwrap();
    let loaded = load_access(None, None, &store);
    assert!(matches!(loaded.client, Err(BuildError::NotConfigured)));
    assert_eq!(loaded.status.source, None);
}

#[test]
fn le_statut_serialise_ne_contient_jamais_le_jeton() {
    let store = MemoryStore::default();
    save_access(&store, URL.into(), TOKEN.into()).unwrap();
    let json = serde_json::to_string(&load_access(None, None, &store).status).unwrap();
    assert!(!json.contains(TOKEN));
    assert_eq!(
        json,
        format!("{{\"source\":\"keychain\",\"url\":\"{URL}\",\"error\":null}}")
    );
}

/// Sans les fonctionnalités natives, keyring retombe silencieusement sur un stockage en mémoire.
#[cfg(any(target_os = "macos", target_os = "windows"))]
#[test]
fn le_trousseau_natif_du_systeme_est_compile() {
    assert!(matches!(
        keyring::default::default_credential_builder().persistence(),
        keyring::credential::CredentialPersistence::UntilDelete
    ));
}

#[test]
fn les_codes_serialises_figurent_dans_le_miroir_typescript() {
    let mirror = include_str!("../../../../packages/shared/src/apiAccess.ts");
    let codes = [
        serde_json::to_value(ApiAccessSource::Environment).unwrap(),
        serde_json::to_value(ApiAccessSource::Keychain).unwrap(),
        serde_json::to_value(CredentialError::InvalidConfiguration).unwrap(),
        serde_json::to_value(CredentialError::TooLong).unwrap(),
        serde_json::to_value(CredentialError::StorageUnavailable).unwrap(),
        serde_json::to_value(CredentialError::ReadFailed).unwrap(),
        serde_json::to_value(CredentialError::WriteFailed).unwrap(),
    ];
    for code in codes {
        let code = code.as_str().unwrap();
        assert!(
            mirror.contains(&format!("'{code}'")),
            "code absent : {code}"
        );
    }
}
