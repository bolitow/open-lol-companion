use super::*;
use crate::aggregation::model::{Accumulator, ObservedRank, StoredMatch};
use crate::model::fixtures::match_detail;
use serde_json::json;

fn end_of_game(kills: u64, deaths: u64, assists: u64) -> Value {
    json!({
        "kills": kills, "deaths": deaths, "assists": assists,
        "totalDamageDealtToChampions": 20_000, "totalMinionsKilled": 200,
        "neutralMinionsKilled": 10, "goldEarned": 12_000, "visionScore": 30,
        "puuid": "identifiant-secret-synthetique"
    })
}

fn frame(timestamp: u64, gold: u64, minions: u64, jungle: u64, xp: u64) -> Value {
    json!({
        "timestamp": timestamp,
        "events": [],
        "participantFrames": {
            "1": {"participantId": 1, "totalGold": gold, "minionsKilled": minions,
                  "jungleMinionsKilled": jungle, "xp": xp, "currentGold": 9999},
            "2": {"participantId": 2, "totalGold": 1, "minionsKilled": 1,
                  "jungleMinionsKilled": 1, "xp": 1}
        }
    })
}

fn timeline(match_id: &str, frames: Vec<Value>) -> Value {
    json!({
        "metadata": {"matchId": match_id},
        "info": {
            "frameInterval": 60_000,
            "participants": (1..=10).map(|id| json!({"participantId": id})).collect::<Vec<_>>(),
            "frames": frames
        }
    })
}

#[test]
fn extrait_les_valeurs_de_fin_de_partie_sans_deviner_un_champ_absent() {
    let observed = extract_end_of_game(&end_of_game(4, 2, 6), 1800).unwrap();
    assert_eq!(
        observed,
        EndOfGame {
            kills: 4,
            deaths: 2,
            assists: 6,
            damage_to_champions: 20_000,
            cs: 210,
            gold: 12_000,
            vision_score: 30,
            duration_s: 1800,
        }
    );
    for field in [
        "kills",
        "deaths",
        "assists",
        "totalDamageDealtToChampions",
        "totalMinionsKilled",
        "neutralMinionsKilled",
        "goldEarned",
        "visionScore",
    ] {
        let mut missing = end_of_game(4, 2, 6);
        missing.as_object_mut().unwrap().remove(field);
        assert_eq!(extract_end_of_game(&missing, 1800), None, "{field} absent");
        let mut invalid = end_of_game(4, 2, 6);
        invalid[field] = json!(-1);
        assert_eq!(extract_end_of_game(&invalid, 1800), None, "{field} négatif");
    }
    // Sans durée exploitable, aucune valeur par minute ne peut être calculée.
    assert_eq!(extract_end_of_game(&end_of_game(4, 2, 6), 0), None);
}

#[test]
fn retient_la_premiere_frame_de_la_fenetre_de_chaque_minute() {
    let timeline = timeline(
        "EUW1_1",
        vec![
            frame(0, 500, 0, 0, 0),
            frame(540_120, 3_000, 70, 0, 4_000),
            frame(600_278, 3_609, 82, 4, 4_850),
            frame(620_000, 9_999, 99, 9, 9_999),
            frame(900_365, 5_800, 130, 6, 8_000),
        ],
    );
    let frames = extract_frames(&timeline, 1);
    assert_eq!(
        frames.get(&10),
        Some(&FrameValues {
            gold: 3_609,
            cs: 86,
            xp: 4_850
        })
    );
    assert_eq!(
        frames.get(&15),
        Some(&FrameValues {
            gold: 5_800,
            cs: 136,
            xp: 8_000
        })
    );
    assert_eq!(frames.len(), FRAME_MINUTES.len());
}

#[test]
fn une_partie_terminee_avant_la_minute_ne_publie_pas_de_frame_pour_elle() {
    // Fin à 14:20 : la frame finale n'appartient pas à la fenêtre des 15 minutes.
    let short = timeline(
        "EUW1_1",
        vec![
            frame(600_278, 3_609, 82, 4, 4_850),
            frame(860_000, 5_000, 120, 0, 7_000),
        ],
    );
    let frames = extract_frames(&short, 1);
    assert!(frames.contains_key(&10));
    assert!(!frames.contains_key(&15));
    // Participant absent des frames ou champ manquant : la minute n'est pas comptée.
    assert!(extract_frames(&short, 3).is_empty());
    let mut broken = short.clone();
    broken["info"]["frames"][0]["participantFrames"]["1"]
        .as_object_mut()
        .unwrap()
        .remove("xp");
    assert!(!extract_frames(&broken, 1).contains_key(&10));
}

fn ranked_game(id: &str, kills: u64, deaths: u64, assists: u64) -> StoredMatch {
    let mut detail = match_detail(id, "EUW1", 420, 1_000_000);
    for p in detail["info"]["participants"].as_array_mut().unwrap() {
        let id = p["participantId"].clone();
        let mut values = end_of_game(kills, deaths, assists);
        values.as_object_mut().unwrap().remove("puuid");
        p.as_object_mut()
            .unwrap()
            .extend(values.as_object().unwrap().clone());
        p["participantId"] = id;
    }
    StoredMatch {
        match_id: id.into(),
        platform_id: "EUW1".into(),
        queue_id: 420,
        patch: "15.19".into(),
        is_remake: false,
        game_duration_s: 1800,
        detail,
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

fn top_one<'a>(
    report: &'a crate::aggregation::AggregationReport,
    rank: &str,
) -> &'a PerformanceStats {
    report
        .performance
        .iter()
        .find(|p| p.key.champion_id == 1 && p.key.rank == rank)
        .unwrap()
}

#[test]
fn publie_les_moyennes_par_population_avec_le_kda_des_sommes() {
    let mut acc = Accumulator::new(2).unwrap();
    let mut first = ranked_game("EUW1_1", 4, 0, 6);
    first.timeline = Some(timeline(
        "EUW1_1",
        vec![
            frame(600_278, 3_600, 80, 0, 4_800),
            frame(900_365, 5_800, 130, 0, 8_000),
        ],
    ));
    acc.add(&first);
    let mut second = ranked_game("EUW1_2", 2, 3, 2);
    second.game_duration_s = 1200;
    second.timeline = Some(timeline(
        "EUW1_2",
        vec![frame(600_100, 3_400, 70, 10, 4_600)],
    ));
    acc.add(&second);
    let report = acc.finish();
    assert!(!report.performance_method.is_empty());
    for rank in ["ALL", "GOLD"] {
        let stats = top_one(&report, rank);
        assert_eq!((stats.participations, stats.games), (2, 2));
        // KDA = (ΣK + ΣA) / max(ΣD, 1) = (6 + 8) / 3, pas la moyenne des KDA par partie.
        assert!((stats.kda.unwrap() - 14.0 / 3.0).abs() < 1e-12);
        assert_eq!(stats.kills, Some(3.0));
        assert_eq!(stats.deaths, Some(1.5));
        assert_eq!(stats.assists, Some(4.0));
        assert_eq!(stats.damage_to_champions, Some(20_000.0));
        assert_eq!(stats.vision_score, Some(30.0));
        // Par minute : sommes rapportées à la durée cumulée (30 + 20 minutes).
        assert!((stats.cs_per_min.unwrap() - 420.0 / 50.0).abs() < 1e-12);
        assert!((stats.gold_per_min.unwrap() - 24_000.0 / 50.0).abs() < 1e-12);
        let at = |minute: u32| stats.frames.iter().find(|f| f.minute == minute).unwrap();
        assert_eq!(at(10).games, 2);
        assert_eq!(at(10).gold, Some(3_500.0));
        assert_eq!(at(10).cs, Some(80.0));
        assert_eq!(at(10).xp, Some(4_700.0));
        // Une seule frame à 15 minutes, sous le seuil de 2 : effectif publié, moyenne nulle.
        assert_eq!(at(15).games, 1);
        assert_eq!((at(15).gold, at(15).cs, at(15).xp), (None, None, None));
    }
    let encoded = serde_json::to_string(&report.performance).unwrap();
    for forbidden in ["puuid", "Joueur", "EUW1_1", "currentGold", "secret"] {
        assert!(!encoded.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn sans_morts_le_kda_divise_par_un_et_sous_le_seuil_rien_n_est_moyenne() {
    let mut acc = Accumulator::new(2).unwrap();
    acc.add(&ranked_game("EUW1_1", 3, 0, 5));
    let report = acc.finish();
    let stats = top_one(&report, "ALL");
    assert_eq!((stats.participations, stats.games), (1, 1));
    assert_eq!(stats.kda, None);
    assert_eq!(stats.cs_per_min, None);
    assert!(stats
        .frames
        .iter()
        .all(|f| f.games == 0 && f.gold.is_none()));
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&ranked_game("EUW1_1", 3, 0, 5));
    assert_eq!(top_one(&acc.finish(), "ALL").kda, Some(8.0));
}

#[test]
fn une_participation_incomplete_reste_dans_le_denominateur_de_couverture() {
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&ranked_game("EUW1_1", 1, 1, 1));
    let mut incomplete = ranked_game("EUW1_2", 9, 9, 9);
    incomplete.detail["info"]["participants"][0]
        .as_object_mut()
        .unwrap()
        .remove("visionScore");
    acc.add(&incomplete);
    let report = acc.finish();
    let stats = top_one(&report, "ALL");
    assert_eq!((stats.participations, stats.games), (2, 1));
    assert_eq!(stats.kills, Some(1.0));
    assert_eq!(
        report.performance.len(),
        report.groups.len(),
        "une ligne par population de la tierlist"
    );
}
