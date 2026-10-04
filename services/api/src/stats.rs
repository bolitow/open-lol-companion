//! Lecture des instantanés publiés par #18, sans recalcul des taux ni mélange de populations.
//! Seul l'indicateur de biais d'une couverture ancienne est complété à la lecture (#82).
use crate::{
    error::ApiError,
    query::{BansQuery, StatsQuery},
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
    /// Origine du rang des bans (#109) : `match_median`, médiane des paliers observés des
    /// joueurs de la partie ; vide pour un instantané antérieur.
    pub ban_rank_basis: String,
    /// Joueurs connus minimaux (sur dix) pour qu'une partie reçoive un palier ; 0 antérieurement.
    pub ban_rank_min_known_players: u32,
    /// Parties sources écartées par motif (`remake`, `short_game`, `afk`, `early_departure`…).
    pub exclusions: BTreeMap<String, u64>,
    pub pick_rate_definition: String,
    pub tier_method: String,
    pub filters: AggregationOptions,
    /// Nature de la population servie pour le `rank` demandé (#82). `ALL` est un échantillon
    /// collecté, non repondéré sur le ladder : sa répartition est `coverage[].tier_participations`.
    pub population_label: PopulationLabel,
    pub coverage: Vec<ScopeCoverage>,
}
/// Étiquette honnête de la population demandée (#82), identifiant stable traduit par l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PopulationLabel {
    /// `ALL` : toutes les participations collectées du périmètre, sans pondération par la
    /// taille réelle des paliers ; ce n'est pas « tous les rangs » du ladder.
    CollectedSample,
    /// Palier Riot observé du joueur, figé à la partie (#80).
    ObservedTier,
    /// Palier Riot de la partie (#109, médiane des joueurs) : population des bans.
    MatchTier,
    /// `UNKNOWN` : aucun rang observé assez proche de la partie (ou partie sans palier).
    UnknownRank,
    /// `UNRANKED` : joueur observé sans classement dans la file.
    Unranked,
    /// `UNRANKED_MODE` : file sans rang compétitif.
    UnrankedMode,
}
/// Dérive l'étiquette du `rank` validé ; `by_player` est faux pour les bans, rangés au palier
/// de leur partie. L'étiquette ne juge pas la répartition : elle est publiée à côté, avec son biais.
fn population_label(rank: &str, by_player: bool) -> PopulationLabel {
    match rank {
        "ALL" => PopulationLabel::CollectedSample,
        "UNKNOWN" => PopulationLabel::UnknownRank,
        "UNRANKED" => PopulationLabel::Unranked,
        "UNRANKED_MODE" => PopulationLabel::UnrankedMode,
        _ if by_player => PopulationLabel::ObservedTier,
        _ => PopulationLabel::MatchTier,
    }
}
#[derive(Serialize)]
pub struct TierlistResponse {
    pub meta: SnapshotMeta,
    pub query: StatsQuery,
    pub total: usize,
    pub entries: Vec<ChampionStats>,
    pub bans: Vec<BanStats>,
}
/// Bans d'une draft pour un périmètre et un palier de partie, indépendants de la tierlist.
#[derive(Serialize)]
pub struct BansResponse {
    pub meta: SnapshotMeta,
    pub query: BansQuery,
    /// Bans publiés pour ce palier, avant `limit`.
    pub total: usize,
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
    let (meta, report) = load(pool, &query, None, true).await?;
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
                && b.rank == query.rank
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
/// Bans les plus fréquents du palier de partie demandé, du plus au moins banni ; un taux
/// absent (échantillon sous le seuil) passe après les taux publiés. Aucun rôle, aucune page.
pub async fn bans(pool: &PgPool, query: BansQuery) -> Result<BansResponse, ApiError> {
    query.validate().map_err(|_| ApiError::InvalidRequest)?;
    let scope = query.as_stats_query();
    let (meta, report) = load(pool, &scope, None, false).await?;
    let mut entries: Vec<_> = report
        .bans
        .into_iter()
        .filter(|b| scope_matches(&b.scope, &scope) && b.rank == query.rank)
        .collect();
    entries.sort_by(|a, b| {
        b.ban_rate
            .unwrap_or(-1.0)
            .total_cmp(&a.ban_rate.unwrap_or(-1.0))
            .then(b.banned_matches.cmp(&a.banned_matches))
            .then(a.champion_id.cmp(&b.champion_id))
    });
    let total = entries.len();
    entries.truncate(query.limit);
    Ok(BansResponse {
        meta,
        query,
        total,
        bans: entries,
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
    let (meta, report) = load(pool, &query, Some(champion_id), true).await?;
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
    groups: bool,
) -> Result<(SnapshotMeta, AggregationReport), ApiError> {
    // Une seule lecture cohérente : sélection indexée des morceaux avant le filtre JSON.
    // La tierlist ne charge pas les builds ni les événements de tous les champions.
    // Sans `groups`, le rôle est nul : la requête SQL ne lit alors aucun morceau de classement.
    let role = groups.then_some(&query.role);
    let vars = serde_json::json!({"patch":query.patch,"platform":query.platform,"queue":query.queue,"role":role,"rank":query.rank,"champion":champion_id});
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
    let mut report: AggregationReport =
        serde_json::from_value(value).map_err(|_| ApiError::Unavailable)?;
    if report.schema_version != 2 || report.min_games == 0 {
        return Err(ApiError::Unavailable);
    }
    // Un instantané publié avant l'indicateur de biais (#82) a sa répartition de paliers mais pas
    // `apex_share` : le servir avec `null`/`false` le ferait passer pour non biaisé.
    for coverage in &mut report.coverage {
        coverage.counts.complete_tier_bias();
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
        ban_rank_basis: report.ban_rank_basis.clone(),
        ban_rank_min_known_players: report.ban_rank_min_known_players,
        exclusions: report.exclusions.clone(),
        pick_rate_definition: report.pick_rate_definition.clone(),
        tier_method: report.tier_method.clone(),
        filters: report.filters.clone(),
        // `groups` n'est faux que pour les bans, rangés au palier de leur partie (#109).
        population_label: population_label(&query.rank, groups),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_est_etiquete_echantillon_collecte_jamais_tous_les_rangs() {
        assert_eq!(
            population_label("ALL", true),
            PopulationLabel::CollectedSample
        );
        assert_eq!(
            population_label("ALL", false),
            PopulationLabel::CollectedSample
        );
        let json = serde_json::to_value(PopulationLabel::CollectedSample).unwrap();
        assert_eq!(json, "collected_sample");
    }

    #[test]
    fn un_palier_designe_le_rang_du_joueur_ou_celui_de_la_partie_pour_les_bans() {
        for tier in ["IRON", "GOLD", "MASTER", "CHALLENGER"] {
            assert_eq!(population_label(tier, true), PopulationLabel::ObservedTier);
            assert_eq!(population_label(tier, false), PopulationLabel::MatchTier);
        }
        for (rank, label) in [
            ("UNKNOWN", PopulationLabel::UnknownRank),
            ("UNRANKED", PopulationLabel::Unranked),
            ("UNRANKED_MODE", PopulationLabel::UnrankedMode),
        ] {
            assert_eq!(population_label(rank, true), label);
            assert_eq!(population_label(rank, false), label);
        }
    }
}
