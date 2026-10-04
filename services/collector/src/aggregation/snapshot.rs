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
    ban_rank_basis: &'a str,
    ban_rank_min_known_players: u32,
    pick_rate_definition: &'a str,
    tier_method: &'a str,
    min_games: u32,
    reliability_floor: u32,
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
    write_head(tx, report).await?;
    // Un recalcul complet remplace aussi les lots (#89) : aucune empreinte ne survit à
    // ses morceaux, que la suppression des lots emporte en cascade.
    sqlx::query("DELETE FROM champion_stats_snapshot_lots WHERE snapshot_id=1")
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM champion_stats_snapshot_chunks WHERE snapshot_id=1")
        .execute(&mut **tx)
        .await?;
    write_sections(tx, &mut SectionWriter::default(), report).await
}

/// Vue empruntée : ne pas dupliquer toutes les listes dans un arbre serde_json::Value.
fn metadata(report: &AggregationReport) -> Metadata<'_> {
    Metadata {
        schema_version: report.schema_version,
        rank_scope: &report.rank_scope,
        rank_max_age_hours: report.rank_max_age_hours,
        min_game_duration_s: report.min_game_duration_s,
        min_played_percent: report.min_played_percent,
        exclude_afk: report.exclude_afk,
        ban_rank_basis: &report.ban_rank_basis,
        ban_rank_min_known_players: report.ban_rank_min_known_players,
        pick_rate_definition: &report.pick_rate_definition,
        tier_method: &report.tier_method,
        min_games: report.min_games,
        reliability_floor: report.reliability_floor,
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
    }
}

/// Paramètres de l'en-tête, sans compteurs ni filtres : base de l'empreinte d'un lot (#89).
/// Tout nouveau champ d'en-tête invalide donc les lots publiés avant lui.
pub(super) fn settings_text(report: &AggregationReport) -> Result<String, AggregationError> {
    let mut head = serde_json::to_value(metadata(report))?;
    if let Some(fields) = head.as_object_mut() {
        // Les filtres et le catalogue d'un lot entrent déjà dans l'empreinte de ses données.
        for field in [
            "source_matches",
            "included_matches",
            "exclusions",
            "omitted_build_variants",
            "filters",
            "item_catalogs",
        ] {
            fields.remove(field);
        }
    }
    Ok(head.to_string())
}

/// En-tête seul (métadonnées et compteurs). Premier verrou d'écriture : un instantané
/// devenu ancien échoue avant de toucher aux morceaux. Tête, suppression et nouveaux
/// morceaux sont validés ensemble.
pub(super) async fn write_head(
    tx: &mut Transaction<'_, Postgres>,
    report: &AggregationReport,
) -> Result<(), AggregationError> {
    sqlx::query(
        "INSERT INTO champion_stats_snapshot (id,source_snapshot_at,published_at,report,storage_version)
        VALUES (1,transaction_timestamp(),clock_timestamp(),$1,2)
        ON CONFLICT (id) DO UPDATE SET source_snapshot_at=EXCLUDED.source_snapshot_at,
            published_at=EXCLUDED.published_at,report=EXCLUDED.report,storage_version=2",
    )
    .bind(sqlx::types::Json(metadata(report)))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Rang du prochain morceau par section et lot d'origine des morceaux écrits (#89).
#[derive(Default)]
pub(super) struct SectionWriter {
    pub lot: Option<super::ScopeKey>,
    pub next_index: BTreeMap<String, i32>,
    pub written: i32,
}

/// Seul point d'écriture des sections, commun au recalcul complet et aux lots : une
/// section ajoutée ici est publiée par les deux modes.
pub(super) async fn write_sections(
    tx: &mut Transaction<'_, Postgres>,
    writer: &mut SectionWriter,
    report: &AggregationReport,
) -> Result<(), AggregationError> {
    write_section(tx, writer, "coverage", &report.coverage).await?;
    write_section(tx, writer, "groups", &report.groups).await?;
    write_section(tx, writer, "bans", &report.bans).await?;
    write_section(tx, writer, "builds", &report.builds).await?;
    write_section(tx, writer, "skill_levels", &report.skill_levels).await?;
    write_section(tx, writer, "item_events", &report.item_events).await?;
    write_section(tx, writer, "splits", &report.splits).await?;
    write_section(tx, writer, "performance", &report.performance).await?;
    write_section(tx, writer, "matchups", &report.matchups).await?;
    Ok(())
}

async fn write_section<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    writer: &mut SectionWriter,
    section: &str,
    entries: &[T],
) -> Result<(), AggregationError> {
    let mut offset = 0;
    while let Some(chunk) = next_chunk(entries, &mut offset)? {
        let index = writer.next_index.entry(section.to_owned()).or_default();
        let lot = writer.lot.as_ref();
        sqlx::query(
            "INSERT INTO champion_stats_snapshot_chunks
            (snapshot_id,section,chunk_index,items,lot_patch,lot_platform_id,lot_queue_id)
            VALUES (1,$1,$2,$3::jsonb,$4,$5,$6)",
        )
        .bind(section)
        .bind(*index)
        .bind(chunk)
        .bind(lot.map(|l| &l.patch))
        .bind(lot.map(|l| &l.platform_id))
        .bind(lot.map(|l| l.queue_id))
        .execute(&mut **tx)
        .await?;
        *index = index
            .checked_add(1)
            .ok_or(AggregationError::SnapshotTooLarge)?;
        writer.written += 1;
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
