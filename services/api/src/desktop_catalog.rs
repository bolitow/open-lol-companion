//! Distribution publique des instantanés vérifiés, sans chemin disque exposé.
use crate::{error::ApiError, server::AppState};
use axum::{
    body::Body,
    extract::{rejection::PathRejection, Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use olc_catalog_cache::{valid_id, valid_path, version_parts, Cache};
fn error(error: olc_catalog_cache::Error) -> ApiError {
    match error {
        olc_catalog_cache::Error::Storage(e) if e.kind() == std::io::ErrorKind::NotFound => {
            ApiError::NotFound
        }
        _ => ApiError::Unavailable,
    }
}
fn response(
    bytes: Vec<u8>,
    media: &str,
    headers: &HeaderMap,
    immutable: bool,
    digest: &str,
) -> Result<Response, ApiError> {
    let tag = format!("\"{digest}\"");
    let matched = headers
        .get_all("if-none-match")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .any(|v| {
            let v = v.trim();
            v == "*" || v.strip_prefix("W/").unwrap_or(v) == tag
        });
    let mut response = if matched {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        Response::new(Body::from(bytes))
    };
    response.headers_mut().insert(
        "etag",
        HeaderValue::from_str(&tag).map_err(|_| ApiError::Unavailable)?,
    );
    response.headers_mut().insert(
        "content-type",
        HeaderValue::from_str(media).map_err(|_| ApiError::Unavailable)?,
    );
    response.headers_mut().insert(
        "cache-control",
        HeaderValue::from_static(if immutable {
            "public, max-age=31536000, immutable"
        } else {
            "public, max-age=60, must-revalidate"
        }),
    );
    Ok(response)
}
pub async fn manifest(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Path(version) = path.map_err(|_| ApiError::InvalidRequest)?;
    if version_parts(&version).is_none() {
        return Err(ApiError::InvalidRequest);
    }
    let root = state.desktop_catalog_dir.ok_or(ApiError::Unavailable)?;
    let (bytes, snapshot_id) = tokio::task::spawn_blocking(move || {
        let cache = Cache::reader(&root).map_err(error)?;
        let manifest = cache.release(&version).map_err(error)?;
        let bytes = serde_json::to_vec(&manifest).map_err(|_| ApiError::Unavailable)?;
        Ok::<_, ApiError>((bytes, manifest.snapshot_id))
    })
    .await
    .map_err(|_| ApiError::Unavailable)??;
    response(bytes, "application/json", &headers, false, &snapshot_id)
}
type FilePath = Result<Path<(String, String)>, PathRejection>;
pub async fn file(
    State(state): State<AppState>,
    path: FilePath,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Path((id, path)) = path.map_err(|_| ApiError::InvalidRequest)?;
    if !valid_id(&id) || !valid_path(&path) {
        return Err(ApiError::InvalidRequest);
    }
    let root = state.desktop_catalog_dir.ok_or(ApiError::Unavailable)?;
    let (bytes, media) = tokio::task::spawn_blocking(move || {
        let cache = Cache::reader(&root).map_err(error)?;
        let manifest = cache.manifest(&id).map_err(error)?;
        let entry = manifest.files.get(&path).ok_or(ApiError::NotFound)?;
        let bytes = cache.read(&id, &path).map_err(error)?;
        Ok::<_, ApiError>((bytes, entry.media_type.clone()))
    })
    .await
    .map_err(|_| ApiError::Unavailable)??;
    let digest = olc_catalog_cache::digest(&bytes);
    response(bytes, &media, &headers, true, &digest)
}
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    #[tokio::test]
    async fn parametres_invalides_refuses_avant_disque() {
        let state = AppState::new(
            sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://localhost/unused")
                .unwrap(),
            crate::auth::Auth::new(b"synthetic-signing-material-for-http-tests", "test", "api")
                .unwrap(),
        );
        for path in [
            "/v1/desktop-catalog/bad/manifest",
            "/v1/desktop-catalog/snapshots/bad/catalog/fr_FR.json",
        ] {
            let r = crate::server::router(state.clone())
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        }
    }
    #[tokio::test]
    async fn publication_immuable_revalidation_et_corruption() {
        use olc_catalog_cache::{digest, snapshot_id, FileEntry, Manifest};
        let dir = tempfile::tempdir().unwrap();
        let mut cache = Cache::open(dir.path()).unwrap();
        let first = b"{\"version\":1}";
        let entry = FileEntry {
            bytes: first.len() as u64,
            sha256: digest(first),
            media_type: "application/json".into(),
        };
        let mut m = Manifest {
            schema_version: 1,
            version: "16.19.1".into(),
            normalizer_version: 1,
            snapshot_id: String::new(),
            files: std::collections::BTreeMap::from([("champions.json".into(), entry.clone())]),
        };
        m.snapshot_id = snapshot_id(&m).unwrap();
        cache.put(&entry, first).unwrap();
        cache.publish(&m).unwrap();
        let mut state = AppState::new(
            sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://localhost/unused")
                .unwrap(),
            crate::auth::Auth::new(b"synthetic-signing-material-for-http-tests", "test", "api")
                .unwrap(),
        );
        state.desktop_catalog_dir = Some(dir.path().into());
        let app = crate::server::router(state);
        let release = "/v1/desktop-catalog/16.19.1/manifest";
        let r = app
            .clone()
            .oneshot(Request::builder().uri(release).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let etag = r.headers()["etag"].clone();
        assert_eq!(etag, format!("\"{}\"", m.snapshot_id));
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(release)
                    .header("if-none-match", format!("W/\"{}\"", m.snapshot_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::NOT_MODIFIED);
        assert!(axum::body::to_bytes(r.into_body(), 10)
            .await
            .unwrap()
            .is_empty());
        let old_path = format!(
            "/v1/desktop-catalog/snapshots/{}/champions.json",
            m.snapshot_id
        );
        let second = b"{\"version\":2}";
        let second_entry = FileEntry {
            bytes: second.len() as u64,
            sha256: digest(second),
            media_type: "application/json".into(),
        };
        m.files
            .insert("champions.json".into(), second_entry.clone());
        m.snapshot_id = snapshot_id(&m).unwrap();
        cache.put(&second_entry, second).unwrap();
        cache.publish(&m).unwrap();
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(release)
                    .header("if-none-match", etag)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&old_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(
            r.headers()["cache-control"],
            "public, max-age=31536000, immutable"
        );
        assert_eq!(
            &axum::body::to_bytes(r.into_body(), 1024).await.unwrap()[..],
            first
        );
        std::fs::write(cache.object_path(&entry.sha256), b"bad").unwrap();
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&old_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(r.into_body(), 1024).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains(dir.path().to_str().unwrap()));
        let r = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/v1/desktop-catalog/snapshots/{}/missing.json",
                        m.snapshot_id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
    }
}
