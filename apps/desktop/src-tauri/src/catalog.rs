//! Orchestration native du catalogue ; aucun chemin disque fourni par le front.
use olc_catalog_cache::{Cache, Manifest};
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};
use tauri::{Emitter, Manager};
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogRuntimeState {
    pub status: String,
    pub version: Option<String>,
    pub snapshot_id: Option<String>,
    pub asset_base: Option<String>,
    pub error: Option<String>,
}
impl Default for CatalogRuntimeState {
    fn default() -> Self {
        Self {
            status: "embedded".into(),
            version: None,
            snapshot_id: None,
            asset_base: None,
            error: None,
        }
    }
}
impl CatalogRuntimeState {
    fn fail(&mut self, error: &str) {
        self.status = "error".into();
        self.error = Some(error.into());
    }
    fn ready(m: &Manifest) -> Self {
        let scheme = if cfg!(target_os = "windows") {
            "http://catalog.localhost"
        } else {
            "catalog://localhost"
        };
        Self {
            status: "ready".into(),
            version: Some(m.version.clone()),
            snapshot_id: Some(m.snapshot_id.clone()),
            asset_base: Some(format!("{scheme}/{}/", m.snapshot_id)),
            error: None,
        }
    }
}
#[derive(Default)]
pub struct CatalogService {
    value: Mutex<CatalogRuntimeState>,
    initialized: tokio::sync::OnceCell<()>,
    gate: tokio::sync::Mutex<()>,
    leases: Mutex<std::collections::BTreeMap<String, olc_catalog_cache::Lease>>,
}
impl CatalogService {
    async fn initialize(
        &self,
        load: impl std::future::Future<Output = Option<CatalogRuntimeState>>,
    ) {
        self.initialized
            .get_or_init(|| async {
                if let Some(state) = load.await {
                    if let Ok(mut value) = self.value.lock() {
                        *value = state;
                    }
                }
            })
            .await;
    }
}
fn pin(app: &tauri::AppHandle, cache: &Cache, m: &Manifest) -> Result<(), &'static str> {
    let service = app.state::<CatalogService>();
    let mut leases = service.leases.lock().map_err(|_| "catalog_storage")?;
    if !leases.contains_key(&m.snapshot_id) {
        leases.insert(
            m.snapshot_id.clone(),
            cache.lease(&m.snapshot_id).map_err(|_| "catalog_storage")?,
        );
    }
    Ok(())
}
fn root(app: &tauri::AppHandle) -> Result<PathBuf, &'static str> {
    app.path()
        .app_local_data_dir()
        .map(|p| p.join("desktop-catalog-v1"))
        .map_err(|_| "catalog_storage")
}
fn value(app: &tauri::AppHandle) -> CatalogRuntimeState {
    app.state::<CatalogService>()
        .value
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default()
}
fn set(app: &tauri::AppHandle, state: CatalogRuntimeState) {
    if let Ok(mut v) = app.state::<CatalogService>().value.lock() {
        *v = state.clone();
    }
    let _ = app.emit("catalog-state", state);
}
#[tauri::command]
pub async fn catalog_state(app: tauri::AppHandle) -> CatalogRuntimeState {
    let service = app.state::<CatalogService>();
    service
        .initialize(async {
            let directory = root(&app);
            let worker_app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let cache = Cache::reader(&directory.ok()?).ok()?;
                let m = cache.active().ok().flatten();
                let recovered = cache.recovered(m.as_ref().map(|m| m.snapshot_id.as_str()));
                let mut state = if let Some(m) = m {
                    pin(&worker_app, &cache, &m).ok()?;
                    CatalogRuntimeState::ready(&m)
                } else {
                    CatalogRuntimeState::default()
                };
                if recovered {
                    state.fail("catalog_recovered");
                }
                Some(state)
            })
            .await
            .ok()
            .flatten()
        })
        .await;
    value(&app)
}
#[tauri::command]
pub async fn catalog_read(
    app: tauri::AppHandle,
    snapshot_id: String,
    path: String,
) -> Result<serde_json::Value, &'static str> {
    if !olc_catalog_cache::valid_id(&snapshot_id)
        || !olc_catalog_cache::valid_path(&path)
        || !path.ends_with(".json")
    {
        return Err("catalog_invalid");
    }
    let directory = root(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let cache = Cache::reader(&directory).map_err(|_| "catalog_storage")?;
        read_json(&cache, &snapshot_id, &path)
    })
    .await
    .map_err(|_| "catalog_storage")?
}
fn read_json(cache: &Cache, snapshot: &str, path: &str) -> Result<serde_json::Value, &'static str> {
    if path == "cosmetics.json"
        && !cache
            .manifest(snapshot)
            .map_err(|_| "catalog_invalid")?
            .files
            .contains_key(path)
    {
        return Ok(serde_json::Value::Null);
    }
    let bytes = cache.read(snapshot, path).map_err(|_| "catalog_invalid")?;
    serde_json::from_slice(&bytes).map_err(|_| "catalog_invalid")
}
#[tauri::command]
pub async fn catalog_sync(app: tauri::AppHandle) -> CatalogRuntimeState {
    let service = app.state::<CatalogService>();
    let Ok(_guard) = service.gate.try_lock() else {
        return value(&app);
    };
    let mut current = catalog_state(app.clone()).await;
    current.status = "updating".into();
    current.error = None;
    set(&app, current);
    let outcome = sync(&app).await;
    match outcome {
        Ok(Some(m)) => set(&app, CatalogRuntimeState::ready(&m)),
        Ok(None) => {
            let mut state = value(&app);
            state.status = if state.snapshot_id.is_some() {
                "ready"
            } else {
                "embedded"
            }
            .into();
            set(&app, state);
        }
        Err(error) => {
            let mut state = value(&app);
            state.fail(error);
            set(&app, state);
        }
    }
    value(&app)
}
async fn sync(app: &tauri::AppHandle) -> Result<Option<Manifest>, &'static str> {
    let Ok(patch) = super::client_patch().await else {
        return Ok(None);
    };
    let client = app
        .state::<crate::api_access::ApiState>()
        .client()
        .await
        .map_err(|_| "catalog_unavailable")?;
    let versions = client
        .static_versions()
        .await
        .map_err(|_| "catalog_unavailable")?;
    let Some(version) = olc_catalog_cache::select_release(&patch.patch, &versions.versions) else {
        return Err("catalog_unavailable");
    };
    let directory = root(app)?;
    let worker_client = client.clone();
    let handle = tokio::runtime::Handle::current();
    // Le verrou interprocessus et les empreintes restent sur un worker dédié.
    let (mut cache, manifest) = tauri::async_runtime::spawn_blocking(move || {
        handle.block_on(async move {
            let mut cache = Cache::open(&directory).map_err(|_| "catalog_busy")?;
            let active = cache.active().map_err(|_| "catalog_storage")?;
            let prior = active
                .as_ref()
                .filter(|m| m.version == version)
                .map(|m| m.snapshot_id.as_str());
            let changed = worker_client
                .catalog_manifest(&version, prior)
                .await
                .map_err(|_| "catalog_unavailable")?;
            let manifest = match changed {
                Some(m) => m,
                None => active.ok_or("catalog_invalid")?,
            };
            worker_client
                .download_catalog(&mut cache, &manifest)
                .await
                .map_err(|_| "catalog_unavailable")?;
            olc_catalog_cache::validate_catalog(&cache, &manifest)
                .map_err(|_| "catalog_invalid")?;
            Ok::<_, &'static str>((cache, manifest))
        })
    })
    .await
    .map_err(|_| "catalog_storage")??;
    let live_patch = super::client_patch().await.map_err(|_| "catalog_changed")?;
    if live_patch.patch != patch.patch {
        return Err("catalog_changed");
    }
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<crate::api_access::ApiState>()
            .with_current_client(&client, || {
                pin(&app, &cache, &manifest)?;
                cache.activate(&manifest).map_err(|_| "catalog_storage")?;
                let _ = cache.cleanup();
                set(&app, CatalogRuntimeState::ready(&manifest));
                Ok(None)
            })
    })
    .await
    .map_err(|_| "catalog_storage")?
}
fn asset_request(path: &str) -> Result<(String, String), &'static str> {
    let path = percent_encoding::percent_decode_str(path)
        .decode_utf8()
        .map_err(|_| "catalog_invalid")?;
    let (id, file) = path
        .trim_start_matches('/')
        .split_once('/')
        .ok_or("catalog_invalid")?;
    if !olc_catalog_cache::valid_id(id)
        || !olc_catalog_cache::valid_path(file)
        || !(file.ends_with(".png") || file.ends_with(".jpg"))
    {
        return Err("catalog_invalid");
    }
    Ok((id.into(), file.into()))
}
pub fn protocol(
    ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let directory = root(ctx.app_handle());
    let path = asset_request(request.uri().path());
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = (|| {
            let root = directory?;
            let (id, path) = path?;
            let cache = Cache::reader(&root).map_err(|_| "catalog_storage")?;
            let bytes = cache.read(&id, &path).map_err(|_| "catalog_invalid")?;
            Ok::<_, &str>((
                bytes,
                if path.ends_with(".png") {
                    "image/png"
                } else {
                    "image/jpeg"
                },
            ))
        })();
        let (status, body, mime) = match bytes {
            Ok((b, m)) => (200, b, m),
            Err(_) => (404, vec![], "text/plain"),
        };
        let mut response = tauri::http::Response::new(body);
        *response.status_mut() = if status == 200 {
            tauri::http::StatusCode::OK
        } else {
            tauri::http::StatusCode::NOT_FOUND
        };
        response.headers_mut().insert(
            tauri::http::header::CONTENT_TYPE,
            tauri::http::HeaderValue::from_static(mime),
        );
        response.headers_mut().insert(
            tauri::http::header::CACHE_CONTROL,
            tauri::http::HeaderValue::from_static("private, max-age=31536000, immutable"),
        );
        responder.respond(response);
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn cosmetiques_absents_acceptes_mais_fichier_corrompu_refuse() {
        use olc_catalog_cache::{digest, snapshot_id, FileEntry};
        let tmp = tempfile::tempdir().unwrap();
        let mut cache = Cache::open(tmp.path()).unwrap();
        let bytes = b"{}";
        let file = FileEntry {
            bytes: 2,
            sha256: digest(bytes),
            media_type: "application/json".into(),
        };
        let mut m = Manifest {
            schema_version: 1,
            version: "16.19.1".into(),
            normalizer_version: 1,
            snapshot_id: String::new(),
            files: std::collections::BTreeMap::from([("other.json".into(), file.clone())]),
        };
        cache.put(&file, bytes).unwrap();
        m.snapshot_id = snapshot_id(&m).unwrap();
        cache.save(&m).unwrap();
        assert_eq!(
            read_json(&cache, &m.snapshot_id, "cosmetics.json"),
            Ok(serde_json::Value::Null)
        );
        assert!(read_json(&cache, &m.snapshot_id, "missing.json").is_err());
        m.files.insert("cosmetics.json".into(), file.clone());
        m.snapshot_id = snapshot_id(&m).unwrap();
        cache.save(&m).unwrap();
        std::fs::write(cache.object_path(&file.sha256), b"xx").unwrap();
        assert!(read_json(&cache, &m.snapshot_id, "cosmetics.json").is_err());
    }
    #[tokio::test]
    async fn toutes_les_fenetres_attendent_la_meme_initialisation() {
        let service = Arc::new(CatalogService::default());
        let (tx, rx) = tokio::sync::oneshot::channel();
        let a = service.clone();
        let first = tokio::spawn(async move {
            a.initialize(async {
                rx.await.unwrap();
                Some(CatalogRuntimeState {
                    version: Some("16.19.1".into()),
                    ..Default::default()
                })
            })
            .await;
        });
        tokio::task::yield_now().await;
        let b = service.clone();
        let second = tokio::spawn(async move {
            b.initialize(async { panic!("ne pas relancer le scan") })
                .await;
            b.value.lock().unwrap().version.clone()
        });
        tokio::task::yield_now().await;
        assert!(!second.is_finished());
        tx.send(()).unwrap();
        first.await.unwrap();
        assert_eq!(second.await.unwrap().as_deref(), Some("16.19.1"));
    }
    #[test]
    fn le_protocole_refuse_les_chemins_hors_instantane() {
        let id = "a".repeat(64);
        assert_eq!(
            asset_request(&format!("/{id}/catalog/icons/a.png")).unwrap(),
            (id.clone(), "catalog/icons/a.png".into())
        );
        for path in [
            format!("/{id}/../secret.json"),
            format!("/{id}/%2e%2e/secret.json"),
            "/bad/catalog/icons/a.png".into(),
            format!("/{id}/catalog/fr_FR.json"),
        ] {
            assert!(asset_request(&path).is_err());
        }
    }
    #[test]
    fn erreur_conserve_la_version_et_les_assets_actifs() {
        let mut s = CatalogRuntimeState {
            status: "ready".into(),
            version: Some("16.19.1".into()),
            snapshot_id: Some("a".repeat(64)),
            asset_base: Some("catalog://localhost/a/".into()),
            error: None,
        };
        s.fail("unavailable");
        assert_eq!(s.status, "error");
        assert_eq!(s.version.as_deref(), Some("16.19.1"));
        assert!(s.asset_base.is_some());
    }
}
