//! Accès desktop à l'API : URL et jeton saisis dans les réglages, gardés dans le trousseau du système.
//!
//! Le jeton ne quitte jamais le cœur Rust après la saisie : l'interface ne relit que la source,
//! l'URL et un code d'erreur fermé. Aucune variante d'erreur ne transporte de message système.

use crate::{BuildClient, BuildError};
use serde::{Deserialize, Serialize};

/// Service du trousseau : identifiant de l'application desktop (`tauri.conf.json`).
pub const KEYCHAIN_SERVICE: &str = "io.github.bolitow.openlolcompanion";
/// Compte unique : une seule paire URL + jeton par session système.
pub const KEYCHAIN_ACCOUNT: &str = "api-access";
/// Le Gestionnaire d'identifiants Windows refuse un secret de plus de 2560 octets
/// (`CRED_MAX_CREDENTIAL_BLOB_SIZE`) ; la marge couvre l'enveloppe JSON.
pub const MAX_STORED_BYTES: usize = 2048;
const _: () = assert!(MAX_STORED_BYTES <= 2560);

/// Origine de la configuration active, miroir de `ApiAccessSource` dans @olc/shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiAccessSource {
    /// `OLC_API_URL` et `OLC_API_TOKEN` (raccordement développeur), prioritaires.
    Environment,
    /// Saisie dans les réglages, relue depuis le trousseau du système.
    Keychain,
}

/// Codes stables traduits par l'interface, miroir de `ApiAccessError` dans @olc/shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialError {
    InvalidConfiguration,
    TooLong,
    StorageUnavailable,
    ReadFailed,
    WriteFailed,
}

/// État lisible par l'interface, miroir de `ApiAccessStatus` dans @olc/shared.
/// Ne contient jamais le jeton : `source` non nul suffit à dire qu'il est présent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiAccessStatus {
    pub source: Option<ApiAccessSource>,
    pub url: Option<String>,
    pub error: Option<CredentialError>,
}

/// Résultat d'un chargement : client prêt (ou erreur de build) et état affichable.
pub struct LoadedAccess {
    pub client: Result<BuildClient, BuildError>,
    pub status: ApiAccessStatus,
}

/// Stockage du secret ; implémenté par le trousseau du système et, en test, par un double en mémoire.
pub trait SecretStore {
    /// `Ok(None)` si aucune entrée n'existe encore.
    fn read(&self) -> Result<Option<Vec<u8>>, CredentialError>;
    fn write(&self, secret: &[u8]) -> Result<(), CredentialError>;
    /// Une entrée absente n'est pas une erreur.
    fn delete(&self) -> Result<(), CredentialError>;
}

/// Contenu de l'entrée du trousseau. Pas de `Debug` : le jeton ne doit jamais être journalisé.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredAccess {
    url: String,
    token: String,
}

/// Variables d'environnement complètes d'abord (développement), sinon trousseau.
pub fn load_access(
    env_url: Option<String>,
    env_token: Option<String>,
    store: &dyn SecretStore,
) -> LoadedAccess {
    if let (Some(url), Some(token)) = (env_url, env_token) {
        let client = BuildClient::new(Some(url.clone()), Some(token));
        return LoadedAccess {
            status: ApiAccessStatus {
                source: Some(ApiAccessSource::Environment),
                url: Some(url),
                error: client
                    .as_ref()
                    .err()
                    .map(|_| CredentialError::InvalidConfiguration),
            },
            client,
        };
    }
    let stored = match store.read() {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return LoadedAccess {
                client: Err(BuildError::NotConfigured),
                status: ApiAccessStatus {
                    source: None,
                    url: None,
                    error: None,
                },
            }
        }
        Err(error) => {
            return LoadedAccess {
                client: Err(BuildError::NotConfigured),
                status: ApiAccessStatus {
                    source: None,
                    url: None,
                    error: Some(error),
                },
            }
        }
    };
    let Ok(access) = serde_json::from_slice::<StoredAccess>(&stored) else {
        return LoadedAccess {
            client: Err(BuildError::InvalidConfiguration),
            status: ApiAccessStatus {
                source: Some(ApiAccessSource::Keychain),
                url: None,
                error: Some(CredentialError::InvalidConfiguration),
            },
        };
    };
    let client = BuildClient::new(Some(access.url.clone()), Some(access.token));
    LoadedAccess {
        status: ApiAccessStatus {
            source: Some(ApiAccessSource::Keychain),
            url: Some(access.url),
            error: client
                .as_ref()
                .err()
                .map(|_| CredentialError::InvalidConfiguration),
        },
        client,
    }
}

/// Valide avec les règles de `BuildClient::new` puis écrit ; rien n'est écrit si la saisie est refusée.
pub fn save_access(
    store: &dyn SecretStore,
    url: String,
    token: String,
) -> Result<(), CredentialError> {
    let access = StoredAccess {
        url: url.trim().to_owned(),
        token: token.trim().to_owned(),
    };
    BuildClient::new(Some(access.url.clone()), Some(access.token.clone()))
        .map_err(|_| CredentialError::InvalidConfiguration)?;
    let bytes = serde_json::to_vec(&access).map_err(|_| CredentialError::WriteFailed)?;
    if bytes.len() > MAX_STORED_BYTES {
        return Err(CredentialError::TooLong);
    }
    store.write(&bytes)
}

/// Retire l'entrée du trousseau ; les variables d'environnement éventuelles restent actives.
pub fn clear_access(store: &dyn SecretStore) -> Result<(), CredentialError> {
    store.delete()
}

/// Trousseau du système : Keychain (macOS) ou Gestionnaire d'identifiants (Windows).
/// Les messages d'erreur de la crate ne sont jamais propagés : ils peuvent citer service et compte.
pub struct KeyringStore;

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl KeyringStore {
    fn entry() -> Result<keyring::Entry, CredentialError> {
        keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
            .map_err(|_| CredentialError::StorageUnavailable)
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl SecretStore for KeyringStore {
    fn read(&self) -> Result<Option<Vec<u8>>, CredentialError> {
        match Self::entry()?.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(keyring::Error::NoStorageAccess(_)) => Err(CredentialError::StorageUnavailable),
            Err(_) => Err(CredentialError::ReadFailed),
        }
    }
    fn write(&self, secret: &[u8]) -> Result<(), CredentialError> {
        match Self::entry()?.set_secret(secret) {
            Ok(()) => Ok(()),
            Err(keyring::Error::TooLong(..)) => Err(CredentialError::TooLong),
            Err(keyring::Error::NoStorageAccess(_)) => Err(CredentialError::StorageUnavailable),
            Err(_) => Err(CredentialError::WriteFailed),
        }
    }
    fn delete(&self) -> Result<(), CredentialError> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(keyring::Error::NoStorageAccess(_)) => Err(CredentialError::StorageUnavailable),
            Err(_) => Err(CredentialError::WriteFailed),
        }
    }
}

/// Hors Windows et macOS (CI Linux), aucun trousseau n'est branché : jamais de stockage factice.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl SecretStore for KeyringStore {
    fn read(&self) -> Result<Option<Vec<u8>>, CredentialError> {
        Err(CredentialError::StorageUnavailable)
    }
    fn write(&self, _secret: &[u8]) -> Result<(), CredentialError> {
        Err(CredentialError::StorageUnavailable)
    }
    fn delete(&self) -> Result<(), CredentialError> {
        Err(CredentialError::StorageUnavailable)
    }
}

#[cfg(test)]
mod tests;
