//! Lecture des instantanés publiés par #18, sans recalcul des taux ni mélange de populations.
//! Seul l'indicateur de biais d'une couverture ancienne est complété à la lecture (#82).
use crate::{
    error::ApiError,
    query::{BansQuery, BuildSort, StatsQuery},
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
    /// Plancher de fiabilité (#91), indépendant de `min_games` : sous cet effectif, un taux est
    /// signalé `low`. 0 pour un instantané antérieur, qui ne porte aucune fiabilité.
    pub reliability_floor: u32,
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
    pub freshness: Freshness,
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
    /// Tri appliqué aux variantes avant pagination (#112).
    pub sort: BuildSort,
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
    /// Variantes non publiées pour le seul groupe demandé (#113), somme de
    /// `omitted_build_variants_by_category` ; le compteur global du snapshot n'est pas servi.
    /// Nul pour un instantané antérieur, où ce compte est inconnu.
    pub omitted_build_variants: Option<u64>,
    /// Variantes non publiées par catégorie du groupe demandé ; liste vide si inconnu.
    pub omitted_build_variants_by_category: Vec<OmittedBuildVariants>,
    /// Plafond d'`item_events` servis pour le groupe demandé.
    pub max_item_events: u32,
    /// Lignes d'`item_events` du groupe retirées par ce plafond (les moins fréquentes).
    pub omitted_item_events: u64,
    /// Règles des étapes d'achat (#81) ; vide pour un instantané antérieur.
    pub build_stage_method: String,
    /// Version du catalogue d'objets jointe au patch demandé ; nulle sans étapes.
    pub item_catalog_version: Option<String>,
}
/// Moyennes de performance d'un champion (#100), sans note ni comparaison.
#[derive(Serialize)]
pub struct PerformanceResponse {
    pub meta: SnapshotMeta,
    pub query: StatsQuery,
    pub champion_id: u32,
    pub summary: Option<ChampionStats>,
    /// Nulle si la population n'a aucune participation ou si l'instantané précède #100.
    pub performance: Option<PerformanceStats>,
    /// Définitions publiées avec l'instantané ; vide pour un instantané antérieur.
    pub performance_method: String,
}
/// Matchups de lane d'un champion (#123) : agrégats seulement, sans recommandation.
#[derive(Serialize)]
pub struct MatchupsResponse {
    pub meta: SnapshotMeta,
    pub query: StatsQuery,
    pub champion_id: u32,
    pub summary: Option<ChampionStats>,
    /// Adversaires publiés pour la population, avant pagination.
    pub total: usize,
    /// Parties du champion appariées à un adversaire de lane, avant pagination.
    pub paired_games: u64,
    pub matchups: Vec<MatchupStats>,
    /// Définitions publiées avec l'instantané ; vide pour un instantané antérieur.
    pub matchup_method: String,
}
/// Variantes de builds coupées par le plafond de publication dans une catégorie (#113).
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct OmittedBuildVariants {
    pub category: String,
    pub omitted: u32,
}
/// Nombre maximal de lignes `item_events` servies pour un groupe : l'effectif par groupe
/// croît avec le volume collecté (objets × minutes) et la réponse n'est pas paginée.
pub const MAX_ITEM_EVENTS: u32 = 2000;
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
    // Avant pagination : le compte décrit tout le groupe, pas la page demandée.
    let (omitted_build_variants, omitted_build_variants_by_category) = omitted_variants(&variants);
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
    let item_events: Vec<_> = report
        .item_events
        .into_iter()
        .filter(|b| selected(&b.key))
        .collect();
    let (item_events, omitted_item_events) = cap_item_events(item_events, MAX_ITEM_EVENTS);
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
        sort,
        champion_id,
        summary,
        total,
        builds,
        skill_levels,
        item_events,
        splits,
        max_build_variants_per_category: report.max_build_variants_per_category,
        omitted_build_variants,
        omitted_build_variants_by_category,
        max_item_events: MAX_ITEM_EVENTS,
        omitted_item_events,
        build_stage_method: report.build_stage_method,
        item_catalog_version,
    })
}

/// Moyennes publiées du champion dans la population demandée, lues sans recalcul.
pub async fn performance(
    pool: &PgPool,
    query: StatsQuery,
    champion_id: u32,
) -> Result<PerformanceResponse, ApiError> {
    query.validate().map_err(|_| ApiError::InvalidRequest)?;
    if champion_id == 0 {
        return Err(ApiError::InvalidRequest);
    }
    let (meta, report) = load(pool, &query, Some(champion_id), true).await?;
    let selected = |key: &GroupKey| key.champion_id == champion_id && matches(key, &query);
    let summary = report.groups.into_iter().find(|g| selected(&g.key));
    let performance = report.performance.into_iter().find(|p| selected(&p.key));
    Ok(PerformanceResponse {
        meta,
        query,
        champion_id,
        summary,
        performance,
        performance_method: report.performance_method,
    })
}

/// Adversaires de lane du champion dans la population demandée, lus sans recalcul.
/// Seul le rang `ALL` est publié : un rang observé renvoie une liste vide.
pub async fn matchups(
    pool: &PgPool,
    query: StatsQuery,
    champion_id: u32,
) -> Result<MatchupsResponse, ApiError> {
    query.validate().map_err(|_| ApiError::InvalidRequest)?;
    if champion_id == 0 {
        return Err(ApiError::InvalidRequest);
    }
    let (meta, report) = load(pool, &query, Some(champion_id), true).await?;
    let selected = |key: &GroupKey| key.champion_id == champion_id && matches(key, &query);
    let summary = report.groups.into_iter().find(|g| selected(&g.key));
    let mut rows: Vec<_> = report
        .matchups
        .into_iter()
        .filter(|m| selected(&m.key))
        .collect();
    rows.sort_by(|a, b| {
        b.games
            .cmp(&a.games)
            .then(a.opponent_champion_id.cmp(&b.opponent_champion_id))
    });
    let total = rows.len();
    let paired_games = rows.iter().map(|m| m.games).sum();
    let matchups = rows
        .into_iter()
        .skip(query.offset)
        .take(query.limit)
        .collect();
    Ok(MatchupsResponse {
        meta,
        query,
        champion_id,
        summary,
        total,
        paired_games,
        matchups,
        matchup_method: report.matchup_method,
    })
}

/// Variantes omises par catégorie du groupe, lues sur les variantes publiées (le compteur est
/// identique pour toutes celles d'une catégorie). Une catégorie sans compteur (instantané
/// antérieur) est absente de la liste et rend le total inconnu, jamais nul.
fn omitted_variants(variants: &[BuildStats]) -> (Option<u64>, Vec<OmittedBuildVariants>) {
    let mut known = BTreeMap::<&str, Option<u32>>::new();
    for variant in variants {
        known
            .entry(&variant.category)
            .or_insert(variant.omitted_variants);
    }
    let complete = known.values().all(Option::is_some);
    let by_category: Vec<_> = known
        .into_iter()
        .filter_map(|(category, omitted)| {
            omitted.map(|omitted| OmittedBuildVariants {
                category: category.to_owned(),
                omitted,
            })
        })
        .collect();
    let total = complete.then(|| by_category.iter().map(|c| u64::from(c.omitted)).sum());
    (total, by_category)
}

/// Garde les `max` lignes les plus fréquentes (égalité : événement, objet, minute) et rend
/// le nombre de lignes retirées ; la liste reste dans l'ordre naturel de lecture.
fn cap_item_events(mut events: Vec<ItemEventStats>, max: u32) -> (Vec<ItemEventStats>, u64) {
    let max = max as usize;
    if events.len() <= max {
        return (events, 0);
    }
    let omitted = (events.len() - max) as u64;
    events.sort_by(|a, b| {
        b.events
            .cmp(&a.events)
            .then_with(|| (&a.event, a.item_id, a.minute).cmp(&(&b.event, b.item_id, b.minute)))
    });
    events.truncate(max);
    events.sort_by(|a, b| (&a.event, a.item_id, a.minute).cmp(&(&b.event, b.item_id, b.minute)));
    (events, omitted)
}

/// Population lue dans l'instantané ; `patch` absent sélectionne tous les patchs publiés.
pub(crate) struct Selection<'a> {
    pub patch: Option<&'a str>,
    pub platform: &'a str,
    pub queue: i32,
    /// `None` sans classement à lire : aucun morceau `groups` n'est chargé (#109).
    pub role: Option<&'a str>,
    pub rank: &'a str,
    pub champion_id: Option<u32>,
    /// Charge aussi builds, compétences et achats du champion (inutile pour une série).
    pub with_details: bool,
}

async fn load(
    pool: &PgPool,
    query: &StatsQuery,
    champion_id: Option<u32>,
    groups: bool,
) -> Result<(SnapshotMeta, AggregationReport), ApiError> {
    load_selection(
        pool,
        &Selection {
            patch: Some(&query.patch),
            platform: &query.platform,
            queue: query.queue,
            role: groups.then_some(query.role.as_str()),
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
    // Sans `groups`, le rôle est nul : la requête SQL ne lit alors aucun morceau de classement.
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
        reliability_floor: report.reliability_floor,
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
        // Le rôle n'est nul que pour les bans, rangés au palier de leur partie (#109).
        population_label: population_label(selection.rank, selection.role.is_some()),
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
    use serde_json::json;

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

    fn build(category: &str, selection: u32, omitted: Option<u32>) -> BuildStats {
        serde_json::from_value(json!({
            "patch":"16.19", "platform_id":"EUW1", "queue_id":420,
            "role":"TOP", "rank":"ALL", "champion_id":1,
            "category":category, "selection":[selection], "games":10,
            "wins":5, "performance_available":true, "population":10,
            "pick_rate":null, "win_rate":null, "omitted_variants":omitted
        }))
        .unwrap()
    }

    #[test]
    fn les_variantes_omises_se_lisent_par_categorie_puis_se_totalisent() {
        let builds = [
            build("summoner_spells", 1, Some(5)),
            build("summoner_spells", 2, Some(5)),
            build("runes", 1, Some(0)),
            build("final_items", 1, Some(12)),
        ];
        let (total, by_category) = omitted_variants(&builds);
        assert_eq!(total, Some(17));
        let read: Vec<_> = by_category
            .iter()
            .map(|c| (c.category.as_str(), c.omitted))
            .collect();
        assert_eq!(
            read,
            [("final_items", 12), ("runes", 0), ("summoner_spells", 5)]
        );
    }

    #[test]
    fn des_variantes_omises_inconnues_ne_sont_jamais_presentees_comme_zero() {
        // Instantané antérieur : aucune variante ne porte le compteur.
        let (total, by_category) = omitted_variants(&[build("runes", 1, None)]);
        assert_eq!((total, by_category.len()), (None, 0));
        // Mélange : seule la catégorie inconnue manque, le total reste inconnu.
        let (total, by_category) =
            omitted_variants(&[build("runes", 1, None), build("final_items", 1, Some(2))]);
        assert_eq!((total, by_category.len()), (None, 1));
        // Aucune variante observée : rien n'a pu être omis.
        assert_eq!(omitted_variants(&[]).0, Some(0));
    }

    fn event(item_id: u32, minute: u32, events: u64) -> ItemEventStats {
        serde_json::from_value(json!({
            "patch":"16.19", "platform_id":"EUW1", "queue_id":420,
            "role":"TOP", "rank":"ALL", "champion_id":1,
            "event":"ITEM_PURCHASED", "item_id":item_id, "minute":minute, "events":events
        }))
        .unwrap()
    }

    #[test]
    fn les_achats_servis_sont_plafonnes_sur_les_plus_frequents() {
        let all = vec![
            event(3, 2, 5),
            event(1, 1, 9),
            event(2, 1, 5),
            event(4, 3, 1),
        ];
        let (kept, omitted) = cap_item_events(all.clone(), 2);
        assert_eq!(omitted, 2);
        // Les deux plus fréquents ; l'égalité se départage par (objet, minute), le résultat
        // est rendu dans l'ordre naturel de lecture.
        let read: Vec<_> = kept.iter().map(|e| (e.item_id, e.minute)).collect();
        assert_eq!(read, [(1, 1), (2, 1)]);
        let (kept, omitted) = cap_item_events(all.clone(), 4);
        assert_eq!((kept, omitted), (all, 0));
    }

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
