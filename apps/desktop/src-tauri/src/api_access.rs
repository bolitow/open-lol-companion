//! Accès à l'API desktop : URL et jeton saisis dans les réglages, gardés dans le trousseau du système.
//!
//! Le client actif est remplaçable à chaud : un enregistrement ou un effacement relit le trousseau,
//! remplace le client et relance l'écoute des publications. Le jeton n'est jamais renvoyé à l'interface.
use olc_build_client::{
    credentials::{self, ApiAccessStatus, CredentialError, KeyringStore, LoadedAccess},
    BuildClient, BuildError,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, RwLock};
use tauri::{AppHandle, Emitter, Manager};

struct Current {
    client: Result<Arc<BuildClient>, BuildError>,
    status: ApiAccessStatus,
    rejected: bool,
}

/// Projection sans jeton, miroir de `ApiAccessStatus` côté interface.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiAccessView {
    #[serde(flatten)]
    status: ApiAccessStatus,
    authorization_rejected: bool,
}

/// État partagé des commandes builds, profils et publications.
pub struct ApiState {
    current: RwLock<Current>,
    /// Passe à `true` après la première lecture du trousseau ; les lectures l'attendent.
    loaded: tokio::sync::watch::Sender<bool>,
    /// Sérialise chargement initial, enregistrement et effacement.
    writes: tokio::sync::Mutex<()>,
    watcher: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl Default for ApiState {
    fn default() -> Self {
        Self {
            current: RwLock::new(Current {
                rejected: false,
                client: Err(BuildError::NotConfigured),
                status: ApiAccessStatus {
                    source: None,
                    url: None,
                    error: None,
                },
            }),
            loaded: tokio::sync::watch::Sender::new(false),
            writes: tokio::sync::Mutex::new(()),
            watcher: Mutex::new(None),
        }
    }
}

impl ApiState {
    async fn wait_loaded(&self) {
        let mut loaded = self.loaded.subscribe();
        // L'émetteur vit aussi longtemps que l'état : l'attente ne peut pas échouer.
        let _ = loaded.wait_for(|loaded| *loaded).await;
    }

    /// Client actif, une fois le trousseau lu au démarrage.
    pub async fn client(&self) -> Result<Arc<BuildClient>, BuildError> {
        self.wait_loaded().await;
        let current = self.current.read().unwrap_or_else(|e| e.into_inner());
        current.client.clone()
    }

    /// Le commit du catalogue partage le verrou avec le remplacement de configuration.
    pub(crate) fn with_current_client<T>(
        &self,
        client: &Arc<BuildClient>,
        commit: impl FnOnce() -> Result<T, &'static str>,
    ) -> Result<T, &'static str> {
        let current = self.current.read().map_err(|_| "catalog_changed")?;
        if !current
            .client
            .as_ref()
            .is_ok_and(|active| Arc::ptr_eq(active, client))
        {
            return Err("catalog_changed");
        }
        commit()
    }

    async fn status(&self) -> ApiAccessView {
        self.wait_loaded().await;
        let current = self.current.read().unwrap_or_else(|e| e.into_inner());
        ApiAccessView {
            status: current.status.clone(),
            authorization_rejected: current.rejected,
        }
    }

    // Un retour tardif ne doit pas invalider le jeton qui vient d'être remplacé.
    fn reject(&self, client: &Arc<BuildClient>) -> bool {
        let mut current = self.current.write().unwrap_or_else(|e| e.into_inner());
        if current
            .client
            .as_ref()
            .is_ok_and(|active| Arc::ptr_eq(active, client))
            && !current.rejected
        {
            current.rejected = true;
            return true;
        }
        false
    }

    /// Remplace la configuration ; renvoie le client à écouter et l'état affichable.
    fn replace(&self, loaded: LoadedAccess) -> (Option<Arc<BuildClient>>, ApiAccessStatus) {
        let client = loaded.client.map(Arc::new);
        let listen = client.as_ref().ok().cloned();
        *self.current.write().unwrap_or_else(|e| e.into_inner()) = Current {
            client,
            rejected: false,
            status: loaded.status.clone(),
        };
        self.loaded.send_replace(true);
        (listen, loaded.status)
    }
}

/// Signale un refus provenant du client courant, sans transporter sa réponse ni son jeton.
pub fn report_rejection(app: &AppHandle, client: &Arc<BuildClient>) {
    if app.state::<ApiState>().reject(client) {
        let _ = app.emit_to("main", "api-access-rejected", ());
    }
}

/// Variables d'environnement complètes (développement) d'abord, sinon trousseau du système.
fn load_from_system() -> LoadedAccess {
    credentials::load_access(
        std::env::var("OLC_API_URL").ok(),
        std::env::var("OLC_API_TOKEN").ok(),
        &KeyringStore,
    )
}

/// À appeler sous le verrou `writes` : client et flux de publications changent ensemble.
fn apply(app: &AppHandle, loaded: LoadedAccess) -> ApiAccessStatus {
    let state = app.state::<ApiState>();
    let (client, status) = state.replace(loaded);
    let mut watcher = state.watcher.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(previous) = watcher.take() {
        previous.abort();
    }
    *watcher = crate::publications::restart(app, client);
    status
}

/// Lit le trousseau hors du fil principal : macOS peut demander une autorisation.
pub fn setup(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<ApiState>();
        let _guard = state.writes.lock().await;
        let loaded = tauri::async_runtime::spawn_blocking(load_from_system)
            .await
            .unwrap_or_else(|_| LoadedAccess {
                client: Err(BuildError::NotConfigured),
                status: ApiAccessStatus {
                    source: None,
                    url: None,
                    error: Some(CredentialError::ReadFailed),
                },
            });
        apply(&app, loaded);
    });
}

/// Saisie des réglages. Pas de `Debug` : le jeton ne doit jamais être journalisé.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiAccessInput {
    url: String,
    token: String,
}

#[tauri::command]
pub async fn api_access_status(
    state: tauri::State<'_, ApiState>,
) -> Result<ApiAccessView, CredentialError> {
    Ok(state.status().await)
}

/// Valide, écrit dans le trousseau puis applique sans redémarrage.
#[tauri::command]
pub async fn save_api_access(
    app: AppHandle,
    access: ApiAccessInput,
) -> Result<ApiAccessView, CredentialError> {
    let state = app.state::<ApiState>();
    let _guard = state.writes.lock().await;
    let loaded = tauri::async_runtime::spawn_blocking(move || {
        credentials::save_access(&KeyringStore, access.url, access.token)?;
        Ok(load_from_system())
    })
    .await
    .map_err(|_| CredentialError::WriteFailed)??;
    let status = apply(&app, loaded);
    Ok(ApiAccessView {
        status,
        authorization_rejected: false,
    })
}

/// Retire le jeton du trousseau ; les variables d'environnement éventuelles restent actives.
#[tauri::command]
pub async fn clear_api_access(app: AppHandle) -> Result<ApiAccessView, CredentialError> {
    let state = app.state::<ApiState>();
    let _guard = state.writes.lock().await;
    let loaded = tauri::async_runtime::spawn_blocking(|| {
        credentials::clear_access(&KeyringStore)?;
        Ok(load_from_system())
    })
    .await
    .map_err(|_| CredentialError::WriteFailed)??;
    let status = apply(&app, loaded);
    Ok(ApiAccessView {
        status,
        authorization_rejected: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use olc_build_client::credentials::ApiAccessSource;

    #[test]
    fn activation_catalogue_refuse_un_client_remplace_et_garde_le_verrou() {
        let state = ApiState::default();
        let (a, _) = state.replace(configured());
        let a = a.unwrap();
        assert_eq!(
            state.with_current_client(&a, || {
                assert!(state.current.try_write().is_err());
                Ok(42)
            }),
            Ok(42)
        );
        state.replace(configured());
        let called = std::cell::Cell::new(false);
        assert!(state
            .with_current_client(&a, || {
                called.set(true);
                Ok(())
            })
            .is_err());
        assert!(!called.get());
    }
    fn configured() -> LoadedAccess {
        LoadedAccess {
            client: BuildClient::new(
                Some("https://api.example.com".into()),
                Some("test-token".into()),
            ),
            status: ApiAccessStatus {
                source: Some(ApiAccessSource::Keychain),
                url: Some("https://api.example.com".into()),
                error: None,
            },
        }
    }

    #[test]
    fn les_lectures_attendent_le_premier_chargement_du_trousseau() {
        let state = Arc::new(ApiState::default());
        let reader = state.clone();
        let pending = tauri::async_runtime::spawn(async move { reader.client().await.is_ok() });
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(!pending.inner().is_finished());
        let (listen, status) = state.replace(configured());
        assert!(listen.is_some());
        assert_eq!(status.source, Some(ApiAccessSource::Keychain));
        assert!(tauri::async_runtime::block_on(pending).unwrap());
    }

    #[test]
    fn un_effacement_remplace_le_client_actif() {
        let state = ApiState::default();
        state.replace(configured());
        let (listen, status) = state.replace(LoadedAccess {
            client: Err(BuildError::NotConfigured),
            status: ApiAccessStatus {
                source: None,
                url: None,
                error: None,
            },
        });
        assert!(listen.is_none());
        assert_eq!(status.source, None);
        assert!(matches!(
            tauri::async_runtime::block_on(state.client()),
            Err(BuildError::NotConfigured)
        ));
    }
    #[test]
    fn le_refus_appartient_au_client_courant_et_disparait_au_remplacement() {
        let state = ApiState::default();
        let (old, _) = state.replace(configured());
        let old = old.unwrap();
        assert!(state.reject(&old));
        assert!(tauri::async_runtime::block_on(state.status()).authorization_rejected);
        let (new, _) = state.replace(configured());
        assert!(!state.reject(&old));
        assert!(!tauri::async_runtime::block_on(state.status()).authorization_rejected);
        assert!(state.reject(&new.unwrap()));
        let view = serde_json::to_value(tauri::async_runtime::block_on(state.status())).unwrap();
        assert_eq!(view["authorizationRejected"], true);
        assert!(!view.to_string().contains("test-token"));
    }
}
