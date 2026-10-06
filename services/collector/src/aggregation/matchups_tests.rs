use super::*;
use crate::aggregation::model::{Accumulator, ObservedRank, StoredMatch};
use crate::aggregation::AggregationReport;
use crate::model::fixtures::match_detail;
use serde_json::json;

/// Dix participations du fixture : équipe 100 (indices 0 à 4) contre 200 (5 à 9),
/// rôles TOP, JUNGLE, MIDDLE, BOTTOM, UTILITY dans cet ordre pour chaque équipe.
fn ranked_slots() -> Vec<LaneSlot> {
    let roles = [
        Role::Top,
        Role::Jungle,
        Role::Middle,
        Role::Bottom,
        Role::Utility,
    ];
    (0..10)
        .map(|i| LaneSlot {
            team: if i < 5 { 100 } else { 200 },
            role: roles[i % 5],
            champion: 1 + i as u32,
        })
        .collect()
}

#[test]
fn apparie_chaque_role_classe_avec_l_adversaire_de_l_autre_equipe_dans_les_deux_sens() {
    let slots = ranked_slots();
    for queue in MATCHUP_QUEUES {
        let mut pairs = lane_opponents(queue, &slots);
        pairs.sort_unstable();
        let expected: Vec<(usize, usize)> = (0..5)
            .map(|i| (i, i + 5))
            .chain((0..5).map(|i| (i + 5, i)))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        assert_eq!(pairs, expected, "file {queue}");
    }
}

#[test]
fn n_apparie_rien_hors_files_classees_ni_sans_role_unique_par_equipe() {
    let slots = ranked_slots();
    // Normale, ARAM : l'unicité (équipe, rôle) n'y est pas garantie par la validation.
    for queue in [400, 430, 450, 490] {
        assert!(lane_opponents(queue, &slots).is_empty(), "file {queue}");
    }
    // Rôle inconnu : la lane TOP n'a plus d'adversaire identifiable, les autres restent.
    let mut unknown = slots.clone();
    unknown[5].role = Role::Unknown;
    let pairs = lane_opponents(420, &unknown);
    assert_eq!(pairs.len(), 8);
    assert!(pairs
        .iter()
        .all(|(a, b)| ![0, 5].contains(a) && ![0, 5].contains(b)));
    // Deux TOP dans la même équipe (donnée incohérente) : aucun appariement deviné.
    let mut duplicated = slots.clone();
    duplicated[6].role = Role::Top;
    let pairs = lane_opponents(420, &duplicated);
    assert!(pairs
        .iter()
        .all(|(a, b)| ![0, 5, 6].contains(a) && ![0, 5, 6].contains(b)));
    assert_eq!(pairs.len(), 6);
}

fn ranked_game(id: &str, queue: i32) -> StoredMatch {
    StoredMatch {
        match_id: id.into(),
        platform_id: "EUW1".into(),
        queue_id: queue,
        patch: "15.19".into(),
        is_remake: false,
        game_duration_s: 1800,
        game_start_ms: 1_000_000,
        detail: match_detail(id, "EUW1", queue, 1_000_000),
        timeline: None,
        ranks: [(
            "fake-puuid-0".to_owned(),
            ObservedRank {
                status: "ranked".into(),
                tier: Some("GOLD".into()),
                gap_s: 60,
            },
        )]
        .into_iter()
        .collect(),
    }
}

fn row(report: &AggregationReport, champion: u32, opponent: u32) -> &MatchupStats {
    report
        .matchups
        .iter()
        .find(|m| m.key.champion_id == champion && m.opponent_champion_id == opponent)
        .unwrap()
}

#[test]
fn publie_les_matchups_complementaires_avec_effectif_et_borne_wilson() {
    let mut acc = Accumulator::new(2).unwrap();
    acc.add(&ranked_game("EUW1_1", 420));
    acc.add(&ranked_game("EUW1_2", 420));
    let report = acc.finish();
    assert!(!report.matchup_method.is_empty());
    // Cinq lanes, deux sens : dix lignes, toutes au rang ALL (le rang GOLD du joueur 0
    // est individuel et ne décrit pas la partie).
    assert_eq!(report.matchups.len(), 10);
    assert!(report.matchups.iter().all(|m| m.key.rank == "ALL"));
    let top = row(&report, 1, 6);
    assert_eq!(top.key.role, Role::Top);
    assert_eq!((top.games, top.wins, top.losses), (2, 2, 0));
    assert_eq!(top.win_rate, Some(100.0));
    let bound = top.win_rate_lower_bound.unwrap();
    assert!(bound > 0.0 && bound < 100.0);
    // Le sens inverse est exactement complémentaire : même effectif, victoires opposées.
    let reverse = row(&report, 6, 1);
    assert_eq!((reverse.games, reverse.wins, reverse.losses), (2, 0, 2));
    assert_eq!(reverse.win_rate, Some(0.0));
    assert_eq!(reverse.win_rate_lower_bound, Some(0.0));
    // Pas de croisement entre rôles : le TOP n'a pour adversaire que le TOP adverse.
    assert!(report
        .matchups
        .iter()
        .filter(|m| m.key.champion_id == 1)
        .all(|m| m.opponent_champion_id == 6));
    assert_eq!(report.coverage[0].counts.lane_matchup_participations, 20);
    let encoded = serde_json::to_string(&report.matchups).unwrap();
    for forbidden in ["puuid", "Joueur", "EUW1_1", "riotIdGameName", "fake"] {
        assert!(!encoded.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn sous_le_seuil_l_effectif_est_publie_sans_taux() {
    let mut acc = Accumulator::new(3).unwrap();
    acc.add(&ranked_game("EUW1_1", 440));
    acc.add(&ranked_game("EUW1_2", 440));
    let report = acc.finish();
    let top = row(&report, 1, 6);
    assert_eq!((top.games, top.wins), (2, 2));
    assert_eq!((top.win_rate, top.win_rate_lower_bound), (None, None));
}

#[test]
fn une_partie_normale_ne_publie_aucun_matchup_et_le_dit_dans_la_couverture() {
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&ranked_game("EUW1_1", 400));
    let report = acc.finish();
    assert!(report.matchups.is_empty());
    assert_eq!(report.coverage[0].counts.participations, 10);
    assert_eq!(report.coverage[0].counts.lane_matchup_participations, 0);
}

#[test]
fn normal_mirrors_keep_participations_and_builds_without_lane_matchups() {
    for queue in [430, 480] {
        let mut game = ranked_game(&format!("EUW1_mirror_{queue}"), queue);
        game.detail["info"]["participants"][5]["championId"] = json!(1);
        for (index, item) in [(0, 1001), (5, 3006)] {
            for slot in 0..6 {
                game.detail["info"]["participants"][index][format!("item{slot}")] =
                    json!(if slot == 0 { item } else { 0 });
            }
        }
        let mut acc = Accumulator::new(1).unwrap();
        acc.add(&game);
        let report = acc.finish();
        assert_eq!((report.source_matches, report.included_matches), (1, 1));
        assert!(report.exclusions.is_empty());
        let champion = report
            .groups
            .iter()
            .find(|g| {
                g.key.queue_id == queue
                    && g.key.champion_id == 1
                    && g.key.role == Role::Top
                    && g.key.rank == "ALL"
            })
            .unwrap();
        assert_eq!((champion.games, champion.wins, champion.losses), (2, 1, 1));
        assert_eq!(champion.bucket_matches, 1);
        assert_eq!(
            (champion.win_rate, champion.pick_rate),
            (Some(50.0), Some(100.0))
        );
        let mut builds: Vec<_> = report
            .builds
            .iter()
            .filter(|b| {
                b.key.queue_id == queue
                    && b.key.champion_id == 1
                    && b.key.role == Role::Top
                    && b.key.rank == "ALL"
                    && b.category == "final_items"
            })
            .map(|b| (b.selection.clone(), b.games, b.wins, b.population))
            .collect();
        builds.sort();
        assert_eq!(
            builds,
            vec![(vec![1001], 1, Some(1), 2), (vec![3006], 1, Some(0), 2)]
        );
        assert!(
            report.matchups.is_empty(),
            "aucun appariement en file {queue}"
        );
        assert_eq!(report.coverage[0].counts.participations, 10);
        assert_eq!(report.coverage[0].counts.lane_matchup_participations, 0);
    }
}

#[test]
fn une_couverture_publiee_avant_les_matchups_reste_lisible() {
    let mut coverage = serde_json::to_value(crate::aggregation::Coverage::default()).unwrap();
    coverage
        .as_object_mut()
        .unwrap()
        .remove("lane_matchup_participations")
        .unwrap();
    let coverage: crate::aggregation::Coverage = serde_json::from_value(coverage).unwrap();
    assert_eq!(coverage.lane_matchup_participations, 0);
    let legacy = json!({"patch":"15.19","platform_id":"EUW1","queue_id":420,"role":"TOP",
        "rank":"ALL","champion_id":1,"opponent_champion_id":6,"games":3,"wins":2,"losses":1,
        "win_rate":null,"win_rate_lower_bound":null});
    let parsed: MatchupStats = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), legacy);
}
