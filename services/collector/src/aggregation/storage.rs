use std::collections::BTreeMap;

use sqlx::{PgConnection, Row};

use super::model::{Accumulator, ScopeKey, StoredMatch};
use super::stages::ItemCatalog;
use super::{AggregationError, AggregationOptions, AggregationReport, QualityThresholds};
use crate::storage::Storage;

/// Clé du verrou consultatif des calculs, distincte de celle du collecteur : l'ingestion
/// peut continuer. Partagée par le recalcul complet et le recalcul par lots (#89).
pub(super) const AGGREGATION_LOCK_KEY: i64 = 0x0018_A660_0001;

/// Paramètres d'un calcul : le recalcul complet et chaque lot (#89) construisent leurs
/// accumulateurs par la même fonction, condition de l'égalité des deux résultats.
#[derive(Debug, Clone)]
pub(super) struct Settings {
    pub min_games: u32,
    pub rank_max_age_hours: u32,
    pub quality: QualityThresholds,
    pub filters: AggregationOptions,
}

impl Settings {
    pub(super) fn accumulator(
        &self,
        catalogs: BTreeMap<String, ItemCatalog>,
    ) -> Result<Accumulator, AggregationError> {
        let mut accumulator = Accumulator::new(self.min_games)?;
        accumulator.set_rank_max_age_hours(self.rank_max_age_hours)?;
        accumulator.set_quality_thresholds(&self.quality)?;
        accumulator.set_filters(self.filters.clone());
        accumulator.set_item_catalogs(catalogs);
        Ok(accumulator)
    }

    /// Filtres et seuils refusés avant toute connexion : l'ancien instantané reste publié.
    pub(super) fn validate(&self) -> Result<(), AggregationError> {
        let filters = &self.filters;
        if filters
            .start_ms
            .zip(filters.end_ms)
            .is_some_and(|(s, e)| s >= e)
            || filters
                .patches
                .iter()
                .any(|p| crate::model::patch_from_version(p).as_ref() != Some(p))
            || filters.queues.iter().any(|q| *q <= 0)
            || filters
                .platforms
                .iter()
                .any(|p| !crate::config::PLATFORMS.contains(&p.as_str()))
        {
            return Err(AggregationError::InvalidFilters);
        }
        self.accumulator(BTreeMap::new()).map(|_| ())
    }
}

/// Colonnes et rang figé (#80) d'une partie, communs aux deux modes de calcul.
/// Rang figé à la partie : observation la plus proche du début, quelle que soit l'heure
/// du calcul ; à écart égal, la plus ancienne. L'écart maximal est appliqué par
/// l'accumulateur pour que la règle reste unique et testable sans base.
// Rang figé à la partie (#80) : observation la plus proche du début, quelle que soit
// l'heure du calcul ; à écart égal, la plus ancienne. L'écart maximal est appliqué
// par l'accumulateur pour que la règle reste unique et testable sans base.
// Deux recherches bornées par l'index (platform_id, puuid, queue_id, observed_at) — la
// dernière observation jusqu'au début, la première après — plutôt qu'un tri de tout
// l'historique ; l'écart est arrondi à la seconde supérieure pour que la borne incluse
// soit exacte (48 h + 400 ms ne devient pas 48 h).
const MATCH_SELECT: &str = "SELECT m.match_id,m.platform_id,m.queue_id,m.patch,m.is_remake,m.game_duration_s,m.detail,t.timeline,
            (extract(epoch FROM m.game_start)*1000)::bigint AS game_start_ms,
            COALESCE((SELECT jsonb_object_agg(p->>'puuid',jsonb_build_object('status',r.status,'tier',r.tier,'gap_s',r.gap_s))
                FROM jsonb_array_elements(CASE WHEN jsonb_typeof(m.detail#>'{info,participants}')='array'
                    THEN m.detail#>'{info,participants}' ELSE '[]'::jsonb END) p
                JOIN LATERAL (SELECT c.status,c.tier,ceil(abs(extract(epoch FROM c.observed_at-m.game_start)))::bigint AS gap_s
                    FROM (
                        (SELECT o.status,o.tier,o.observed_at,o.id FROM participant_rank_observations o
                            WHERE o.platform_id=m.platform_id AND o.queue_id=m.queue_id AND o.puuid=p->>'puuid'
                                AND o.observed_at<=m.game_start
                            ORDER BY o.observed_at DESC,o.id DESC LIMIT 1)
                        UNION ALL
                        (SELECT o.status,o.tier,o.observed_at,o.id FROM participant_rank_observations o
                            WHERE o.platform_id=m.platform_id AND o.queue_id=m.queue_id AND o.puuid=p->>'puuid'
                                AND o.observed_at>m.game_start
                            ORDER BY o.observed_at,o.id DESC LIMIT 1)
                    ) c
                    ORDER BY abs(extract(epoch FROM c.observed_at-m.game_start)),c.observed_at,c.id DESC LIMIT 1) r ON true), '{}'::jsonb) AS ranks
            FROM matches m LEFT JOIN match_timelines t ON t.match_id=m.match_id AND t.status='available'";

fn stored_match(row: &sqlx::postgres::PgRow) -> Result<StoredMatch, AggregationError> {
    Ok(StoredMatch {
        match_id: row.try_get("match_id")?,
        platform_id: row.try_get("platform_id")?,
        queue_id: row.try_get("queue_id")?,
        patch: row.try_get("patch")?,
        is_remake: row.try_get("is_remake")?,
        game_duration_s: row.try_get("game_duration_s")?,
        detail: row.try_get("detail")?,
        timeline: row.try_get("timeline")?,
        ranks: serde_json::from_value(row.try_get("ranks")?)?,
        game_start_ms: row.try_get("game_start_ms")?,
    })
}

/// Parties d'un lot (#89) après `after` (vide pour la première page), par pages de 25 sur
/// l'index (patch, plateforme, file, identifiant) : mémoire des détails bornée.
pub(super) async fn lot_page(
    connection: &mut PgConnection,
    scope: &ScopeKey,
    filters: &AggregationOptions,
    after: &str,
) -> Result<Vec<StoredMatch>, AggregationError> {
    let rows = sqlx::query(&format!(
        "{MATCH_SELECT}
            WHERE m.patch=$1 AND m.platform_id=$2 AND m.queue_id=$3 AND m.match_id>$4
                AND ($5::bigint IS NULL OR m.game_start>=to_timestamp($5::float8/1000))
                AND ($6::bigint IS NULL OR m.game_start<to_timestamp($6::float8/1000))
            ORDER BY m.match_id LIMIT 25"
    ))
    .bind(&scope.patch)
    .bind(&scope.platform_id)
    .bind(scope.queue_id)
    .bind(after)
    .bind(filters.start_ms)
    .bind(filters.end_ms)
    .fetch_all(&mut *connection)
    .await?;
    rows.iter().map(stored_match).collect()
}

/// Recalcule les données stockées sans filtre (usage local et tests).
pub async fn recalculate(
    storage: &Storage,
    min_games: u32,
) -> Result<AggregationReport, AggregationError> {
    recalculate_filtered(
        storage,
        min_games,
        super::DEFAULT_RANK_MAX_AGE_HOURS,
        &super::AggregationOptions::default(),
    )
    .await
}

/// Recalcule et publie atomiquement une sélection explicite, sans appel réseau.
pub async fn recalculate_filtered(
    storage: &Storage,
    min_games: u32,
    rank_max_age_hours: u32,
    filters: &super::AggregationOptions,
) -> Result<AggregationReport, AggregationError> {
    recalculate_with_quality(
        storage,
        min_games,
        rank_max_age_hours,
        filters,
        &super::QualityThresholds::default(),
    )
    .await
}

/// Comme `recalculate_filtered`, avec les seuils des contrôles de qualité classés (#111).
pub async fn recalculate_with_quality(
    storage: &Storage,
    min_games: u32,
    rank_max_age_hours: u32,
    filters: &super::AggregationOptions,
    quality: &super::QualityThresholds,
) -> Result<AggregationReport, AggregationError> {
    let settings = Settings {
        min_games,
        rank_max_age_hours,
        quality: *quality,
        filters: filters.clone(),
    };
    settings.validate()?;
    let mut connection = storage.transaction_connection().await?;
    let mut tx = connection.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    // Le verrou suit cette transaction/connexion ; aucune session séparée à maintenir.
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(AGGREGATION_LOCK_KEY)
        .fetch_one(&mut *tx)
        .await?;
    if !acquired {
        return Err(AggregationError::Busy);
    }
    let mut accumulator =
        settings.accumulator(load_item_catalogs(&mut tx, &filters.patches).await?)?;

    let page = format!(
        "{MATCH_SELECT}
            WHERE ($1::text IS NULL OR m.match_id > $1)
                AND (cardinality($2::text[])=0 OR m.patch=ANY($2))
                AND (cardinality($3::text[])=0 OR m.platform_id=ANY($3))
                AND (cardinality($4::int[])=0 OR m.queue_id=ANY($4))
                AND ($5::bigint IS NULL OR m.game_start>=to_timestamp($5::float8/1000))
                AND ($6::bigint IS NULL OR m.game_start<to_timestamp($6::float8/1000))
            ORDER BY m.match_id LIMIT 25"
    );
    let mut last_id: Option<String> = None;
    loop {
        // Pagination par clé unique, dans le même instantané : mémoire des détails bornée.
        let rows = sqlx::query(&page)
            .bind(&last_id)
            .bind(&filters.patches)
            .bind(&filters.platforms)
            .bind(&filters.queues)
            .bind(filters.start_ms)
            .bind(filters.end_ms)
            .fetch_all(&mut *tx)
            .await?;
        if rows.is_empty() {
            break;
        }
        for row in &rows {
            let game = stored_match(row)?;
            accumulator.add(&game);
            last_id = Some(game.match_id);
        }
    }
    let report = accumulator.finish();
    super::snapshot::publish(&mut tx, &report).await?;
    tx.commit().await?;
    Ok(report)
}

/// Catalogue normalisé (#61) courant de chaque patch, lu dans l'instantané du calcul.
/// Plusieurs révisions d'un même patch : la plus récente (16.19.2 avant 16.19.1).
pub(super) async fn load_item_catalogs(
    connection: &mut PgConnection,
    patches: &[String],
) -> Result<BTreeMap<String, ItemCatalog>, AggregationError> {
    let versions: Vec<String> = sqlx::query_scalar("SELECT version FROM game_catalog_current")
        .fetch_all(&mut *connection)
        .await?;
    let mut selected = BTreeMap::<String, ((u32, u32, u32), String)>::new();
    for version in versions {
        let Some(parts) = version_parts(&version) else {
            continue;
        };
        let patch = format!("{}.{}", parts.0, parts.1);
        if !patches.is_empty() && !patches.contains(&patch) {
            continue;
        }
        if selected.get(&patch).map_or(true, |(best, _)| parts > *best) {
            selected.insert(patch, (parts, version));
        }
    }
    let mut catalogs = BTreeMap::new();
    for (patch, (_, version)) in selected {
        // Les champs structurels sont identiques entre langues : une seule fiche par objet.
        let rows = sqlx::query(
            "SELECT DISTINCT ON (e.id) e.id, e.data->'fields' AS fields
            FROM game_catalog_current c JOIN game_catalog_entries e ON e.publication_id=c.publication_id
            WHERE c.version=$1 AND e.kind='item' AND e.namespace='standard'
            ORDER BY e.id, (e.locale='en_US') DESC, e.locale",
        )
        .bind(&version)
        .fetch_all(&mut *connection)
        .await?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.try_get("id")?;
            let fields: Option<serde_json::Value> = row.try_get("fields")?;
            records.push((id, fields.unwrap_or_default()));
        }
        let catalog =
            ItemCatalog::from_records(&version, records.iter().map(|(id, f)| (id.as_str(), f)));
        catalogs.insert(patch, catalog);
    }
    Ok(catalogs)
}

fn version_parts(version: &str) -> Option<(u32, u32, u32)> {
    if !crate::catalog::valid_version(version) {
        return None;
    }
    let mut parts = version.split('.').map(|p| p.parse::<u32>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}
