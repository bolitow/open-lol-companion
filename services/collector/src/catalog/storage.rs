//! Publications atomiques et archives de sources du catalogue.
use super::*;
use crate::storage::Storage;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet};

/// Archive les sources et publie toutes les entités dans la même transaction.
/// Une coupure au COMMIT reste une erreur : aucun succès non confirmé n'est annoncé.
pub async fn publish(
    storage: &Storage,
    projection: &CatalogProjection,
) -> Result<CatalogManifest, CatalogError> {
    publish_with_head(storage, projection, None).await
}

/// Une reconstruction historique ne peut remplacer qu'une tête issue de sa source.
pub(super) async fn publish_rebuild(
    storage: &Storage,
    projection: &CatalogProjection,
    original: &str,
) -> Result<CatalogManifest, CatalogError> {
    publish_with_head(storage, projection, Some(original)).await
}

async fn publish_with_head(
    storage: &Storage,
    projection: &CatalogProjection,
    original: Option<&str>,
) -> Result<CatalogManifest, CatalogError> {
    validate_projection(projection)?;
    let mut records: Vec<_> = projection.records.iter().collect();
    records.sort_by_key(|r| (&r.kind, &r.namespace, &r.id, &r.locale));
    let mut sources: Vec<_> = projection.sources.iter().collect();
    sources.sort_by_key(|s| &s.id);
    let mut digest = Sha256::new();
    digest.update(
        json!([
            projection.version,
            NORMALIZER_VERSION,
            projection.degraded,
            projection.warnings
        ])
        .to_string()
        .as_bytes(),
    );
    for source in &sources {
        digest.update(source.id.as_bytes());
    }
    for record in &records {
        let bytes = serde_json::to_vec(record).map_err(|_| CatalogError::InvalidSource)?;
        // Borne par entité, indépendante de la taille du catalogue entier.
        if bytes.len() > 2 * 1024 * 1024 {
            return Err(CatalogError::InvalidSource);
        }
        digest.update(bytes);
    }
    let id = format!("{:x}", digest.finalize());
    let mut connection = storage.transaction_connection().await?;
    let mut tx = connection.begin().await?;
    let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(0x0061_CA7A_0001_i64)
        .fetch_one(&mut *tx)
        .await?;
    if !locked {
        return Err(CatalogError::Busy);
    }
    let existing: Option<Value> =
        sqlx::query_scalar("SELECT manifest FROM game_catalog_publications WHERE id=$1")
            .bind(&id)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some(existing) = existing {
        // Ne pas repointer vers une publication historique lors d'une simple répétition.
        let manifest = serde_json::from_value(existing).map_err(|_| CatalogError::InvalidSource)?;
        tx.commit().await?;
        return Ok(manifest);
    }
    let now: String = sqlx::query_scalar("SELECT clock_timestamp()::text")
        .fetch_one(&mut *tx)
        .await?;
    let mut metadata = Vec::new();
    for source in sources {
        // Le bind JSONB de SQLx normaliserait -0.0 avant le cast : passer explicitement du texte.
        let document = serde_json::to_string(source).map_err(|_| CatalogError::InvalidSource)?;
        sqlx::query(
            "INSERT INTO game_catalog_sources(id, document) VALUES($1,$2::text::json) ON CONFLICT DO NOTHING",
        )
        .bind(&source.id)
        .bind(document)
        .execute(&mut *tx)
        .await?;
        let archived: Value =
            sqlx::query_scalar("SELECT document FROM game_catalog_sources WHERE id=$1")
                .bind(&source.id)
                .fetch_one(&mut *tx)
                .await?;
        let archived: CatalogSource =
            serde_json::from_value(archived).map_err(|_| CatalogError::InvalidSource)?;
        metadata.push(super::source::source_meta(&archived));
    }
    let mut coverage = CatalogCoverage::default();
    for record in &records {
        coverage.records += 1;
        coverage.source_fields += u64::from(record.coverage.source_fields);
        coverage.normalized_fields += u64::from(record.coverage.normalized_fields);
        coverage.unmapped_fields += record.coverage.unmapped_fields.len() as u64;
        coverage.records_with_issues += usize::from(!record.coverage.issues.is_empty());
        *coverage.by_kind.entry(record.kind.clone()).or_default() += 1;
    }
    let manifest = CatalogManifest {
        publication_id: id.clone(),
        version: projection.version.clone(),
        normalizer_version: NORMALIZER_VERSION,
        published_at: now.clone(),
        degraded: projection.degraded,
        warnings: projection.warnings.clone(),
        sources: metadata,
        coverage,
        source_inventory: inventory(&projection.sources, &projection.records),
    };
    sqlx::query("INSERT INTO game_catalog_publications(id,version,normalizer_version,published_at,manifest) VALUES($1,$2,$3,$4::text::timestamptz,$5)")
        .bind(&id).bind(&projection.version).bind(NORMALIZER_VERSION as i32).bind(&now)
        .bind(serde_json::to_value(&manifest).map_err(|_| CatalogError::InvalidSource)?)
        .execute(&mut *tx).await?;
    for source in &projection.sources {
        sqlx::query("INSERT INTO game_catalog_source_refs(publication_id,source_id) VALUES($1,$2)")
            .bind(&id)
            .bind(&source.id)
            .execute(&mut *tx)
            .await?;
    }
    for record in records {
        sqlx::query("INSERT INTO game_catalog_entries(publication_id,kind,id,namespace,locale,name,data) VALUES($1,$2,$3,$4,$5,$6,$7)")
            .bind(&id).bind(&record.kind).bind(&record.id).bind(&record.namespace).bind(&record.locale).bind(&record.name)
            .bind(serde_json::to_value(record).map_err(|_| CatalogError::InvalidSource)?)
            .execute(&mut *tx).await?;
    }
    if let Some(original) = original {
        // CAS : une révision plus récente arrivée pendant le replay garde la priorité.
        sqlx::query("UPDATE game_catalog_current SET publication_id=$2 WHERE version=$1 AND publication_id=$3")
            .bind(&projection.version).bind(&id).bind(original).execute(&mut *tx).await?;
    } else {
        sqlx::query("INSERT INTO game_catalog_current(version,publication_id) VALUES($1,$2) ON CONFLICT(version) DO UPDATE SET publication_id=EXCLUDED.publication_id")
            .bind(&projection.version).bind(&id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(manifest)
}

fn validate_projection(projection: &CatalogProjection) -> Result<(), CatalogError> {
    if !valid_version(&projection.version)
        || projection.sources.is_empty()
        || projection.records.is_empty()
    {
        return Err(CatalogError::InvalidSource);
    }
    let mut ids = BTreeSet::new();
    let mut documents = BTreeMap::new();
    for source in &projection.sources {
        if !ids.insert(source.id.as_str()) {
            return Err(CatalogError::InvalidSource);
        }
        let computed = make_source(
            &source.provider,
            &source.key,
            &source.version,
            source.locale.as_deref(),
            &source.url,
            &source.observed_at,
            source.data.clone(),
        );
        if source.id != computed.id {
            return Err(CatalogError::InvalidSource);
        }
        documents.insert(source.id.as_str(), &source.data);
    }
    let mut keys = BTreeSet::new();
    for record in &projection.records {
        if record.id.is_empty()
            || !matches!(record.locale.as_str(), "fr_FR" | "en_US" | "und")
            || !matches!(record.namespace.as_str(), "standard" | "classic" | "global")
            || !keys.insert((&record.kind, &record.id, &record.namespace, &record.locale))
        {
            return Err(CatalogError::InvalidSource);
        }
        let values = record.fields.values().chain(record.stats.values()).chain(
            record
                .effects
                .iter()
                .flat_map(|e| e.parameters.values().chain(e.calculation.iter())),
        );
        for value in values {
            if value.sources.iter().any(|s| {
                documents
                    .get(s.source_id.as_str())
                    .and_then(|data| data.pointer(&s.pointer))
                    .is_none()
            }) {
                return Err(CatalogError::InvalidSource);
            }
        }
    }
    Ok(())
}

/// Relit les sources exactes d'une publication, même après un refresh du cache #18.
pub async fn archived_sources(
    storage: &Storage,
    publication: &str,
) -> Result<Vec<CatalogSource>, CatalogError> {
    let rows = sqlx::query("SELECT s.document FROM game_catalog_source_refs r JOIN game_catalog_sources s ON s.id=r.source_id WHERE r.publication_id=$1 ORDER BY s.id")
        .bind(publication).fetch_all(storage.pool()).await?;
    if rows.is_empty() {
        return Err(CatalogError::NotFound);
    }
    rows.into_iter()
        .map(|row| {
            let data: Value = row.try_get("document")?;
            serde_json::from_value(data).map_err(|_| CatalogError::InvalidSource)
        })
        .collect()
}
