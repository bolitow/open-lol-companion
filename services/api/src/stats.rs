//! Lecture des instantanés publiés par #18, sans recalcul ni mélange de populations.
use crate::{error::ApiError, query::StatsQuery};
use olc_collector::aggregation::*;
use serde::Serialize;
use sqlx::{PgPool, Row};

#[derive(Serialize)]
pub struct SnapshotMeta {
    pub source_snapshot_at: String,
    pub published_at: String,
    pub schema_version: u32,
    pub min_games: u32,
    pub rank_scope: String,
    pub rank_max_age_hours: u32,
    pub pick_rate_definition: String,
    pub tier_method: String,
    pub filters: AggregationOptions,
    pub freshness: Freshness,
    pub coverage: Vec<ScopeCoverage>,
}
/// Fraîcheur réelle des périmètres lus (#103), distincte de l'heure du calcul : les dates
/// de parties viennent de la couverture publiée ; `null` pour un instantané antérieur.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct Freshness {
    /// Date du calcul (même instant que `source_snapshot_at`).
    pub computed_at: String,
    /// Début (ms Unix) de la plus ancienne partie incluse des périmètres lus.
    pub first_game_start_ms: Option<i64>,
    /// Début (ms Unix) de la plus récente partie incluse des périmètres lus.
    pub last_game_start_ms: Option<i64>,
}
#[derive(Serialize)]
pub struct TierlistResponse {
    pub meta: SnapshotMeta,
    pub query: StatsQuery,
    pub total: usize,
    pub entries: Vec<ChampionStats>,
    pub bans: Vec<BanStats>,
}
#[derive(Serialize)]
pub struct BuildsResponse {
    pub meta: SnapshotMeta,
    pub query: StatsQuery,
    pub champion_id: u32,
    pub summary: Option<ChampionStats>,
    pub total: usize,
    pub builds: Vec<BuildStats>,
    pub skill_levels: Vec<SkillStats>,
    pub item_events: Vec<ItemEventStats>,
    /// Winrate du champion par tranche de durée puis par côté (#119) ; le côté n'est publié
    /// que pour le rang `ALL`. Vide pour un instantané antérieur ou pour Arena.
    pub splits: Vec<SplitStats>,
    pub max_build_variants_per_category: u32,
    pub omitted_build_variants: u64,
    /// Règles des étapes d'achat (#81) ; vide pour un instantané antérieur.
    pub build_stage_method: String,
    /// Version du catalogue d'objets jointe au patch demandé ; nulle sans étapes.
    pub item_catalog_version: Option<String>,
}
/// Tierlist filtrée sur une population explicite, triée selon le rang publié.
pub async fn tierlist(pool: &PgPool, query: StatsQuery) -> Result<TierlistResponse, ApiError> {
    query.validate().map_err(|_| ApiError::InvalidRequest)?;
    let (meta, report) = load(pool, &query, None).await?;
    let mut entries: Vec<_> = report
        .groups
        .into_iter()
        .filter(|g| matches(&g.key, &query))
        .collect();
    entries.sort_by_key(|g| (g.position.unwrap_or(u32::MAX), g.key.champion_id));
    let total = entries.len();
    let entries: Vec<_> = entries
        .into_iter()
        .skip(query.offset)
        .take(query.limit)
        .collect();
    let bans = report
        .bans
        .into_iter()
        .filter(|b| {
            scope_matches(&b.scope, &query)
                && entries.iter().any(|g| g.key.champion_id == b.champion_id)
        })
        .collect();
    Ok(TierlistResponse {
        meta,
        query,
        total,
        entries,
        bans,
    })
}
/// Variantes de build, compétences et achats du champion dans la même population.
pub async fn builds(
    pool: &PgPool,
    query: StatsQuery,
    champion_id: u32,
) -> Result<BuildsResponse, ApiError> {
    query.validate().map_err(|_| ApiError::InvalidRequest)?;
    if champion_id == 0 {
        return Err(ApiError::InvalidRequest);
    }
    let (meta, report) = load(pool, &query, Some(champion_id)).await?;
    let selected = |key: &GroupKey| key.champion_id == champion_id && matches(key, &query);
    let summary = report.groups.into_iter().find(|g| selected(&g.key));
    let mut variants: Vec<_> = report
        .builds
        .into_iter()
        .filter(|b| selected(&b.key))
        .collect();
    variants.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then(b.games.cmp(&a.games))
            .then(a.selection.cmp(&b.selection))
    });
    let total = variants.len();
    let builds = variants
        .into_iter()
        .skip(query.offset)
        .take(query.limit)
        .collect();
    let skill_levels = report
        .skill_levels
        .into_iter()
        .filter(|b| selected(&b.key))
        .collect();
    let item_events = report
        .item_events
        .into_iter()
        .filter(|b| selected(&b.key))
        .collect();
    let mut splits: Vec<_> = report
        .splits
        .into_iter()
        .filter(|s| selected(&s.key))
        .collect();
    splits.sort_by_key(|s| (s.dimension, s.bucket));
    let item_catalog_version = report
        .item_catalogs
        .into_iter()
        .find(|c| c.patch == query.patch)
        .map(|c| c.version);
    Ok(BuildsResponse {
        meta,
        query,
        champion_id,
        summary,
        total,
        builds,
        skill_levels,
        item_events,
        splits,
        max_build_variants_per_category: report.max_build_variants_per_category,
        omitted_build_variants: report.omitted_build_variants,
        build_stage_method: report.build_stage_method,
        item_catalog_version,
    })
}

/// Population lue dans l'instantané ; `patch` absent sélectionne tous les patchs publiés.
pub(crate) struct Selection<'a> {
    pub patch: Option<&'a str>,
    pub platform: &'a str,
    pub queue: i32,
    pub role: &'a str,
    pub rank: &'a str,
    pub champion_id: Option<u32>,
    /// Charge aussi builds, compétences et achats du champion (inutile pour une série).
    pub with_details: bool,
}

async fn load(
    pool: &PgPool,
    query: &StatsQuery,
    champion_id: Option<u32>,
) -> Result<(SnapshotMeta, AggregationReport), ApiError> {
    load_selection(
        pool,
        &Selection {
            patch: Some(&query.patch),
            platform: &query.platform,
            queue: query.queue,
            role: &query.role,
            rank: &query.rank,
            champion_id,
            with_details: champion_id.is_some(),
        },
    )
    .await
}

pub(crate) async fn load_selection(
    pool: &PgPool,
    selection: &Selection<'_>,
) -> Result<(SnapshotMeta, AggregationReport), ApiError> {
    // Une seule lecture cohérente : sélection indexée des morceaux avant le filtre JSON.
    // La tierlist ne charge pas les builds ni les événements de tous les champions.
    let vars = serde_json::json!({"patch":selection.patch,"platform":selection.platform,"queue":selection.queue,"role":selection.role,"rank":selection.rank,"champion":selection.champion_id});
    let population = "$[*] ? (($patch == null || @.patch == $patch) && @.platform_id == $platform && @.queue_id == $queue && @.role == $role && @.rank == $rank && ($champion == null || @.champion_id == $champion))";
    let scope = "$[*] ? (($patch == null || @.patch == $patch) && @.platform_id == $platform && @.queue_id == $queue)";
    let rows = sqlx::query(include_str!("sql/stats_snapshot.sql"))
        .bind(vars)
        .bind(population)
        .bind(scope)
        .bind(selection.with_details)
        .fetch_all(pool)
        .await?;
    let row = rows.first().ok_or(ApiError::Unavailable)?;
    let storage_version: i16 = row.try_get("storage_version")?;
    if ![1, 2].contains(&storage_version) {
        return Err(ApiError::Unavailable);
    }
    let mut value: serde_json::Value = row.try_get("report")?;
    // Une seule requête lit la tête et les morceaux de la même publication.
    // Le filtrage SQL précède le transfert ; aucun JSONB global n'est reconstruit.
    if storage_version == 2 {
        for chunk in &rows {
            if let Some(section) = chunk.try_get::<Option<String>, _>("section")? {
                let items: sqlx::types::Json<Vec<serde_json::Value>> = chunk.try_get("items")?;
                value
                    .get_mut(&section)
                    .and_then(serde_json::Value::as_array_mut)
                    .ok_or(ApiError::Unavailable)?
                    .extend(items.0);
            }
        }
    }
    let report: AggregationReport =
        serde_json::from_value(value).map_err(|_| ApiError::Unavailable)?;
    if report.schema_version != 2 || report.min_games == 0 {
        return Err(ApiError::Unavailable);
    }
    let coverage: Vec<ScopeCoverage> = report
        .coverage
        .iter()
        .filter(|c| selection.patch.is_none() || selection.patch == Some(c.scope.patch.as_str()))
        .filter(|c| {
            c.scope.platform_id == selection.platform && c.scope.queue_id == selection.queue
        })
        .cloned()
        .collect();
    let source_snapshot_at: String = row.try_get("source_snapshot_at")?;
    let meta = SnapshotMeta {
        freshness: freshness(&source_snapshot_at, &coverage),
        source_snapshot_at,
        published_at: row.try_get("published_at")?,
        schema_version: report.schema_version,
        min_games: report.min_games,
        rank_scope: report.rank_scope.clone(),
        rank_max_age_hours: report.rank_max_age_hours,
        pick_rate_definition: report.pick_rate_definition.clone(),
        tier_method: report.tier_method.clone(),
        filters: report.filters.clone(),
        coverage,
    };
    Ok((meta, report))
}
/// Bornes des parties incluses sur les périmètres lus ; une date absente d'un périmètre
/// (instantané antérieur) ne masque pas celles des autres.
fn freshness(computed_at: &str, coverage: &[ScopeCoverage]) -> Freshness {
    let first = coverage.iter().filter_map(|c| c.counts.first_game_start_ms);
    let last = coverage.iter().filter_map(|c| c.counts.last_game_start_ms);
    Freshness {
        computed_at: computed_at.to_owned(),
        first_game_start_ms: first.min(),
        last_game_start_ms: last.max(),
    }
}
fn scope_matches(scope: &ScopeKey, query: &StatsQuery) -> bool {
    scope.patch == query.patch
        && scope.platform_id == query.platform
        && scope.queue_id == query.queue
}
fn matches(key: &GroupKey, query: &StatsQuery) -> bool {
    let role = match key.role {
        Role::Top => "TOP",
        Role::Jungle => "JUNGLE",
        Role::Middle => "MIDDLE",
        Role::Bottom => "BOTTOM",
        Role::Utility => "UTILITY",
        Role::Unknown => "UNKNOWN",
    };
    key.patch == query.patch
        && key.platform_id == query.platform
        && key.queue_id == query.queue
        && key.rank == query.rank
        && role == query.role
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(patch: &str, first: Option<i64>, last: Option<i64>) -> ScopeCoverage {
        let mut value = serde_json::to_value(Coverage::default()).unwrap();
        value["patch"] = patch.into();
        value["platform_id"] = "EUW1".into();
        value["queue_id"] = 420.into();
        value["first_game_start_ms"] = first.into();
        value["last_game_start_ms"] = last.into();
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn la_fraicheur_borne_les_parties_de_tous_les_perimetres_lus() {
        let coverage = [
            scope("16.18", Some(100), Some(500)),
            scope("16.19", Some(300), Some(900)),
            scope("16.20", None, None),
        ];
        assert_eq!(
            freshness("2026-10-04 10:00:00+00", &coverage),
            Freshness {
                computed_at: "2026-10-04 10:00:00+00".into(),
                first_game_start_ms: Some(100),
                last_game_start_ms: Some(900),
            }
        );
        assert_eq!(freshness("x", &[]).last_game_start_ms, None);
    }
}
