//! Tendances entre patchs (#110) : une série par champion, rôle et rang, calculée à
//! partir de l'instantané publié, sans table historisée ni mélange de populations.
use crate::{
    error::ApiError,
    query::TrendsQuery,
    stats::{self, Selection, SnapshotMeta},
};
use olc_collector::aggregation::*;
use serde::Serialize;
use sqlx::PgPool;
use std::collections::BTreeMap;

/// Un patch de la série. Les taux sont des pourcentages, nuls sous le seuil de l'instantané.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrendPoint {
    pub patch: String,
    /// Parties du champion dans la population ; 0 si l'instantané n'en contient aucune.
    pub games: u64,
    pub wins: u64,
    /// Participations de tous les champions du même patch/rôle/rang (dénominateur du pick).
    pub population: u64,
    /// Nul en Arena (#104), comme dans la tierlist.
    pub win_rate: Option<f64>,
    pub pick_rate: Option<f64>,
    /// Parties de draft du patch où le champion est banni ; 0 sans entrée de ban.
    pub banned_matches: u64,
    pub draft_matches: u64,
    pub ban_rate: Option<f64>,
    /// Écart en points de pourcentage avec le patch publié précédent ; nul si l'un des
    /// deux winrates est nul, ou pour le premier patch de la série.
    pub delta_win_rate: Option<f64>,
    pub delta_pick_rate: Option<f64>,
    pub delta_ban_rate: Option<f64>,
}

#[derive(Serialize)]
pub struct TrendsResponse {
    pub meta: SnapshotMeta,
    pub query: TrendsQuery,
    pub champion_id: u32,
    /// Du plus ancien au plus récent patch publié ; au plus un point par patch.
    pub points: Vec<TrendPoint>,
}

/// Série patch par patch du champion pour la population demandée.
pub async fn trends(
    pool: &PgPool,
    query: TrendsQuery,
    champion_id: u32,
) -> Result<TrendsResponse, ApiError> {
    query.validate().map_err(|_| ApiError::InvalidRequest)?;
    if champion_id == 0 {
        return Err(ApiError::InvalidRequest);
    }
    let (meta, report) = stats::load_selection(
        pool,
        &Selection {
            patch: None,
            platform: &query.platform,
            queue: query.queue,
            role: Some(&query.role),
            rank: &query.rank,
            champion_id: Some(champion_id),
            with_details: false,
        },
    )
    .await?;
    let points = series(&report, &query, champion_id);
    Ok(TrendsResponse {
        meta,
        query,
        champion_id,
        points,
    })
}

/// Fonction pure : ne retient que les entrées de la population demandée.
/// Un patch connu de l'instantané mais sans ligne du champion devient un point vide
/// (0 partie, taux nuls) plutôt que d'être omis : l'absence reste visible dans la série.
pub(crate) fn series(
    report: &AggregationReport,
    query: &TrendsQuery,
    champion_id: u32,
) -> Vec<TrendPoint> {
    let in_scope = |scope: &ScopeKey| {
        scope.platform_id == query.platform
            && scope.queue_id == query.queue
            && patch_order(&scope.patch).is_some()
    };
    let groups: BTreeMap<_, _> = report
        .groups
        .iter()
        .filter(|g| {
            g.key.champion_id == champion_id
                && g.key.platform_id == query.platform
                && g.key.queue_id == query.queue
                && g.key.rank == query.rank
                && role_name(g.key.role) == query.role
        })
        .filter_map(|g| Some((patch_order(&g.key.patch)?, g)))
        .collect();
    let bans: BTreeMap<_, _> = report
        .bans
        .iter()
        .filter(|b| b.champion_id == champion_id && in_scope(&b.scope))
        .filter_map(|b| Some((patch_order(&b.scope.patch)?, b)))
        .collect();
    let drafts: BTreeMap<_, _> = report
        .coverage
        .iter()
        .filter(|c| in_scope(&c.scope))
        .filter_map(|c| {
            Some((
                patch_order(&c.scope.patch)?,
                (&c.scope.patch, c.counts.draft_matches),
            ))
        })
        .collect();
    // Les patchs de la série : ceux que l'instantané a observés pour ce périmètre.
    let mut patches: BTreeMap<(u32, u32), &str> = BTreeMap::new();
    for (order, (patch, _)) in &drafts {
        patches.insert(*order, patch);
    }
    for (order, group) in &groups {
        patches.insert(*order, &group.key.patch);
    }
    for (order, ban) in &bans {
        patches.insert(*order, &ban.scope.patch);
    }
    let min_games = u64::from(report.min_games);
    let mut points: Vec<TrendPoint> = Vec::with_capacity(patches.len());
    for (order, patch) in patches {
        let group = groups.get(&order);
        let (banned_matches, draft_matches) = match (bans.get(&order), drafts.get(&order)) {
            (Some(ban), _) => (ban.banned_matches, ban.draft_matches),
            (None, Some((_, draft_matches))) => (0, *draft_matches),
            (None, None) => (0, 0),
        };
        // Même règle que le collecteur : pas de taux sous le seuil de drafts observées.
        let ban_rate = match bans.get(&order) {
            Some(ban) => ban.ban_rate,
            None => (draft_matches >= min_games && draft_matches > 0).then_some(0.0),
        };
        let previous = points.last();
        let win_rate = group.and_then(|g| g.win_rate);
        let pick_rate = group.and_then(|g| g.pick_rate);
        points.push(TrendPoint {
            patch: patch.to_string(),
            games: group.map_or(0, |g| g.games),
            wins: group.map_or(0, |g| g.wins),
            population: group.map_or(0, |g| g.population),
            win_rate,
            pick_rate,
            banned_matches,
            draft_matches,
            ban_rate,
            delta_win_rate: delta(win_rate, previous.and_then(|p| p.win_rate)),
            delta_pick_rate: delta(pick_rate, previous.and_then(|p| p.pick_rate)),
            delta_ban_rate: delta(ban_rate, previous.and_then(|p| p.ban_rate)),
        });
    }
    points
}

/// Écart en points de pourcentage ; nul dès qu'une des deux valeurs est inconnue.
fn delta(current: Option<f64>, previous: Option<f64>) -> Option<f64> {
    Some(current? - previous?)
}

/// Clé d'ordre numérique : `16.10` suit `16.9`. Les patchs illisibles sont ignorés.
fn patch_order(patch: &str) -> Option<(u32, u32)> {
    let (major, minor) = patch.split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Top => "TOP",
        Role::Jungle => "JUNGLE",
        Role::Middle => "MIDDLE",
        Role::Bottom => "BOTTOM",
        Role::Utility => "UTILITY",
        Role::Unknown => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn query() -> TrendsQuery {
        TrendsQuery {
            platform: "EUW1".into(),
            queue: 420,
            role: "TOP".into(),
            rank: "ALL".into(),
        }
    }
    fn group(patch: &str, champion: u32, games: u64, win_rate: Option<f64>) -> Value {
        json!({
            "patch": patch, "platform_id": "EUW1", "queue_id": 420, "role": "TOP",
            "rank": "ALL", "champion_id": champion, "games": games, "wins": games / 2,
            "losses": games - games / 2, "population": 1000, "win_rate": win_rate,
            "pick_rate": win_rate.map(|_| games as f64 / 10.0), "win_rate_lower_bound": null,
            "position": null, "tier": null, "most_picked_rank": null
        })
    }
    fn ban(patch: &str, champion: u32, banned: u64) -> Value {
        json!({
            "patch": patch, "platform_id": "EUW1", "queue_id": 420, "champion_id": champion,
            "banned_matches": banned, "draft_matches": 200, "ban_rate": banned as f64 / 2.0
        })
    }
    fn coverage(patch: &str, draft_matches: u64) -> Value {
        let mut value = json!({
            "patch": patch, "platform_id": "EUW1", "queue_id": 420,
            "matches": 0, "participations": 0, "excluded_bot_participations": 0,
            "ranked_participations": 0, "unranked_participations": 0,
            "unknown_rank_participations": 0, "unranked_mode_participations": 0,
            "unknown_role_participations": 0, "timeline_matches": 0,
            "timeline_participations": 0, "invalid_timeline_participations": 0,
            "unidentified_item_undos": 0, "draft_matches": draft_matches
        });
        value["matches"] = json!(draft_matches);
        value
    }
    fn report(coverage: Vec<Value>, groups: Vec<Value>, bans: Vec<Value>) -> AggregationReport {
        serde_json::from_value(json!({
            "schema_version": 2, "rank_scope": "x", "rank_max_age_hours": 168,
            "pick_rate_definition": "participations_in_group", "tier_method": "wilson_lower_bound",
            "min_games": 100,
            "filters": {"patches": [], "platforms": [], "queues": [], "start_ms": null, "end_ms": null},
            "source_matches": 0, "included_matches": 0, "exclusions": {},
            "coverage": coverage, "groups": groups, "bans": bans, "builds": [],
            "skill_levels": [], "item_events": [],
            "max_build_variants_per_category": 20, "omitted_build_variants": 0
        }))
        .unwrap()
    }
    fn close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("valeur attendue");
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }

    #[test]
    fn la_serie_est_triee_par_patch_numerique_avec_les_ecarts_au_patch_precedent() {
        // 16.9 < 16.10 : l'ordre est numérique, pas alphabétique.
        let report = report(
            vec![coverage("16.10", 200), coverage("16.9", 200)],
            vec![
                group("16.10", 1, 300, Some(53.5)),
                group("16.9", 1, 250, Some(50.0)),
            ],
            vec![ban("16.10", 1, 40), ban("16.9", 1, 20)],
        );
        let points = series(&report, &query(), 1);
        assert_eq!(
            points.iter().map(|p| p.patch.as_str()).collect::<Vec<_>>(),
            ["16.9", "16.10"]
        );
        assert_eq!(points[0].games, 250);
        assert_eq!(points[1].games, 300);
        assert_eq!(points[1].population, 1000);
        assert_eq!(points[1].banned_matches, 40);
        assert_eq!(points[1].draft_matches, 200);
        close(points[1].win_rate, 53.5);
        close(points[1].ban_rate, 20.0);
        assert_eq!(points[0].delta_win_rate, None);
        assert_eq!(points[0].delta_pick_rate, None);
        assert_eq!(points[0].delta_ban_rate, None);
        close(points[1].delta_win_rate, 3.5);
        close(points[1].delta_pick_rate, 5.0);
        close(points[1].delta_ban_rate, 10.0);
    }

    #[test]
    fn la_serie_ne_melange_ni_champion_ni_role_ni_rang_ni_plateforme() {
        let mut other_role = group("16.9", 1, 999, Some(99.0));
        other_role["role"] = json!("MIDDLE");
        let mut other_rank = group("16.9", 1, 998, Some(98.0));
        other_rank["rank"] = json!("GOLD");
        let mut other_platform = group("16.9", 1, 997, Some(97.0));
        other_platform["platform_id"] = json!("KR");
        let mut other_queue = group("16.9", 1, 996, Some(96.0));
        other_queue["queue_id"] = json!(440);
        let mut foreign_ban = ban("16.9", 1, 77);
        foreign_ban["platform_id"] = json!("KR");
        let report = report(
            vec![coverage("16.9", 200)],
            vec![
                group("16.9", 1, 250, Some(50.0)),
                group("16.9", 2, 123, Some(40.0)),
                other_role,
                other_rank,
                other_platform,
                other_queue,
            ],
            vec![ban("16.9", 2, 60), foreign_ban],
        );
        let points = series(&report, &query(), 1);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].games, 250);
        assert_eq!(points[0].banned_matches, 0);
    }

    #[test]
    fn un_patch_sans_donnee_du_champion_reste_un_point_vide_et_coupe_les_ecarts() {
        let report = report(
            vec![
                coverage("16.8", 300),
                coverage("16.9", 200),
                coverage("16.10", 200),
            ],
            vec![
                group("16.8", 1, 250, Some(50.0)),
                group("16.10", 1, 300, Some(55.0)),
            ],
            vec![],
        );
        let points = series(&report, &query(), 1);
        assert_eq!(points.len(), 3);
        let empty = &points[1];
        assert_eq!(empty.patch, "16.9");
        assert_eq!((empty.games, empty.wins, empty.population), (0, 0, 0));
        assert_eq!(empty.win_rate, None);
        assert_eq!(empty.pick_rate, None);
        // Aucune ligne de ban et assez de drafts : 0 % réel, pas une valeur inconnue.
        assert_eq!(empty.banned_matches, 0);
        assert_eq!(empty.draft_matches, 200);
        close(empty.ban_rate, 0.0);
        // Pas d'écart contre un patch vide : on ne compare qu'à des valeurs connues.
        assert_eq!(empty.delta_win_rate, None);
        assert_eq!(points[2].delta_win_rate, None);
        assert_eq!(points[2].delta_pick_rate, None);
    }

    #[test]
    fn sous_le_seuil_les_taux_restent_nuls_et_aucun_ecart_n_est_fabrique() {
        let mut thin = group("16.10", 1, 3, None);
        thin["wins"] = json!(1);
        let report = report(
            vec![coverage("16.9", 200), coverage("16.10", 40)],
            vec![group("16.9", 1, 250, Some(50.0)), thin],
            vec![],
        );
        let points = series(&report, &query(), 1);
        assert_eq!(points[1].games, 3);
        assert_eq!(points[1].wins, 1);
        assert_eq!(points[1].win_rate, None);
        assert_eq!(points[1].delta_win_rate, None);
        // 40 drafts < min_games (100) : le ban rate n'est pas publiable.
        assert_eq!(points[1].ban_rate, None);
        assert_eq!(points[1].delta_ban_rate, None);
    }

    #[test]
    fn un_instantane_sans_patch_pour_la_population_donne_une_serie_vide() {
        let report = report(vec![], vec![], vec![]);
        assert!(series(&report, &query(), 1).is_empty());
    }
}
