use sqlx::Row;

use super::model::{Accumulator, StoredMatch};
use super::{AggregationError, AggregationReport};
use crate::storage::Storage;

/// Recalcule les données stockées sans filtre (usage local et tests).
pub async fn recalculate(
    storage: &Storage,
    min_games: u32,
) -> Result<AggregationReport, AggregationError> {
    recalculate_filtered(storage, min_games, &super::AggregationOptions::default()).await
}

/// Recalcule et publie atomiquement une sélection explicite, sans appel réseau.
pub async fn recalculate_filtered(
    storage: &Storage,
    min_games: u32,
    filters: &super::AggregationOptions,
) -> Result<AggregationReport, AggregationError> {
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
    let mut accumulator = Accumulator::new(min_games)?;
    accumulator.set_filters(filters.clone());
    let mut connection = storage.transaction_connection().await?;
    let mut tx = connection.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    // Clé distincte de celle du collecteur : l'ingestion peut continuer.
    // Le verrou suit cette transaction/connexion ; aucune session séparée à maintenir.
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(0x0018_A660_0001_i64)
        .fetch_one(&mut *tx)
        .await?;
    if !acquired {
        return Err(AggregationError::Busy);
    }

    let mut last_id: Option<String> = None;
    loop {
        // Pagination par clé unique, dans le même instantané : mémoire des détails bornée.
        let rows = sqlx::query(
            "SELECT m.match_id,m.platform_id,m.queue_id,m.patch,m.is_remake,m.detail,t.timeline,
            COALESCE((SELECT jsonb_object_agg(p->>'puuid',jsonb_build_object('status',r.status,'tier',r.tier))
                FROM jsonb_array_elements(CASE WHEN jsonb_typeof(m.detail#>'{info,participants}')='array'
                    THEN m.detail#>'{info,participants}' ELSE '[]'::jsonb END) p
                JOIN LATERAL (SELECT status,tier FROM participant_rank_observations o
                    WHERE o.platform_id=m.platform_id AND o.queue_id=m.queue_id AND o.puuid=p->>'puuid'
                    AND o.observed_at <= transaction_timestamp()
                    AND o.observed_at >= transaction_timestamp()-interval '24 hours'
                    ORDER BY o.observed_at DESC,o.id DESC LIMIT 1) r ON true), '{}'::jsonb) AS ranks
            FROM matches m LEFT JOIN match_timelines t ON t.match_id=m.match_id AND t.status='available'
            WHERE ($1::text IS NULL OR m.match_id > $1)
                AND (cardinality($2::text[])=0 OR m.patch=ANY($2))
                AND (cardinality($3::text[])=0 OR m.platform_id=ANY($3))
                AND (cardinality($4::int[])=0 OR m.queue_id=ANY($4))
                AND ($5::bigint IS NULL OR m.game_start>=to_timestamp($5::float8/1000))
                AND ($6::bigint IS NULL OR m.game_start<to_timestamp($6::float8/1000))
            ORDER BY m.match_id LIMIT 25",
        )
        .bind(&last_id)
        .bind(&filters.patches).bind(&filters.platforms).bind(&filters.queues)
        .bind(filters.start_ms).bind(filters.end_ms)
        .fetch_all(&mut *tx)
        .await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let game = StoredMatch {
                match_id: row.try_get("match_id")?,
                platform_id: row.try_get("platform_id")?,
                queue_id: row.try_get("queue_id")?,
                patch: row.try_get("patch")?,
                is_remake: row.try_get("is_remake")?,
                detail: row.try_get("detail")?,
                timeline: row.try_get("timeline")?,
                ranks: serde_json::from_value(row.try_get("ranks")?)?,
            };
            accumulator.add(&game);
            last_id = Some(game.match_id);
        }
    }
    let report = accumulator.finish();
    super::snapshot::publish(&mut tx, &report).await?;
    tx.commit().await?;
    Ok(report)
}
