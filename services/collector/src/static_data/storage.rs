use serde::Serialize;
use serde_json::{json, Value};
use sqlx::Row;

use super::download::{download_catalogs, download_release, validate_bundle, ReleaseBundle};
use super::model::select_versions;
use super::transport::{fetch_json, PublicHttp};
use super::{StaticError, StaticTransport};
use crate::storage::Storage;

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseSummary {
    pub version: String,
    pub patch: String,
    pub completed_at: String,
    pub documents: usize,
    pub champions: usize,
    pub classic_champions: usize,
    pub reused: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncReport {
    pub live_version: String,
    pub checked_at: String,
    pub releases: Vec<ReleaseSummary>,
}

/// Vérifie la version EUW courante et publie les derniers patchs distincts complets.
pub async fn sync_recent(storage: &Storage, patch_count: usize) -> Result<SyncReport, StaticError> {
    sync_recent_refresh(storage, patch_count, false).await
}

/// `refresh` force le téléchargement des mêmes versions, sans écraser le cache en cas d'échec.
pub async fn sync_recent_refresh(
    storage: &Storage,
    patch_count: usize,
    refresh: bool,
) -> Result<SyncReport, StaticError> {
    check_count(patch_count)?;
    sync_with_transport(storage, patch_count, PublicHttp::new()?, refresh).await
}

fn check_count(count: usize) -> Result<(), StaticError> {
    if !(1..=10).contains(&count) {
        Err(StaticError::InvalidCount)
    } else {
        Ok(())
    }
}

/// Variante injectable : seul le transport HTTP est remplacé, la validation et SQL restent réels.
pub async fn sync_with_transport<T: StaticTransport>(
    storage: &Storage,
    patch_count: usize,
    transport: T,
    refresh: bool,
) -> Result<SyncReport, StaticError> {
    check_count(patch_count)?;
    let mut connection = storage.transaction_connection().await?;
    let mut tx = connection.begin().await?;
    // Ce verrou transactionnel couvre aussi les téléchargements : deux synchronisations
    // ne peuvent pas publier leurs manifestes dans l'ordre inverse de leur observation.
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(0x0018_DDAA_0004_i64)
        .fetch_one(&mut *tx)
        .await?;
    if !acquired {
        return Err(StaticError::Busy);
    }
    let versions = fetch_json(
        transport.clone(),
        "https://ddragon.leagueoflegends.com/api/versions.json".into(),
    )
    .await?;
    let realm = fetch_json(
        transport.clone(),
        "https://ddragon.leagueoflegends.com/realms/euw.json".into(),
    )
    .await?;
    let selected = select_versions(&versions, &realm, patch_count)?;
    let live_version = realm
        .get("v")
        .and_then(Value::as_str)
        .ok_or(StaticError::InvalidManifest)?
        .to_owned();
    let mut releases = Vec::new();
    for version in &selected {
        let cached = if refresh {
            None
        } else {
            sqlx::query("SELECT bundle, patch, completed_at::text FROM static_data_releases WHERE version=$1")
                .bind(version).fetch_optional(&mut *tx).await?
        };
        let (bundle, reused, completed_at) = if let Some(row) = cached {
            let bundle: Value = row.try_get("bundle")?;
            let bundle: ReleaseBundle =
                serde_json::from_value(bundle).map_err(|_| StaticError::InvalidDocument)?;
            validate_bundle(&bundle, version)?;
            let stored_patch: String = row.try_get("patch")?;
            if stored_patch != bundle.patch {
                return Err(StaticError::InvalidDocument);
            }
            (bundle, true, row.try_get::<String, _>("completed_at")?)
        } else {
            let bundle = download_release(transport.clone(), version).await?;
            let data = serde_json::to_value(&bundle).map_err(|_| StaticError::InvalidDocument)?;
            let completed_at: String = sqlx::query_scalar(
                "INSERT INTO static_data_releases (version,patch,completed_at,bundle)
                VALUES ($1,$2,clock_timestamp(),$3) ON CONFLICT (version) DO UPDATE
                SET patch=EXCLUDED.patch, completed_at=EXCLUDED.completed_at, bundle=EXCLUDED.bundle
                RETURNING completed_at::text",
            )
            .bind(version)
            .bind(&bundle.patch)
            .bind(data)
            .fetch_one(&mut *tx)
            .await?;
            (bundle, false, completed_at)
        };
        releases.push(ReleaseSummary {
            version: bundle.version,
            patch: bundle.patch,
            completed_at,
            documents: bundle.documents.len(),
            champions: bundle.champion_count,
            classic_champions: bundle.classic_champion_count,
            reused,
        });
    }
    // Ces catalogues officiels n'ont pas de version de patch : fraîcheur explicite du manifeste.
    let catalogs = download_catalogs(transport).await?;
    let selected = serde_json::to_value(selected).map_err(|_| StaticError::InvalidManifest)?;
    let catalogs = serde_json::to_value(catalogs).map_err(|_| StaticError::InvalidDocument)?;
    let checked_at: String = sqlx::query_scalar("INSERT INTO static_data_manifest (id,live_version,checked_at,versions,catalogs)
        VALUES (1,$1,clock_timestamp(),$2,$3) ON CONFLICT (id) DO UPDATE SET live_version=EXCLUDED.live_version,
        checked_at=EXCLUDED.checked_at, versions=EXCLUDED.versions, catalogs=EXCLUDED.catalogs RETURNING checked_at::text")
        .bind(&live_version).bind(selected).bind(catalogs).fetch_one(&mut *tx).await?;
    // Une interruption après l'envoi de COMMIT peut rendre le résultat inconnu ; pas de faux succès.
    tx.commit().await?;
    Ok(SyncReport {
        live_version,
        checked_at,
        releases,
    })
}

/// Patchs du dernier manifeste valide, sans réseau ni prétention de fraîcheur supplémentaire.
pub async fn cached_patches(storage: &Storage, count: usize) -> Result<Vec<String>, StaticError> {
    check_count(count)?;
    let mut connection = storage.transaction_connection().await?;
    let mut tx = connection.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let row = sqlx::query("SELECT versions,live_version FROM static_data_manifest WHERE id=1")
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(StaticError::CacheMissing)?;
    let versions: Value = row.try_get("versions")?;
    let live: String = row.try_get("live_version")?;
    let versions = select_versions(&versions, &json!({"v":live}), count)
        .map_err(|_| StaticError::CacheMissing)?;
    let mut patches = Vec::new();
    for version in versions {
        let row = sqlx::query("SELECT bundle,patch FROM static_data_releases WHERE version=$1")
            .bind(&version)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StaticError::CacheMissing)?;
        let raw: Value = row.try_get("bundle")?;
        let bundle: ReleaseBundle =
            serde_json::from_value(raw).map_err(|_| StaticError::CacheMissing)?;
        validate_bundle(&bundle, &version).map_err(|_| StaticError::CacheMissing)?;
        let patch: String = row.try_get("patch")?;
        if patch != bundle.patch {
            return Err(StaticError::CacheMissing);
        }
        patches.push(patch);
    }
    tx.commit().await?;
    Ok(patches)
}
