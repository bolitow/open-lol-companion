//! Retraitement du cache #18 et enrichissement facultatif explicitement déclaré.
use super::*;
use crate::storage::Storage;
use serde_json::Value;
use sqlx::Row;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum CommunityPolicy {
    Off,
    Optional,
    Required,
}

/// Valide puis transforme le cache brut existant en sources immuables candidates.
pub async fn cached_sources(
    storage: &Storage,
    version: &str,
) -> Result<Vec<CatalogSource>, CatalogError> {
    if !valid_version(version) {
        return Err(CatalogError::InvalidRequest);
    }
    let row=sqlx::query("SELECT r.bundle,r.completed_at::text AS observed_at,m.catalogs,m.checked_at::text AS catalogs_at FROM static_data_releases r CROSS JOIN static_data_manifest m WHERE r.version=$1 AND m.id=1")
        .bind(version).fetch_optional(storage.pool()).await?.ok_or(CatalogError::NotFound)?;
    let bundle: Value = row.try_get("bundle")?;
    crate::static_data::validate_cached_bundle(version, &bundle)
        .map_err(|_| CatalogError::InvalidSource)?;
    let observed_at: String = row.try_get("observed_at")?;
    let mut result = Vec::new();
    let documents = bundle["documents"]
        .as_object()
        .ok_or(CatalogError::InvalidSource)?;
    for (key, document) in documents {
        let locale = key.split('/').next().ok_or(CatalogError::InvalidSource)?;
        let url = document["url"]
            .as_str()
            .ok_or(CatalogError::InvalidSource)?;
        result.push(make_source(
            "ddragon",
            key,
            version,
            Some(locale),
            url,
            &observed_at,
            document["data"].clone(),
        ));
    }
    let catalogs: Value = row.try_get("catalogs")?;
    let catalogs_at: String = row.try_get("catalogs_at")?;
    for (key, document) in catalogs.as_object().ok_or(CatalogError::InvalidSource)? {
        let url = document["url"]
            .as_str()
            .ok_or(CatalogError::InvalidSource)?;
        // Catalogue global daté, sans attribution mensongère au patch du jeu.
        result.push(make_source(
            "riot_catalog",
            key,
            "unversioned",
            None,
            url,
            &catalogs_at,
            document["data"].clone(),
        ));
    }
    Ok(result)
}

/// Projette les sources en mémoire et vérifie les relations après enrichissement.
pub fn project_sources(
    version: &str,
    sources: Vec<CatalogSource>,
    degraded: bool,
    warnings: Vec<String>,
) -> Result<CatalogProjection, CatalogError> {
    let mut records = normalize::normalize(version, &sources)?;
    if sources.iter().any(|s| s.provider == "cdragon") {
        community::enrich(version, &mut records, &sources)?;
    }
    normalize::finalize(&mut records);
    Ok(CatalogProjection {
        version: version.into(),
        sources,
        records,
        degraded,
        warnings,
    })
}

/// Construit depuis le cache brut ; seuls les compléments publics peuvent être téléchargés.
pub async fn build(
    storage: &Storage,
    version: &str,
    policy: CommunityPolicy,
    refresh: bool,
) -> Result<CatalogManifest, CatalogError> {
    let mut connection = storage.transaction_connection().await?;
    let mut tx = connection.begin().await?;
    let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(0x0061_CA7A_0002_i64)
        .fetch_one(&mut *tx)
        .await?;
    if !locked {
        return Err(CatalogError::Busy);
    }
    let sources = cached_sources(storage, version).await?;
    let mut supplement = Ok(Vec::new());
    if policy != CommunityPolicy::Off {
        if !refresh {
            let current: Option<String> = sqlx::query_scalar(
                "SELECT publication_id FROM game_catalog_current WHERE version=$1",
            )
            .bind(version)
            .fetch_optional(storage.pool())
            .await?;
            if let Some(current) = current {
                supplement = Ok(archived_sources(storage, &current)
                    .await?
                    .into_iter()
                    .filter(|s| s.provider == "cdragon")
                    .collect());
            }
        }
        if supplement.as_ref().is_ok_and(Vec::is_empty) {
            supplement = community::fetch_sources(version).await;
        }
    }
    let projection = combine_sources(version, sources, policy, supplement)?;
    let manifest = publish(storage, &projection).await?;
    tx.commit().await?;
    Ok(manifest)
}

/// Point de raccordement testable : seuls les échecs réseau d'un complément facultatif sont dégradables.
pub fn combine_sources(
    version: &str,
    mut sources: Vec<CatalogSource>,
    policy: CommunityPolicy,
    supplement: Result<Vec<CatalogSource>, CatalogError>,
) -> Result<CatalogProjection, CatalogError> {
    let mut warnings = Vec::new();
    match policy {
        CommunityPolicy::Off => warnings.push("community_disabled".into()),
        CommunityPolicy::Required | CommunityPolicy::Optional => match supplement {
            Ok(extra) if !extra.is_empty() => sources.extend(extra),
            Err(CatalogError::Network) if policy == CommunityPolicy::Optional => {
                warnings.push("community_unavailable".into())
            }
            Err(error) => return Err(error),
            _ => return Err(CatalogError::InvalidSource),
        },
    }
    project_sources(version, sources, !warnings.is_empty(), warnings)
}

/// Rejoue les sources archivées avec le normaliseur courant, sans aucun accès réseau.
pub async fn rebuild(
    storage: &Storage,
    publication: &str,
) -> Result<CatalogManifest, CatalogError> {
    let raw: Value =
        sqlx::query_scalar("SELECT manifest FROM game_catalog_publications WHERE id=$1")
            .bind(publication)
            .fetch_optional(storage.pool())
            .await?
            .ok_or(CatalogError::NotFound)?;
    let old: CatalogManifest =
        serde_json::from_value(raw).map_err(|_| CatalogError::InvalidSource)?;
    let sources = archived_sources(storage, publication).await?;
    let projection = project_sources(&old.version, sources, old.degraded, old.warnings)?;
    super::storage::publish_rebuild(storage, &projection, publication).await
}
