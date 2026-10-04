//! Publication atomique en morceaux : aucun conteneur JSONB ne porte le rapport entier.

use std::collections::BTreeMap;

use serde::Serialize;
use sqlx::{Postgres, Transaction};

use super::{AggregationError, AggregationOptions, AggregationReport};

const MAX_CHUNK_BYTES: usize = 1024 * 1024;
const MAX_CHUNK_ENTRIES: usize = 512;

#[derive(Serialize)]
struct Metadata<'a> {
    schema_version: u32,
    rank_scope: &'a str,
    rank_max_age_hours: u32,
    min_game_duration_s: u32,
    min_played_percent: u32,
    exclude_afk: bool,
    pick_rate_definition: &'a str,
    tier_method: &'a str,
    min_games: u32,
    filters: &'a AggregationOptions,
    source_matches: u64,
    included_matches: u64,
    exclusions: &'a BTreeMap<String, u64>,
    max_build_variants_per_category: u32,
    omitted_build_variants: u64,
    build_stage_method: &'a str,
    item_catalogs: &'a [super::ItemCatalogRef],
    performance_method: &'a str,
    matchup_method: &'a str,
}

/// La transaction du calcul possède déjà le verrou et l'instantané REPEATABLE READ.
pub(super) async fn publish(
    tx: &mut Transaction<'_, Postgres>,
    report: &AggregationReport,
) -> Result<(), AggregationError> {
    // Vue empruntée : ne pas dupliquer toutes les listes dans un arbre serde_json::Value.
    let metadata = Metadata {
        schema_version: report.schema_version,
        rank_scope: &report.rank_scope,
        rank_max_age_hours: report.rank_max_age_hours,
        min_game_duration_s: report.min_game_duration_s,
        min_played_percent: report.min_played_percent,
        exclude_afk: report.exclude_afk,
        pick_rate_definition: &report.pick_rate_definition,
        tier_method: &report.tier_method,
        min_games: report.min_games,
        filters: &report.filters,
        source_matches: report.source_matches,
        included_matches: report.included_matches,
        exclusions: &report.exclusions,
        max_build_variants_per_category: report.max_build_variants_per_category,
        omitted_build_variants: report.omitted_build_variants,
        build_stage_method: &report.build_stage_method,
        item_catalogs: &report.item_catalogs,
        performance_method: &report.performance_method,
        matchup_method: &report.matchup_method,
    };
    // Premier verrou d'écriture : un instantané devenu ancien échoue avant de toucher
    // aux morceaux. Tête, suppression et nouveaux morceaux sont validés ensemble.
    sqlx::query(
        "INSERT INTO champion_stats_snapshot (id,source_snapshot_at,published_at,report,storage_version)
        VALUES (1,transaction_timestamp(),clock_timestamp(),$1,2)
        ON CONFLICT (id) DO UPDATE SET source_snapshot_at=EXCLUDED.source_snapshot_at,
            published_at=EXCLUDED.published_at,report=EXCLUDED.report,storage_version=2",
    )
    .bind(sqlx::types::Json(metadata))
    .execute(&mut **tx)
    .await?;
    sqlx::query("DELETE FROM champion_stats_snapshot_chunks WHERE snapshot_id=1")
        .execute(&mut **tx)
        .await?;
    write_section(tx, "coverage", &report.coverage).await?;
    write_section(tx, "groups", &report.groups).await?;
    write_section(tx, "bans", &report.bans).await?;
    write_section(tx, "builds", &report.builds).await?;
    write_section(tx, "skill_levels", &report.skill_levels).await?;
    write_section(tx, "item_events", &report.item_events).await?;
    write_section(tx, "performance", &report.performance).await?;
    write_section(tx, "matchups", &report.matchups).await?;
    Ok(())
}

async fn write_section<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    section: &str,
    entries: &[T],
) -> Result<(), AggregationError> {
    let mut offset = 0;
    let mut index = 0_i32;
    while let Some(chunk) = next_chunk(entries, &mut offset)? {
        sqlx::query(
            "INSERT INTO champion_stats_snapshot_chunks (snapshot_id,section,chunk_index,items)
            VALUES (1,$1,$2,$3::jsonb)",
        )
        .bind(section)
        .bind(index)
        .bind(chunk)
        .execute(&mut **tx)
        .await?;
        index = index
            .checked_add(1)
            .ok_or(AggregationError::SnapshotTooLarge)?;
    }
    Ok(())
}

fn next_chunk<T: Serialize>(
    entries: &[T],
    offset: &mut usize,
) -> Result<Option<String>, AggregationError> {
    if *offset == entries.len() {
        return Ok(None);
    }
    let mut chunk = String::from("[");
    let mut count = 0;
    while *offset < entries.len() && count < MAX_CHUNK_ENTRIES {
        let item = serde_json::to_string(&entries[*offset])?;
        if item.len() + 2 > MAX_CHUNK_BYTES {
            return Err(AggregationError::SnapshotTooLarge);
        }
        let separator = usize::from(count > 0);
        if chunk.len() + separator + item.len() + 1 > MAX_CHUNK_BYTES {
            break;
        }
        if count > 0 {
            chunk.push(',');
        }
        chunk.push_str(&item);
        *offset += 1;
        count += 1;
    }
    chunk.push(']');
    Ok(Some(chunk))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_morceaux_conservent_l_ordre_et_bornent_les_entrees() {
        let entries: Vec<_> = (0..MAX_CHUNK_ENTRIES * 2 + 1).collect();
        let mut offset = 0;
        let mut combined = Vec::<usize>::new();
        let mut chunks = 0;
        while let Some(chunk) = next_chunk(&entries, &mut offset).unwrap() {
            let values: Vec<usize> = serde_json::from_str(&chunk).unwrap();
            assert!(values.len() <= MAX_CHUNK_ENTRIES);
            assert!(chunk.len() <= MAX_CHUNK_BYTES);
            combined.extend(values);
            chunks += 1;
        }
        assert_eq!(chunks, 3);
        assert_eq!(combined, entries);
    }

    #[test]
    fn la_borne_en_octets_prend_en_compte_les_caracteres_echappes() {
        let entries = vec!["\"".repeat(MAX_CHUNK_BYTES / 4); 3];
        let mut offset = 0;
        let mut chunks = 0;
        while let Some(chunk) = next_chunk(&entries, &mut offset).unwrap() {
            assert!(chunk.len() <= MAX_CHUNK_BYTES);
            chunks += 1;
        }
        assert_eq!(chunks, 3);
        assert!(matches!(
            next_chunk(&["x".repeat(MAX_CHUNK_BYTES)], &mut 0),
            Err(AggregationError::SnapshotTooLarge)
        ));
    }
}
