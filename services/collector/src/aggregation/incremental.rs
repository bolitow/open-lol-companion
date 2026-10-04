//! Recalcul par lots (#89). Un lot est un périmètre patch/plateforme/file : chaque
//! section du rapport est indexée par ce périmètre et triée d'abord par lui, et `finish`
//! ne mélange jamais deux périmètres. Concaténer les rapports des lots, dans l'ordre des
//! périmètres, redonne donc exactement le recalcul complet.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Row};

use super::model::ScopeKey;
use super::snapshot::{self, SectionWriter};
use super::stages::ItemCatalog;
use super::storage::{self, Settings, AGGREGATION_LOCK_KEY};
use super::{AggregationError, AggregationOptions, AggregationReport, QualityThresholds};
use crate::storage::Storage;

/// Bilan d'un recalcul par lots : en-tête publié et lots recalculés ou réutilisés.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IncrementalReport {
    pub lots: u64,
    pub recomputed_lots: u64,
    pub reused_lots: u64,
    /// En-tête publié, identique à celui d'un recalcul complet. Les sections restent
    /// vides : elles ne sont jamais réunies en mémoire et se lisent dans l'instantané.
    pub report: AggregationReport,
}

/// Recalcule et publie atomiquement, lot par lot (#89) : seul un lot dont l'empreinte a
/// changé depuis la dernière publication est relu ; la mémoire est bornée par le plus
/// gros lot. Le résultat publié est celui du recalcul complet, aux mêmes paramètres.
pub async fn recalculate_incremental(
    storage: &Storage,
    min_games: u32,
    rank_max_age_hours: u32,
    filters: &AggregationOptions,
    quality: &QualityThresholds,
) -> Result<IncrementalReport, AggregationError> {
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
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(AGGREGATION_LOCK_KEY)
        .fetch_one(&mut *tx)
        .await?;
    if !acquired {
        return Err(AggregationError::Busy);
    }
    let catalogs = storage::load_item_catalogs(&mut tx, &filters.patches).await?;
    let mut head = settings.accumulator(catalogs.clone())?.finish();
    // Premier verrou d'écriture, comme le recalcul complet : un instantané devenu ancien
    // échoue avant tout calcul. L'en-tête définitif est réécrit à la fin.
    snapshot::write_head(&mut tx, &head).await?;
    let base = format!("{}\n{}", binary_identity(), snapshot::settings_text(&head)?);
    let current = current_lots(&mut tx, &settings, &catalogs, &base).await?;
    let mut reused = BTreeMap::<ScopeKey, LotCounts>::new();
    let mut stale = Vec::<ScopeKey>::new();
    for (scope, previous) in previous_lots(&mut tx).await? {
        match previous {
            Some((fingerprint, counts)) if current.get(&scope) == Some(&fingerprint) => {
                reused.insert(scope, counts);
            }
            _ => stale.push(scope),
        }
    }
    delete_stale(&mut tx, &stale).await?;
    let mut next_index = next_chunk_indexes(&mut tx).await?;
    let mut recomputed_lots = 0;
    for (scope, fingerprint) in &current {
        if let Some(counts) = reused.get(scope) {
            counts.add_to(&mut head);
            continue;
        }
        let mut accumulator = settings.accumulator(lot_catalogs(&catalogs, scope))?;
        let mut after = String::new();
        loop {
            let page = storage::lot_page(&mut tx, scope, &settings.filters, &after).await?;
            let Some(last) = page.last() else { break };
            after.clone_from(&last.match_id);
            for game in &page {
                accumulator.add(game);
            }
        }
        let lot = accumulator.finish();
        let counts = LotCounts::of(&lot);
        sqlx::query(
            "INSERT INTO champion_stats_snapshot_lots
            (snapshot_id,patch,platform_id,queue_id,fingerprint,counts,chunks)
            VALUES (1,$1,$2,$3,$4,$5,0)",
        )
        .bind(&scope.patch)
        .bind(&scope.platform_id)
        .bind(scope.queue_id)
        .bind(fingerprint)
        .bind(sqlx::types::Json(&counts))
        .execute(&mut *tx)
        .await?;
        let mut writer = SectionWriter {
            lot: Some(scope.clone()),
            next_index: std::mem::take(&mut next_index),
            written: 0,
        };
        snapshot::write_sections(&mut tx, &mut writer, &lot).await?;
        next_index = writer.next_index;
        sqlx::query(
            "UPDATE champion_stats_snapshot_lots SET chunks=$4
            WHERE snapshot_id=1 AND patch=$1 AND platform_id=$2 AND queue_id=$3",
        )
        .bind(&scope.patch)
        .bind(&scope.platform_id)
        .bind(scope.queue_id)
        .bind(writer.written)
        .execute(&mut *tx)
        .await?;
        counts.add_to(&mut head);
        recomputed_lots += 1;
    }
    snapshot::write_head(&mut tx, &head).await?;
    tx.commit().await?;
    Ok(IncrementalReport {
        lots: current.len() as u64,
        recomputed_lots,
        reused_lots: current.len() as u64 - recomputed_lots,
        report: head,
    })
}

/// Identité du binaire (chemin, taille, date de modification) : un nouveau binaire
/// recalcule tous les lots une fois, sans dépendre d'un numéro de version à incrémenter
/// à la main quand une section ou une règle change. Repli : identité propre au processus.
fn binary_identity() -> &'static str {
    static IDENTITY: OnceLock<String> = OnceLock::new();
    IDENTITY.get_or_init(|| {
        std::env::current_exe()
            .and_then(|path| {
                let meta = std::fs::metadata(&path)?;
                let modified = meta
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                Ok(format!(
                    "{}:{}:{}",
                    path.display(),
                    meta.len(),
                    modified.as_nanos()
                ))
            })
            .unwrap_or_else(|error| {
                // Sans identité stable, chaque processus recalcule tous les lots une fois.
                tracing::warn!(%error, "identité du binaire illisible : lots recalculés à chaque démarrage");
                format!("process:{}", std::process::id())
            })
    })
}

/// Empreinte de chaque lot présent, calculée dans l'instantané du calcul. Elle couvre
/// tout ce que lit un lot : parties et timelines (identifiants, `xmin` donc toute
/// modification, état), observations de rang de sa file assez proches de ses parties,
/// contenu du catalogue d'objets de son patch (version et classement des objets, pour
/// qu'une republication de la même version qui change les étapes relise le patch),
/// paramètres et binaire. Une observation plus loin
/// que l'écart maximal (+1 h pour l'arrondi) de toutes les parties du lot ne peut ni
/// devenir leur rang ni changer leur écart retenu : elle n'invalide pas le lot.
async fn current_lots(
    connection: &mut PgConnection,
    settings: &Settings,
    catalogs: &BTreeMap<String, ItemCatalog>,
    base: &str,
) -> Result<BTreeMap<ScopeKey, String>, AggregationError> {
    let filters = &settings.filters;
    let rows = sqlx::query(
        "WITH lots AS (
            SELECT m.patch,m.platform_id,m.queue_id,count(*) AS matches,
                min(m.game_start) AS first_start,max(m.game_start) AS last_start,
                md5(string_agg(m.match_id||':'||m.xmin::text||':'||COALESCE(t.status||':'||t.xmin::text,'-'),
                    ',' ORDER BY m.match_id)) AS matches_digest
            FROM matches m LEFT JOIN match_timelines t ON t.match_id=m.match_id
            WHERE (cardinality($1::text[])=0 OR m.patch=ANY($1))
                AND (cardinality($2::text[])=0 OR m.platform_id=ANY($2))
                AND (cardinality($3::int[])=0 OR m.queue_id=ANY($3))
                AND ($4::bigint IS NULL OR m.game_start>=to_timestamp($4::float8/1000))
                AND ($5::bigint IS NULL OR m.game_start<to_timestamp($5::float8/1000))
            GROUP BY m.patch,m.platform_id,m.queue_id)
        SELECT l.patch,l.platform_id,l.queue_id,l.matches,l.matches_digest,
            (SELECT count(*)::text||':'||COALESCE(md5(string_agg(o.id::text||':'||o.xmin::text,',' ORDER BY o.id)),'-')
                FROM participant_rank_observations o
                WHERE o.platform_id=l.platform_id AND o.queue_id=l.queue_id
                    AND o.observed_at BETWEEN l.first_start-$6*interval '1 hour'
                        AND l.last_start+$6*interval '1 hour') AS ranks_digest
        FROM lots l",
    )
    .bind(&filters.patches)
    .bind(&filters.platforms)
    .bind(&filters.queues)
    .bind(filters.start_ms)
    .bind(filters.end_ms)
    .bind(f64::from(settings.rank_max_age_hours) + 1.0)
    .fetch_all(&mut *connection)
    .await?;
    // Une seule empreinte par patch, quel que soit le nombre de ses lots.
    let catalogs: BTreeMap<&str, String> = catalogs
        .iter()
        .map(|(patch, catalog)| (patch.as_str(), catalog.fingerprint()))
        .collect();
    let mut lots = BTreeMap::new();
    for row in rows {
        let scope = ScopeKey {
            patch: row.try_get("patch")?,
            platform_id: row.try_get("platform_id")?,
            queue_id: row.try_get("queue_id")?,
        };
        let catalog = catalogs
            .get(scope.patch.as_str())
            .map_or("-", String::as_str);
        let matches: i64 = row.try_get("matches")?;
        let matches_digest: String = row.try_get("matches_digest")?;
        let ranks_digest: String = row.try_get("ranks_digest")?;
        let fingerprint = Sha256::digest(
            format!("{base}\n{catalog}\n{matches}:{matches_digest}\n{ranks_digest}").as_bytes(),
        );
        lots.insert(scope, format!("{fingerprint:x}"));
    }
    Ok(lots)
}

/// Lots publiés ; `None` si le lot ne peut pas être réutilisé tel quel (compteurs
/// illisibles ou morceaux manquants).
async fn previous_lots(
    connection: &mut PgConnection,
) -> Result<Vec<(ScopeKey, Option<(String, LotCounts)>)>, AggregationError> {
    let rows = sqlx::query(
        "SELECT l.patch,l.platform_id,l.queue_id,l.fingerprint,l.counts,l.chunks,
            (SELECT count(*) FROM champion_stats_snapshot_chunks c WHERE c.snapshot_id=l.snapshot_id
                AND c.lot_patch=l.patch AND c.lot_platform_id=l.platform_id
                AND c.lot_queue_id=l.queue_id) AS written
        FROM champion_stats_snapshot_lots l WHERE l.snapshot_id=1",
    )
    .fetch_all(&mut *connection)
    .await?;
    let mut lots = Vec::with_capacity(rows.len());
    for row in rows {
        let scope = ScopeKey {
            patch: row.try_get("patch")?,
            platform_id: row.try_get("platform_id")?,
            queue_id: row.try_get("queue_id")?,
        };
        let chunks: i32 = row.try_get("chunks")?;
        let written: i64 = row.try_get("written")?;
        let counts = serde_json::from_value::<LotCounts>(row.try_get("counts")?).ok();
        let fingerprint: String = row.try_get("fingerprint")?;
        let reusable = counts
            .filter(|_| i64::from(chunks) == written)
            .map(|counts| (fingerprint, counts));
        lots.push((scope, reusable));
    }
    Ok(lots)
}

/// Supprime les lots à recalculer ou disparus (leurs morceaux suivent en cascade) et les
/// morceaux d'un recalcul complet, qui n'appartiennent à aucun lot.
async fn delete_stale(
    connection: &mut PgConnection,
    stale: &[ScopeKey],
) -> Result<(), AggregationError> {
    let patches: Vec<&str> = stale.iter().map(|s| s.patch.as_str()).collect();
    let platforms: Vec<&str> = stale.iter().map(|s| s.platform_id.as_str()).collect();
    let queues: Vec<i32> = stale.iter().map(|s| s.queue_id).collect();
    sqlx::query(
        "DELETE FROM champion_stats_snapshot_lots WHERE snapshot_id=1
            AND (patch,platform_id,queue_id) IN (SELECT * FROM unnest($1::text[],$2::text[],$3::int[]))",
    )
    .bind(&patches)
    .bind(&platforms)
    .bind(&queues)
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "DELETE FROM champion_stats_snapshot_chunks WHERE snapshot_id=1 AND lot_patch IS NULL",
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Les morceaux des lots réutilisés gardent leur rang : les nouveaux sont écrits après.
async fn next_chunk_indexes(
    connection: &mut PgConnection,
) -> Result<BTreeMap<String, i32>, AggregationError> {
    let rows = sqlx::query(
        "SELECT section,max(chunk_index)+1 AS next FROM champion_stats_snapshot_chunks
        WHERE snapshot_id=1 GROUP BY section",
    )
    .fetch_all(&mut *connection)
    .await?;
    rows.iter()
        .map(|row| Ok((row.try_get("section")?, row.try_get("next")?)))
        .collect()
}

/// Compteurs additifs d'un lot, conservés en base pour réutiliser le lot sans le relire.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct LotCounts {
    pub source_matches: u64,
    pub included_matches: u64,
    pub exclusions: BTreeMap<String, u64>,
    pub omitted_build_variants: u64,
}

impl LotCounts {
    /// Déstructuration exhaustive, sans `..` : un champ ajouté au rapport (section ou
    /// compteur, par une autre évolution) ne compile plus tant qu'il n'est pas classé ici
    /// et dans `append`. Les paramètres sont identiques pour tous les lots d'un calcul.
    pub(super) fn of(report: &AggregationReport) -> Self {
        let AggregationReport {
            source_matches,
            included_matches,
            exclusions,
            omitted_build_variants,
            schema_version: _,
            rank_scope: _,
            rank_max_age_hours: _,
            min_game_duration_s: _,
            min_played_percent: _,
            exclude_afk: _,
            ban_rank_basis: _,
            ban_rank_min_known_players: _,
            pick_rate_definition: _,
            tier_method: _,
            min_games: _,
            filters: _,
            max_build_variants_per_category: _,
            build_stage_method: _,
            item_catalogs: _,
            coverage: _,
            groups: _,
            bans: _,
            builds: _,
            skill_levels: _,
            item_events: _,
        } = report;
        Self {
            source_matches: *source_matches,
            included_matches: *included_matches,
            exclusions: exclusions.clone(),
            omitted_build_variants: *omitted_build_variants,
        }
    }

    pub(super) fn add_to(&self, report: &mut AggregationReport) {
        report.source_matches += self.source_matches;
        report.included_matches += self.included_matches;
        for (reason, count) in &self.exclusions {
            *report.exclusions.entry(reason.clone()).or_default() += count;
        }
        report.omitted_build_variants += self.omitted_build_variants;
    }
}

impl AggregationReport {
    /// Ajoute le rapport d'un lot, lots pris dans l'ordre croissant des périmètres. Les
    /// lots écrivent leurs sections l'un après l'autre : c'est cette concaténation que
    /// la publication réalise, sans jamais réunir les sections en mémoire.
    #[cfg(test)]
    pub(super) fn append(&mut self, lot: AggregationReport) {
        LotCounts::of(&lot).add_to(self);
        // Même règle d'exhaustivité que `LotCounts::of` : chaque section est concaténée.
        let AggregationReport {
            coverage,
            groups,
            bans,
            builds,
            skill_levels,
            item_events,
            source_matches: _,
            included_matches: _,
            exclusions: _,
            omitted_build_variants: _,
            schema_version: _,
            rank_scope: _,
            rank_max_age_hours: _,
            min_game_duration_s: _,
            min_played_percent: _,
            exclude_afk: _,
            ban_rank_basis: _,
            ban_rank_min_known_players: _,
            pick_rate_definition: _,
            tier_method: _,
            min_games: _,
            filters: _,
            max_build_variants_per_category: _,
            build_stage_method: _,
            item_catalogs: _,
        } = lot;
        self.coverage.extend(coverage);
        self.groups.extend(groups);
        self.bans.extend(bans);
        self.builds.extend(builds);
        self.skill_levels.extend(skill_levels);
        self.item_events.extend(item_events);
    }
}

/// Catalogue du seul patch du lot : les autres patches n'y interviennent jamais.
pub(super) fn lot_catalogs(
    catalogs: &BTreeMap<String, ItemCatalog>,
    scope: &ScopeKey,
) -> BTreeMap<String, ItemCatalog> {
    catalogs
        .get_key_value(&scope.patch)
        .map(|(patch, catalog)| (patch.clone(), catalog.clone()))
        .into_iter()
        .collect()
}

#[cfg(test)]
#[path = "incremental_tests.rs"]
mod tests;
