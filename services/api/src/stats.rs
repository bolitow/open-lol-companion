//! Lecture des instantanés publiés par #18, sans recalcul ni mélange de populations.
use crate::{
    error::ApiError,
    query::{BuildSort, StatsQuery},
};
use olc_collector::aggregation::*;
use serde::Serialize;
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct SnapshotMeta {
    pub source_snapshot_at: String,
    pub published_at: String,
    pub schema_version: u32,
    pub min_games: u32,
    pub rank_scope: String,
    pub rank_max_age_hours: u32,
    /// Durée minimale (s) d'une partie classée (#111) ; 0 pour un instantané antérieur.
    pub min_game_duration_s: u32,
    /// Part minimale (%) de la durée jouée par chaque participant (#111) ; 0 antérieurement.
    pub min_played_percent: u32,
    /// Parties classées avec un participant `wasAfk` écartées (`afk`) (#111) ; `false` antérieurement.
    pub exclude_afk: bool,
    /// Parties sources écartées par motif (`remake`, `short_game`, `afk`, `early_departure`…).
    pub exclusions: BTreeMap<String, u64>,
    pub pick_rate_definition: String,
    pub tier_method: String,
    pub filters: AggregationOptions,
    pub coverage: Vec<ScopeCoverage>,
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
    /// Tri appliqué aux variantes avant pagination (#112).
    pub sort: BuildSort,
    pub champion_id: u32,
    pub summary: Option<ChampionStats>,
    pub total: usize,
    pub builds: Vec<BuildStats>,
    pub skill_levels: Vec<SkillStats>,
    pub item_events: Vec<ItemEventStats>,
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
/// Variantes de build, compétences et achats du champion dans la même population,
/// triées par effectif.
pub async fn builds(
    pool: &PgPool,
    query: StatsQuery,
    champion_id: u32,
) -> Result<BuildsResponse, ApiError> {
    builds_sorted(pool, query, champion_id, BuildSort::Games).await
}
/// Comme [`builds`], avec un tri explicite des variantes au sein de chaque catégorie.
pub async fn builds_sorted(
    pool: &PgPool,
    query: StatsQuery,
    champion_id: u32,
    sort: BuildSort,
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
            .then_with(|| match sort {
                BuildSort::Games => std::cmp::Ordering::Equal,
                // Sans borne publiée (sous le seuil, Arena), la variante passe après les autres.
                BuildSort::Performance => b
                    .win_rate_lower_bound
                    .unwrap_or(-1.0)
                    .total_cmp(&a.win_rate_lower_bound.unwrap_or(-1.0)),
            })
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
    let item_catalog_version = report
        .item_catalogs
        .into_iter()
        .find(|c| c.patch == query.patch)
        .map(|c| c.version);
    Ok(BuildsResponse {
        meta,
        query,
        sort,
        champion_id,
        summary,
        total,
        builds,
        skill_levels,
        item_events,
        max_build_variants_per_category: report.max_build_variants_per_category,
        omitted_build_variants: report.omitted_build_variants,
        build_stage_method: report.build_stage_method,
        item_catalog_version,
    })
}

async fn load(
    pool: &PgPool,
    query: &StatsQuery,
    champion_id: Option<u32>,
) -> Result<(SnapshotMeta, AggregationReport), ApiError> {
    // Une seule lecture cohérente : sélection indexée des morceaux avant le filtre JSON.
    // La tierlist ne charge pas les builds ni les événements de tous les champions.
    let vars = serde_json::json!({"patch":query.patch,"platform":query.platform,"queue":query.queue,"role":query.role,"rank":query.rank,"champion":champion_id});
    let population = "$[*] ? (@.patch == $patch && @.platform_id == $platform && @.queue_id == $queue && @.role == $role && @.rank == $rank && ($champion == null || @.champion_id == $champion))";
    let scope = "$[*] ? (@.patch == $patch && @.platform_id == $platform && @.queue_id == $queue)";
    let rows = sqlx::query(include_str!("sql/stats_snapshot.sql"))
        .bind(vars)
        .bind(population)
        .bind(scope)
        .bind(champion_id.is_some())
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
    let meta = SnapshotMeta {
        source_snapshot_at: row.try_get("source_snapshot_at")?,
        published_at: row.try_get("published_at")?,
        schema_version: report.schema_version,
        min_games: report.min_games,
        rank_scope: report.rank_scope.clone(),
        rank_max_age_hours: report.rank_max_age_hours,
        min_game_duration_s: report.min_game_duration_s,
        min_played_percent: report.min_played_percent,
        exclude_afk: report.exclude_afk,
        exclusions: report.exclusions.clone(),
        pick_rate_definition: report.pick_rate_definition.clone(),
        tier_method: report.tier_method.clone(),
        filters: report.filters.clone(),
        coverage: report
            .coverage
            .iter()
            .filter(|c| scope_matches(&c.scope, query))
            .cloned()
            .collect(),
    };
    Ok((meta, report))
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
