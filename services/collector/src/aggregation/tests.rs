use serde_json::{json, Value};

use super::model::{Accumulator, ObservedRank, StoredMatch};
use super::{
    AggregationError, QualityThresholds, Role, DEFAULT_MIN_GAME_DURATION_S,
    DEFAULT_MIN_PLAYED_PERCENT, DEFAULT_RANK_MAX_AGE_HOURS, MAX_MIN_GAME_DURATION_S,
};
use crate::model::fixtures::match_detail;

fn game(id: &str) -> StoredMatch {
    StoredMatch {
        match_id: id.into(),
        platform_id: "EUW1".into(),
        queue_id: 420,
        patch: "15.19".into(),
        is_remake: false,
        game_duration_s: 1800,
        detail: match_detail(id, "EUW1", 420, 1_000_000),
        timeline: None,
        ranks: Default::default(),
    }
}

fn observed(status: &str, tier: Option<&str>, gap_s: u64) -> ObservedRank {
    ObservedRank {
        status: status.into(),
        tier: tier.map(Into::into),
        gap_s,
    }
}

fn reverse_winner(game: &mut StoredMatch) {
    for p in game.detail["info"]["participants"].as_array_mut().unwrap() {
        p["win"] = json!(!p["win"].as_bool().unwrap());
    }
}

#[test]
fn calcule_deux_victoires_sur_trois_sans_donnees_personnelles() {
    let mut acc = Accumulator::new(3).unwrap();
    acc.add(&game("EUW1_1"));
    acc.add(&game("EUW1_2"));
    let mut lost = game("EUW1_3");
    reverse_winner(&mut lost);
    acc.add(&lost);
    let report = acc.finish();
    assert_eq!((report.source_matches, report.included_matches), (3, 3));
    assert_eq!(report.groups.len(), 20);
    let champion = &report.groups[0];
    assert_eq!(champion.key.champion_id, 1);
    assert_eq!((champion.games, champion.wins, champion.losses), (3, 2, 1));
    assert!((champion.win_rate.unwrap() - 66.66666666666667).abs() < 1e-10);
    assert_eq!(champion.position, Some(1));
    let encoded = serde_json::to_string(&report).unwrap();
    for forbidden in ["puuid", "Joueur", "EUW1_1", "GOLD", "seed"] {
        assert!(!encoded.contains(forbidden));
    }
    assert_eq!(
        report.rank_scope,
        "observed_rank_nearest_to_game_start_of_same_ranked_queue"
    );
    assert_eq!(report.rank_max_age_hours, DEFAULT_RANK_MAX_AGE_HOURS);
}

#[test]
fn le_seuil_masque_le_taux_et_la_position_mais_conserve_les_comptes() {
    for (minimum, eligible) in [(1, true), (2, true), (3, false)] {
        let mut acc = Accumulator::new(minimum).unwrap();
        acc.add(&game("EUW1_1"));
        acc.add(&game("EUW1_2"));
        let report = acc.finish();
        assert!(report.groups.iter().all(|g| g.games == 2));
        assert!(report
            .groups
            .iter()
            .all(|g| g.win_rate.is_some() == eligible));
        assert!(report
            .groups
            .iter()
            .all(|g| g.position.is_some() == eligible));
    }
}

#[test]
fn refuse_le_seuil_nul_et_produit_un_bilan_vide_serialisable() {
    assert!(matches!(
        Accumulator::new(0),
        Err(AggregationError::InvalidThreshold)
    ));
    let empty = Accumulator::new(100).unwrap().finish();
    assert_eq!(empty.included_matches, 0);
    assert!(empty.groups.is_empty());
    assert!(serde_json::to_string(&empty).is_ok());
}

#[test]
fn separe_les_patchs_et_les_roles_du_meme_champion() {
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&game("EUW1_1"));
    let mut next = game("EUW1_2");
    next.patch = "15.20".into();
    next.detail["info"]["gameVersion"] = json!("15.20.1.2");
    acc.add(&next);
    let mut swapped = game("EUW1_3");
    swapped.detail["info"]["participants"][0]["teamPosition"] = json!("JUNGLE");
    swapped.detail["info"]["participants"][1]["teamPosition"] = json!("TOP");
    acc.add(&swapped);
    let report = acc.finish();
    let groups: Vec<_> = report
        .groups
        .iter()
        .filter(|g| g.key.champion_id == 1 && g.key.rank == "ALL")
        .collect();
    assert_eq!(groups.len(), 3);
    assert!(groups.iter().all(|g| g.games == 1));
    assert!(groups.iter().any(|g| g.key.role == Role::Jungle));
}

#[test]
fn exclut_remakes_et_perimetres_etrangers() {
    let mut acc = Accumulator::new(1).unwrap();
    let mut remake = game("EUW1_1");
    remake.is_remake = true;
    acc.add(&remake);
    let mut raw_remake = game("EUW1_2");
    raw_remake.detail["info"]["participants"][0]["gameEndedInEarlySurrender"] = json!(true);
    acc.add(&raw_remake);
    let mut flex = game("EUW1_3");
    flex.queue_id = 440;
    acc.add(&flex);
    let mut other = game("EUN1_4");
    other.platform_id = "EUN1".into();
    acc.add(&other);
    let report = acc.finish();
    assert_eq!(report.included_matches, 0);
    assert_eq!(report.source_matches, 4);
    assert_eq!(report.exclusions.get("remake"), Some(&2));
    assert_eq!(report.exclusions.get("invalid_match"), Some(&2));
}

#[test]
fn rejette_toute_la_partie_si_un_participant_est_invalide() {
    let variants = [
        ("championId", json!(0)),
        ("championId", json!(-1)),
        ("championId", json!(2)),
        ("win", Value::Null),
        ("win", json!("true")),
        ("win", json!(false)),
        ("teamPosition", json!("JUNGLE")),
        ("teamId", json!(300)),
        ("gameEndedInEarlySurrender", json!("false")),
    ];
    for (field, value) in variants {
        let mut invalid = game("EUW1_1");
        invalid.detail["info"]["participants"][0][field] = value;
        let mut acc = Accumulator::new(1).unwrap();
        acc.add(&invalid);
        let report = acc.finish();
        assert_eq!(report.included_matches, 0, "{field}");
        assert!(
            report.groups.is_empty(),
            "aucune contribution partielle : {field}"
        );
        assert_eq!(report.exclusions.get("invalid_match"), Some(&1));
    }
}

#[test]
fn refuse_les_metadonnees_incoherentes_et_les_participants_manquants() {
    for path in [
        "/metadata/matchId",
        "/info/gameVersion",
        "/info/platformId",
        "/info/queueId",
        "/info/participants",
    ] {
        let mut invalid = game("EUW1_1");
        *invalid.detail.pointer_mut(path).unwrap() = Value::Null;
        let mut acc = Accumulator::new(1).unwrap();
        acc.add(&invalid);
        assert_eq!(
            acc.finish().exclusions.get("invalid_match"),
            Some(&1),
            "{path}"
        );
    }
    let mut mismatch = game("EUW1_1");
    mismatch.patch = "15.20".into();
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&mismatch);
    assert_eq!(acc.finish().included_matches, 0);
}

#[test]
fn classe_par_taux_puis_effectif_puis_identifiant_independamment_de_l_ordre() {
    let mut third = game("EUW1_3");
    third.detail["info"]["participants"][0]["championId"] = json!(20);
    third.detail["info"]["participants"][5]["championId"] = json!(21);
    let games = [game("EUW1_1"), game("EUW1_2"), third];
    let mut acc = Accumulator::new(1).unwrap();
    let mut reversed = Accumulator::new(1).unwrap();
    for g in &games {
        acc.add(g);
    }
    for g in games.iter().rev() {
        reversed.add(g);
    }
    let report = acc.finish();
    assert_eq!(report, reversed.finish());
    let top: Vec<_> = report
        .groups
        .iter()
        .filter(|g| g.key.role == Role::Top && g.key.rank == "ALL")
        .collect();
    // Score #85 : 6 (0/2) précède 21 (0/1) grâce à sa présence (pick rate 66,7 % contre
    // 33,3 %), pas au départage par effectif.
    assert_eq!(
        top.iter()
            .map(|g| (g.key.champion_id, g.position))
            .collect::<Vec<_>>(),
        vec![(1, Some(1)), (20, Some(2)), (6, Some(3)), (21, Some(4))]
    );
}

#[test]
fn les_groupes_sous_le_seuil_restent_apres_les_ex_aequo_classes() {
    let mut acc = Accumulator::new(2).unwrap();
    acc.add(&game("EUW1_1"));
    let mut lost = game("EUW1_2");
    reverse_winner(&mut lost);
    acc.add(&lost);
    let mut small = game("EUW1_3");
    small.detail["info"]["participants"][0]["championId"] = json!(20);
    small.detail["info"]["participants"][5]["championId"] = json!(21);
    acc.add(&small);
    let report = acc.finish();
    let top: Vec<_> = report
        .groups
        .iter()
        .filter(|g| g.key.role == Role::Top && g.key.rank == "ALL")
        .collect();
    // 1 et 6 ont chacun une victoire sur deux : ordre par identifiant.
    // 20 a 100 % sur une seule partie et reste hors classement.
    assert_eq!(
        top.iter()
            .map(|g| (g.key.champion_id, g.position, g.win_rate))
            .collect::<Vec<_>>(),
        vec![
            (1, Some(1), Some(50.0)),
            (6, Some(2), Some(50.0)),
            (20, None, None),
            (21, None, None)
        ]
    );
}

#[test]
fn calcule_pickrate_et_bans_sur_des_populations_explicites() {
    let mut acc = Accumulator::new(1).unwrap();
    let mut g = game("EUW1_extended");
    g.detail["info"]["teams"] = json!([
        {"teamId":100,"bans":(1..=5).map(|turn|json!({"championId":if turn==1 {99}else{-1},"pickTurn":turn})).collect::<Vec<_>>()},
        {"teamId":200,"bans":(6..=10).map(|turn|json!({"championId":if turn==6 {99}else{-1},"pickTurn":turn})).collect::<Vec<_>>()}
    ]);
    acc.add(&g);
    let report = serde_json::to_value(acc.finish()).unwrap();
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["groups"][0]["population"], 2);
    assert_eq!(report["groups"][0]["bucket_matches"], 1);
    assert_eq!(report["groups"][0]["pick_rate"], 100.0);
    assert_eq!(report["groups"][0]["selection_share"], 50.0);
    assert_eq!(report["bans"][0]["banned_matches"], 1);
    assert_eq!(report["bans"][0]["draft_matches"], 1);
    assert_eq!(report["bans"][0]["ban_rate"], 100.0);
}

fn find_group<'a>(
    report: &'a super::AggregationReport,
    champion_id: u32,
    role: Role,
    rank: &str,
) -> &'a super::ChampionStats {
    report
        .groups
        .iter()
        .find(|g| g.key.champion_id == champion_id && g.key.role == role && g.key.rank == rank)
        .unwrap()
}

#[test]
fn le_pickrate_compte_les_parties_ou_le_champion_apparait_pas_les_participations() {
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&game("EUW1_1"));
    // Le champion 1 (TOP) laisse sa place au champion 99 dans la seconde partie.
    let mut other = game("EUW1_2");
    other.detail["info"]["participants"][0]["championId"] = json!(99);
    acc.add(&other);
    let report = acc.finish();
    let one = find_group(&report, 1, Role::Top, "ALL");
    assert_eq!((one.games, one.population, one.bucket_matches), (1, 4, 2));
    assert_eq!(one.pick_rate, Some(50.0));
    // Part des sélections : 1 participation sur les 4 du compartiment TOP.
    assert_eq!(one.selection_share, Some(25.0));
    let other_top = find_group(&report, 99, Role::Top, "ALL");
    assert_eq!(other_top.pick_rate, Some(50.0));
    // Champion présent dans toutes les parties : 100 %, contre 50 % par participation.
    let ally = find_group(&report, 6, Role::Top, "ALL");
    assert_eq!(
        (ally.pick_rate, ally.selection_share),
        (Some(100.0), Some(50.0))
    );
}

#[test]
fn le_pickrate_par_partie_est_comparable_au_ban_rate_dans_un_mode_sans_role() {
    let mut acc = Accumulator::new(1).unwrap();
    for id in ["EUW1_a", "EUW1_b"] {
        let mut aram = game(id);
        aram.queue_id = 450;
        aram.detail["info"]["queueId"] = json!(450);
        for p in aram.detail["info"]["participants"].as_array_mut().unwrap() {
            p["teamPosition"] = json!("");
        }
        if id == "EUW1_b" {
            aram.detail["info"]["participants"][0]["championId"] = json!(99);
        }
        acc.add(&aram);
    }
    let report = acc.finish();
    let one = find_group(&report, 1, Role::Unknown, "ALL");
    // 10 participations par partie : l'ancienne part valait 1/20 = 5 %.
    assert_eq!((one.bucket_matches, one.population), (2, 20));
    assert_eq!(one.pick_rate, Some(50.0));
    assert_eq!(one.selection_share, Some(5.0));
    let always = find_group(&report, 2, Role::Unknown, "ALL");
    assert_eq!(always.pick_rate, Some(100.0));
    assert_eq!(always.selection_share, Some(10.0));
}

#[test]
fn un_champion_en_double_dans_une_partie_ne_depasse_pas_cent_pour_cent() {
    let mut acc = Accumulator::new(1).unwrap();
    let mut urf = game("EUW1_dup");
    urf.queue_id = 1020;
    urf.detail["info"]["queueId"] = json!(1020);
    for p in urf.detail["info"]["participants"].as_array_mut().unwrap() {
        p["teamPosition"] = json!("");
    }
    urf.detail["info"]["participants"][1]["championId"] = json!(1);
    acc.add(&urf);
    let report = acc.finish();
    let doubled = find_group(&report, 1, Role::Unknown, "ALL");
    assert_eq!((doubled.games, doubled.bucket_matches), (2, 1));
    assert_eq!(doubled.pick_rate, Some(100.0));
    assert_eq!(doubled.selection_share, Some(20.0));
}

#[test]
fn le_denominateur_du_pickrate_est_propre_au_rang_observe() {
    let mut a = game("EUW1_r1");
    a.ranks.insert(
        "fake-puuid-0".into(),
        observed("ranked", Some("GOLD"), 3600),
    );
    let mut b = game("EUW1_r2");
    // Seule la seconde partie a un top GOLD, sous un autre champion.
    b.detail["info"]["participants"][0]["championId"] = json!(99);
    b.ranks.insert(
        "fake-puuid-0".into(),
        observed("ranked", Some("GOLD"), 3600),
    );
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&a);
    acc.add(&b);
    let report = acc.finish();
    let gold = find_group(&report, 1, Role::Top, "GOLD");
    assert_eq!(
        (gold.bucket_matches, gold.population, gold.pick_rate),
        (2, 2, Some(50.0))
    );
    assert_eq!(gold.selection_share, Some(50.0));
}

#[test]
fn le_pickrate_reste_masque_sous_le_seuil_et_lit_les_anciens_instantanes() {
    let mut acc = Accumulator::new(3).unwrap();
    acc.add(&game("EUW1_1"));
    acc.add(&game("EUW1_2"));
    let report = acc.finish();
    assert!(report
        .groups
        .iter()
        .all(|g| g.pick_rate.is_none() && g.selection_share.is_none()));
    assert!(report.groups.iter().all(|g| g.bucket_matches == 2));
    assert_eq!(
        report.pick_rate_definition,
        "champion_matches / bucket_matches * 100"
    );
    // Instantané publié avant #84 : les champs absents prennent leur valeur neutre.
    let mut old = serde_json::to_value(&report.groups[0]).unwrap();
    old.as_object_mut().unwrap().remove("bucket_matches");
    old.as_object_mut().unwrap().remove("selection_share");
    let read: super::ChampionStats = serde_json::from_value(old).unwrap();
    assert_eq!((read.bucket_matches, read.selection_share), (0, None));
}

#[test]
fn accepte_les_autres_regions_et_les_modes_sans_roles_fixes() {
    let mut acc = Accumulator::new(1).unwrap();
    let mut g = game("NA1_aram");
    g.platform_id = "NA1".into();
    g.queue_id = 450;
    g.detail["info"]["platformId"] = json!("NA1");
    g.detail["info"]["queueId"] = json!(450);
    for p in g.detail["info"]["participants"].as_array_mut().unwrap() {
        p["teamPosition"] = json!("");
    }
    acc.add(&g);
    assert_eq!(acc.finish().included_matches, 1);
}

#[test]
fn les_bans_incomplets_ne_deviennent_pas_un_denominateur() {
    let mut acc = Accumulator::new(1).unwrap();
    let mut g = game("EUW1_bans");
    g.detail["info"]["teams"] = json!([
        {"teamId":100,"bans":[{"championId":99,"pickTurn":1}]},
        {"teamId":200,"bans":[]}
    ]);
    acc.add(&g);
    let report = acc.finish();
    assert!(report.bans.is_empty());
    assert_eq!(report.coverage[0].counts.draft_matches, 0);
}

#[test]
fn les_rangs_individuels_ne_se_propagant_pas_aux_autres_participants() {
    let mut g = game("EUW1_ranks");
    g.ranks.insert(
        "fake-puuid-0".into(),
        observed("ranked", Some("GOLD"), 3600),
    );
    g.ranks
        .insert("fake-puuid-5".into(), observed("unranked", None, 3600));
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&g);
    let r = acc.finish();
    let gold: Vec<_> = r.groups.iter().filter(|g| g.key.rank == "GOLD").collect();
    assert_eq!(gold.len(), 1);
    assert_eq!(
        (gold[0].key.champion_id, gold[0].games, gold[0].population),
        (1, 1, 1)
    );
    assert_eq!(gold[0].pick_rate, Some(100.0));
    assert_eq!(r.coverage[0].counts.unknown_rank_participations, 8);
    assert_eq!(r.coverage[0].counts.unranked_participations, 1);
    assert_eq!(
        r.groups
            .iter()
            .find(|g| g.key.rank == "ALL" && g.key.champion_id == 1)
            .unwrap()
            .most_picked_rank
            .as_deref(),
        Some("GOLD")
    );
}

#[test]
fn conserve_un_role_inconnu_sans_le_deviner() {
    let mut g = game("EUW1_unknown");
    g.detail["info"]["participants"][0]["teamPosition"] = json!("");
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&g);
    let r = acc.finish();
    assert_eq!(r.included_matches, 1);
    assert_eq!(r.coverage[0].counts.unknown_role_participations, 1);
    assert!(r
        .groups
        .iter()
        .any(|g| g.key.champion_id == 1 && g.key.role == Role::Unknown));
}

#[test]
fn arena_utilise_les_sous_equipes_et_ne_deduit_pas_de_role() {
    let mut g = game("EUW1_arena");
    g.queue_id = 1700;
    g.detail["info"]["queueId"] = json!(1700);
    g.detail["metadata"]["participants"] = json!((0..16)
        .map(|i| format!("synthetic-{i}"))
        .collect::<Vec<_>>());
    g.detail["info"]["participants"] =
        json!((0..16).map(|i|json!({
        "participantId":i+1,"teamId":100,"playerSubteamId":1+i/2,"championId":1+i%4,"win":i<8
    })).collect::<Vec<_>>());
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&g);
    let r = acc.finish();
    assert_eq!(r.included_matches, 1);
    assert_eq!(r.coverage[0].counts.participations, 16);
    assert!(r
        .groups
        .iter()
        .all(|g| g.key.role == Role::Unknown && g.games == 4 && g.population == 16));
}

/// Partie dont les TOP bleu (gagnant) et rouge sont remplacés ; bans facultatifs (équipe bleue).
fn top_duel(id: &str, blue: u32, red: u32, bans: &[u32]) -> StoredMatch {
    let mut g = game(id);
    g.detail["info"]["participants"][0]["championId"] = json!(blue);
    g.detail["info"]["participants"][5]["championId"] = json!(red);
    let slot = |turn: usize| json!({"championId": bans.get(turn - 1).map_or(-1, |c| i64::from(*c)), "pickTurn": turn});
    g.detail["info"]["teams"] = json!([
        {"teamId": 100, "bans": (1..=5).map(slot).collect::<Vec<_>>()},
        {"teamId": 200, "bans": (6..=10).map(slot).collect::<Vec<_>>()}
    ]);
    g
}

fn top_all(report: &super::AggregationReport) -> Vec<&super::ChampionStats> {
    report
        .groups
        .iter()
        .filter(|g| g.key.rank == "ALL" && g.key.role == Role::Top)
        .collect()
}

/// Dix parties, vingt champions TOP distincts (100..110 gagnants, 200..210 perdants).
fn twenty_top_champions(acc: &mut Accumulator, games: u32) {
    for n in 0..games {
        acc.add(&top_duel(&format!("EUW1_tier{n}"), 100 + n, 200 + n, &[]));
    }
}

#[test]
fn des_champions_proches_de_50_pourcent_restent_tous_b_sans_repartition_forcee() {
    let mut acc = Accumulator::new(1).unwrap();
    twenty_top_champions(&mut acc, 10);
    let r = acc.finish();
    let top = top_all(&r);
    assert_eq!(top.len(), 20);
    assert!(top.iter().all(|g| g.tier.as_deref() == Some("B")));
    assert!(top.iter().all(|g| g.win_rate_lower_bound.is_some()));
    // Les gagnants (1 sur 1) précèdent les perdants, sans lettre S ni D imposée.
    assert!(top[..10].iter().all(|g| g.wins == 1));
    assert_eq!(top[0].position, Some(1));
    // JUNGLE : deux champions seulement, aucun tier mais une position.
    assert!(r
        .groups
        .iter()
        .filter(|g| g.key.role == Role::Jungle)
        .all(|g| g.tier.is_none() && g.position.is_some()));
}

#[test]
fn sous_vingt_champions_eligibles_aucun_tier_n_est_attribue() {
    let mut acc = Accumulator::new(1).unwrap();
    twenty_top_champions(&mut acc, 9);
    let r = acc.finish();
    let top = top_all(&r);
    assert_eq!(top.len(), 18);
    assert!(top.iter().all(|g| g.tier.is_none() && g.position.is_some()));
}

#[test]
fn un_champion_dominant_est_s_son_adversaire_d_et_le_reste_b() {
    let mut games: Vec<_> = (0..10)
        .map(|n| top_duel(&format!("EUW1_tier{n}"), 100 + n, 200 + n, &[]))
        .collect();
    games.extend((0..60).map(|n| top_duel(&format!("EUW1_duel{n}"), 500, 501, &[])));
    let mut acc = Accumulator::new(1).unwrap();
    let mut reversed = Accumulator::new(1).unwrap();
    for g in &games {
        acc.add(g);
    }
    for g in games.iter().rev() {
        reversed.add(g);
    }
    let r = acc.finish();
    assert_eq!(r, reversed.finish());
    let top = top_all(&r);
    assert_eq!(top.len(), 22);
    assert_eq!(
        (
            top[0].key.champion_id,
            top[0].tier.as_deref(),
            top[0].position
        ),
        (500, Some("S"), Some(1))
    );
    let last = top.last().unwrap();
    assert_eq!(
        (last.key.champion_id, last.tier.as_deref(), last.position),
        (501, Some("D"), Some(22))
    );
    assert!(top[1..21].iter().all(|g| g.tier.as_deref() == Some("B")));
}

#[test]
fn un_pick_rate_sous_le_seuil_garde_sa_position_sans_tier() {
    let mut acc = Accumulator::new(1).unwrap();
    for n in 0..200 {
        acc.add(&top_duel(
            &format!("EUW1_tier{n}"),
            100 + n % 10,
            110 + n % 10,
            &[],
        ));
    }
    // 1 partie sur 201 : pick rate de 0,497 %, sous le seuil de 0,5 %.
    acc.add(&top_duel("EUW1_rare", 999, 110, &[]));
    let r = acc.finish();
    let top = top_all(&r);
    let rare = top.iter().find(|g| g.key.champion_id == 999).unwrap();
    assert!(rare.pick_rate.unwrap() < 0.5);
    assert!(rare.position.is_some() && rare.tier.is_none());
    assert!(top
        .iter()
        .filter(|g| g.key.champion_id != 999)
        .all(|g| g.tier.is_some()));
}

#[test]
fn un_champion_sous_le_seuil_de_pick_rate_ne_compte_pas_parmi_les_vingt_eligibles() {
    let mut acc = Accumulator::new(1).unwrap();
    // 19 champions réguliers : 100 à 109 en bleu, 110 à 118 en rouge.
    for n in 0..200 {
        acc.add(&top_duel(
            &format!("EUW1_tier{n}"),
            100 + n % 10,
            110 + n % 9,
            &[],
        ));
    }
    // Vingtième champion avec winrate, mais 1 partie sur 201 : pick rate sous 0,5 %.
    acc.add(&top_duel("EUW1_rare", 999, 110, &[]));
    let r = acc.finish();
    let top = top_all(&r);
    assert_eq!(top.iter().filter(|g| g.win_rate.is_some()).count(), 20);
    assert!(top.iter().all(|g| g.position.is_some() && g.tier.is_none()));
}

#[test]
fn le_ban_rate_de_all_entre_dans_le_score_mais_pas_pour_un_rang_unknown() {
    let mut acc = Accumulator::new(1).unwrap();
    // 100 et 101 gagnent chacun leur unique partie ; 100 est banni dans les neuf autres.
    for n in 0..10 {
        let bans = if n == 0 { vec![] } else { vec![100] };
        acc.add(&top_duel(&format!("EUW1_tier{n}"), 100 + n, 200 + n, &bans));
    }
    let r = acc.finish();
    let banned = find_group(&r, 100, Role::Top, "ALL");
    let twin = find_group(&r, 101, Role::Top, "ALL");
    assert_eq!((banned.wins, twin.wins), (1, 1));
    // Présence : 10 % de pick + 90 % de ban → +2 points, contre +0,2 pour 101.
    assert_eq!(
        (banned.tier.as_deref(), banned.position),
        (Some("A"), Some(1))
    );
    assert_eq!(twin.tier.as_deref(), Some("B"));
    // Rang de joueur UNKNOWN : sans équivalent au palier de partie, le ban n'est pas compté.
    let unknown = find_group(&r, 100, Role::Top, "UNKNOWN");
    assert_eq!(unknown.tier.as_deref(), Some("B"));
}

/// Partie `top_duel` dont les dix joueurs ont un rang observé `tier` proche du début.
fn ranked_top_duel(id: &str, blue: u32, red: u32, bans: &[u32], tier: &str) -> StoredMatch {
    let mut g = top_duel(id, blue, red, bans);
    rank_first_players(&mut g, 10, tier, 3600);
    g
}

#[test]
fn un_palier_classe_utilise_le_ban_rate_de_son_palier_et_non_celui_de_all() {
    let mut acc = Accumulator::new(1).unwrap();
    // Dix parties GOLD, vingt champions TOP : 100 y est banni neuf fois, 101 jamais.
    for n in 0..10 {
        let bans = if n == 0 { vec![] } else { vec![100] };
        acc.add(&ranked_top_duel(
            &format!("EUW1_gold{n}"),
            100 + n,
            200 + n,
            &bans,
            "GOLD",
        ));
    }
    // Trente parties DIAMOND sans 100 ni 101 en jeu : 101 y est toujours banni.
    for n in 0..30 {
        acc.add(&ranked_top_duel(
            &format!("EUW1_diamond{n}"),
            300 + n,
            400 + n,
            &[101],
            "DIAMOND",
        ));
    }
    let r = acc.finish();
    let ban = |rank: &str, champion: u32| find_ban(&r, rank, champion).and_then(|b| b.ban_rate);
    // Bans de GOLD et de ALL volontairement opposés pour 100 et 101.
    assert_eq!(
        (ban("GOLD", 100), ban("ALL", 100)),
        (Some(90.0), Some(22.5))
    );
    assert_eq!((ban("GOLD", 101), ban("ALL", 101)), (None, Some(75.0)));
    let gold: Vec<_> = r
        .groups
        .iter()
        .filter(|g| g.key.rank == "GOLD" && g.key.role == Role::Top)
        .collect();
    assert_eq!(gold.len(), 20);
    assert!(gold.iter().all(|g| g.pick_rate == Some(10.0)));
    // GOLD, ban GOLD : 100 → 0,25 + 0,02 × (10 + 90) = 2,25 (A) ; 101 → 0,45 (B). Avec le ban
    // de ALL, 100 tomberait à 0,90 (B) et 101 monterait à 1,95 (A) ; sans ban, 100 serait B.
    let gold_100 = find_group(&r, 100, Role::Top, "GOLD");
    let gold_101 = find_group(&r, 101, Role::Top, "GOLD");
    assert_eq!(
        (gold_100.tier.as_deref(), gold_100.position),
        (Some("A"), Some(1))
    );
    assert_eq!(gold_101.tier.as_deref(), Some("B"));
    assert!(gold
        .iter()
        .filter(|g| g.key.champion_id != 100)
        .all(|g| g.tier.as_deref() == Some("B")));
    // ALL, ban ALL (40 drafts) : l'ordre s'inverse, 101 → 1,80 (A) et 100 → 0,75 (B).
    let all_100 = find_group(&r, 100, Role::Top, "ALL");
    let all_101 = find_group(&r, 101, Role::Top, "ALL");
    assert_eq!(
        (all_101.tier.as_deref(), all_101.position),
        (Some("A"), Some(1))
    );
    assert_eq!(all_100.tier.as_deref(), Some("B"));
}

#[test]
fn une_file_non_classee_compte_le_ban_rate_de_unranked_mode() {
    let mut acc = Accumulator::new(1).unwrap();
    for n in 0..10 {
        let bans = if n == 0 { vec![] } else { vec![100] };
        let mut g = top_duel(&format!("EUW1_normal{n}"), 100 + n, 200 + n, &bans);
        g.queue_id = 400;
        g.detail["info"]["queueId"] = json!(400);
        acc.add(&g);
    }
    let r = acc.finish();
    assert_eq!(
        find_ban(&r, "UNRANKED_MODE", 100).and_then(|b| b.ban_rate),
        Some(90.0)
    );
    let banned = find_group(&r, 100, Role::Top, "UNRANKED_MODE");
    let twin = find_group(&r, 101, Role::Top, "UNRANKED_MODE");
    // 0,25 + 0,02 × (10 + 90) = 2,25 (A) ; sans le ban, 0,45 (B) comme 101.
    assert_eq!(
        (banned.tier.as_deref(), banned.position),
        (Some("A"), Some(1))
    );
    assert_eq!(twin.tier.as_deref(), Some("B"));
}

fn top_rank<'a>(report: &'a super::AggregationReport, rank: &str) -> Vec<&'a super::ChampionStats> {
    report
        .groups
        .iter()
        .filter(|g| g.key.rank == rank && g.key.role == Role::Top)
        .collect()
}

/// Winrate moyen (%) d'un compartiment, recalculé depuis les comptes publiés.
fn bucket_mean(groups: &[&super::ChampionStats]) -> f64 {
    let wins: u64 = groups.iter().map(|g| g.wins).sum();
    let games: u64 = groups.iter().map(|g| g.games).sum();
    100.0 * wins as f64 / games as f64
}

/// Agrège les parties dans l'ordre puis à rebours et vérifie que le bilan est identique.
fn finish_in_both_orders(games: &[StoredMatch]) -> super::AggregationReport {
    let mut acc = Accumulator::new(1).unwrap();
    let mut reversed = Accumulator::new(1).unwrap();
    for g in games {
        acc.add(g);
    }
    for g in games.iter().rev() {
        reversed.add(g);
    }
    let report = acc.finish();
    assert_eq!(report, reversed.finish());
    report
}

/// Partie `top_duel` où seuls les cinq joueurs bleus ont un rang GOLD : le compartiment
/// GOLD TOP ne contient que des TOP bleus, dont le winrate moyen n'est plus forcé à 50 %.
fn gold_blue_top(id: &str, blue: u32, red: u32, blue_wins: bool) -> StoredMatch {
    let mut g = top_duel(id, blue, red, &[]);
    rank_first_players(&mut g, 5, "GOLD", 3600);
    if !blue_wins {
        reverse_winner(&mut g);
    }
    g
}

#[test]
fn le_score_part_du_winrate_moyen_du_compartiment_et_non_de_50_pourcent() {
    // (champion bleu GOLD, parties, victoires) : 300 victoires sur 500, μ = 60 %.
    let mut plan = vec![(500, 30, 18), (600, 100, 55), (700, 100, 65)];
    plan.extend((0..18).map(|n| (100 + n, 15, 9)));
    let mut games = Vec::new();
    for (champion, played, won) in plan {
        for n in 0..played {
            games.push(gold_blue_top(
                &format!("EUW1_g{champion}_{n}"),
                champion,
                900 + n % 20,
                n < won,
            ));
        }
    }
    let r = finish_in_both_orders(&games);
    let gold = top_rank(&r, "GOLD");
    assert_eq!(gold.len(), 21);
    assert_eq!(gold.iter().filter(|g| g.tier.is_some()).count(), 21);
    assert!((bucket_mean(&gold) - 60.0).abs() < 1e-9);
    let at_mean = find_group(&r, 500, Role::Top, "GOLD");
    assert!((at_mean.win_rate.unwrap() - 60.0).abs() < 1e-9);
    assert!((at_mean.pick_rate.unwrap() - 6.0).abs() < 1e-9);
    // μ = 60 : 500 → 0 + 0,02 × 6 = 0,12 (B), 700 (65 %) → 2,07 (A), 600 (55 %) → −1,27 (C).
    // Avec une base fixe de 50 : 500 → 1,42 (A), 700 → 5,40 (S), 600 → 2,07 (A).
    let tier = |champion: u32| {
        let g = find_group(&r, champion, Role::Top, "GOLD");
        (g.tier.as_deref(), g.position)
    };
    assert_eq!(tier(500), (Some("B"), Some(2)));
    assert_eq!(tier(700), (Some("A"), Some(1)));
    assert_eq!(tier(600), (Some("C"), Some(21)));
    assert!((100..118).all(|c| tier(c).0 == Some("B")));
    // ALL TOP : un gagnant et un perdant par partie, μ = 50 ; 600 y est donc A.
    assert!((bucket_mean(&top_all(&r)) - 50.0).abs() < 1e-9);
    assert_eq!(
        find_group(&r, 600, Role::Top, "ALL").tier.as_deref(),
        Some("A")
    );
}

/// Partie `top_duel` entre un TOP bleu GOLD et un TOP rouge DIAMOND (dix rangs observés).
fn gold_vs_diamond(id: &str, gold: u32, diamond: u32, gold_wins: bool) -> StoredMatch {
    let mut g = top_duel(id, gold, diamond, &[]);
    for i in 0..10 {
        let tier = if i < 5 { "GOLD" } else { "DIAMOND" };
        g.ranks.insert(
            format!("fake-puuid-{i}"),
            observed("ranked", Some(tier), 3600),
        );
    }
    if !gold_wins {
        reverse_winner(&mut g);
    }
    g
}

#[test]
fn un_meme_winrate_change_de_lettre_selon_la_moyenne_de_son_palier() {
    let mut games = Vec::new();
    // Vingt duels GOLD contre DIAMOND de dix parties, gagnés sept fois par GOLD.
    for pair in 0..20 {
        for n in 0..10 {
            games.push(gold_vs_diamond(
                &format!("EUW1_pair{pair}_{n}"),
                100 + pair,
                200 + pair,
                n < 7,
            ));
        }
    }
    // 500 joue 100 parties en GOLD et 100 en DIAMOND, avec 50 % de victoires de chaque côté.
    for n in 0..100 {
        games.push(gold_vs_diamond(
            &format!("EUW1_g500_{n}"),
            500,
            200 + n % 20,
            n % 2 == 0,
        ));
        games.push(gold_vs_diamond(
            &format!("EUW1_d500_{n}"),
            100 + n % 20,
            500,
            n % 2 == 1,
        ));
    }
    let r = finish_in_both_orders(&games);
    let gold = top_rank(&r, "GOLD");
    let diamond = top_rank(&r, "DIAMOND");
    for bucket in [&gold, &diamond] {
        assert_eq!(bucket.len(), 21);
        assert_eq!(bucket.iter().filter(|g| g.tier.is_some()).count(), 21);
    }
    // GOLD gagne 240 parties sur 400 : μ = 60 % en GOLD, 40 % en DIAMOND.
    assert!((bucket_mean(&gold) - 60.0).abs() < 1e-9);
    assert!((bucket_mean(&diamond) - 40.0).abs() < 1e-9);
    let in_gold = find_group(&r, 500, Role::Top, "GOLD");
    let in_diamond = find_group(&r, 500, Role::Top, "DIAMOND");
    for g in [in_gold, in_diamond] {
        assert_eq!((g.games, g.wins), (100, 50));
        assert!((g.pick_rate.unwrap() - 25.0).abs() < 1e-9);
    }
    // GOLD : (5 000 + 200 × 60) / 300 − 60 + 0,02 × 25 = −2,83 (D, dernier) ;
    // DIAMOND : +3,83 (S, premier). Avec une base fixe de 50, les deux vaudraient 0,5 (B).
    assert_eq!(
        (in_gold.tier.as_deref(), in_gold.position),
        (Some("D"), Some(21))
    );
    assert_eq!(
        (in_diamond.tier.as_deref(), in_diamond.position),
        (Some("S"), Some(1))
    );
}

#[test]
fn tier_method_publie_les_constantes_appliquees() {
    let method = Accumulator::new(1).unwrap().finish().tier_method;
    for expected in [
        "(games + 200)",
        "0.02*(pick_rate + ban_rate)",
        "S>=2.5 A>=1 B>=-1 C>=-2.5 else D",
        "pick_rate>=0.5",
        "at least 20 such champions",
        "no forced distribution",
    ] {
        assert!(
            method.contains(expected),
            "« {expected} » absent de {method}"
        );
    }
    assert!(!method.contains("percentile"));
}

#[test]
fn rejette_les_formats_standard_incomplets_et_resultats_impossibles() {
    for variant in 0..3 {
        let mut g = game("EUW1_invalid_format");
        if variant < 2 {
            g.queue_id = 450;
            g.detail["info"]["queueId"] = json!(450);
            if variant == 0 {
                let p = g.detail["info"]["participants"].as_array().unwrap();
                g.detail["info"]["participants"] = json!([p[0], p[5]]);
            } else {
                for p in g.detail["info"]["participants"].as_array_mut().unwrap() {
                    p["win"] = json!(true);
                }
            }
        } else {
            g.detail["info"]["participants"][5]["teamId"] = json!(100);
            g.detail["info"]["participants"][5]["teamPosition"] = json!("");
            g.detail["info"]["participants"][5]["win"] = json!(true);
        }
        let mut acc = Accumulator::new(1).unwrap();
        acc.add(&g);
        assert_eq!(acc.finish().included_matches, 0, "variant{variant}");
    }
}

#[test]
fn refuse_arena_si_aucune_equipe_ne_perd() {
    let mut g = game("EUW1_arena_all_win");
    g.queue_id = 1700;
    g.detail["info"]["queueId"] = json!(1700);
    g.detail["metadata"]["participants"] = json!((0..16)
        .map(|i| format!("synthetic-{i}"))
        .collect::<Vec<_>>());
    g.detail["info"]["participants"]=json!((0..16).map(|i|json!({"participantId":i+1,"teamId":100,"playerSubteamId":1+i/2,"championId":i+1,"win":true})).collect::<Vec<_>>());
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&g);
    assert_eq!(acc.finish().included_matches, 0);
}

#[test]
fn les_builds_ont_leurs_denominateurs_et_leurs_victoires_propres() {
    let mut acc = Accumulator::new(1).unwrap();
    for n in 0..3 {
        let mut g = game(&format!("EUW1_build{n}"));
        if n == 1 {
            reverse_winner(&mut g);
        }
        if n < 2 {
            let p = &mut g.detail["info"]["participants"][0];
            for slot in 0..6 {
                p[format!("item{slot}")] = json!(if slot < 2 { 1055 } else { 0 });
            }
            p["summoner1Id"] = json!(4);
            p["summoner2Id"] = json!(14);
            g.timeline = Some(
                json!({"metadata":{"matchId":g.match_id},"info":{"participants":[{"participantId":1}],"frames":[{"events":[{"type":"SKILL_LEVEL_UP","participantId":1,"timestamp":1000+n*2000,"skillSlot":2,"levelUpType":"NORMAL"}]}]}}),
            );
        }
        acc.add(&g);
    }
    let r = acc.finish();
    let item = r
        .builds
        .iter()
        .find(|b| b.key.rank == "ALL" && b.key.champion_id == 1 && b.category == "item")
        .unwrap();
    assert_eq!((item.games, item.wins, item.population), (2, Some(1), 2));
    assert_eq!((item.win_rate, item.pick_rate), (Some(50.0), Some(100.0)));
    assert_eq!(item.selection, vec![1055]);
    let skill = r
        .skill_levels
        .iter()
        .find(|s| s.key.rank == "ALL" && s.key.champion_id == 1)
        .unwrap();
    assert_eq!(
        (
            skill.point,
            skill.slot,
            skill.games,
            skill.mean_timestamp_ms
        ),
        (1, 2, 2, 2000.0)
    );
}

#[test]
fn les_variantes_builds_sont_bornees_sans_modifier_leur_population() {
    let mut acc = Accumulator::new(1).unwrap();
    for n in 0..25 {
        let mut g = game(&format!("EUW1_variant{n}"));
        let p = &mut g.detail["info"]["participants"][0];
        p["summoner1Id"] = json!(4);
        p["summoner2Id"] = json!(100 + n);
        acc.add(&g);
    }
    let r = acc.finish();
    let variants: Vec<_> = r
        .builds
        .iter()
        .filter(|b| b.key.rank == "ALL" && b.category == "summoner_spells")
        .collect();
    assert_eq!(variants.len(), 20);
    assert!(variants
        .iter()
        .all(|b| b.population == 25 && b.pick_rate == Some(4.0)));
    assert_eq!(r.omitted_build_variants, 10); // cinq variantes dans ALL, cinq dans UNKNOWN.
}

#[test]
fn preserve_arena_trio_et_le_mode_pve_solo_sans_leur_inventer_des_adversaires() {
    let mut trio = game("EUW1_trio");
    trio.queue_id = 1740;
    trio.detail["info"]["queueId"] = json!(1740);
    trio.detail["info"]["gameMode"] = json!("CHERRY");
    trio.detail["metadata"]["participants"] = json!((0..18)
        .map(|i| format!("synthetic-{i}"))
        .collect::<Vec<_>>());
    trio.detail["info"]["participants"]=json!((0..18).map(|i|json!({"participantId":i+1,"teamId":if i<9 {100}else{200},"playerSubteamId":1+i/3,"championId":i+1,"win":i<9})).collect::<Vec<_>>());
    let mut solo = game("EUW1_swarm");
    solo.queue_id = 1810;
    solo.detail["info"]["queueId"] = json!(1810);
    solo.detail["metadata"]["participants"] = json!(["fake-puuid-0"]);
    solo.detail["info"]["participants"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&trio);
    acc.add(&solo);
    let r = acc.finish();
    assert_eq!(r.included_matches, 2);
    assert_eq!(
        r.coverage
            .iter()
            .map(|c| c.counts.participations)
            .sum::<u64>(),
        19
    );
    assert!(r.groups.iter().all(|g| g.key.role == Role::Unknown));
}

#[test]
fn les_items_arena_ne_publient_aucune_victoire_ni_winrate_ni_tri_par_victoire() {
    let mut g = game("EUW1_arena_policy");
    g.queue_id = 1740;
    g.detail["info"]["queueId"] = json!(1740);
    g.detail["info"]["gameMode"] = json!("CHERRY");
    g.detail["metadata"]["participants"] = json!((0..18)
        .map(|i| format!("synthetic-{i}"))
        .collect::<Vec<_>>());
    g.detail["info"]["participants"]=json!((0..18).map(|i|json!({"participantId":i+1,"teamId":if i<9 {100}else{200},"playerSubteamId":1+i/3,"championId":i+1,"win":i<9,"item0":1001,"item1":0,"item2":0,"item3":0,"item4":0,"item5":0,"item6":0})).collect::<Vec<_>>());
    g.detail["info"]["participants"][0]["item0"] = json!(2000);
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&g);
    g.match_id = "EUW1_arena_policy_loss".into();
    g.detail["metadata"]["matchId"] = json!(g.match_id);
    reverse_winner(&mut g);
    g.detail["info"]["participants"][0]["item0"] = json!(1001);
    acc.add(&g);
    let r = serde_json::to_value(acc.finish()).unwrap();
    let builds = r["builds"].as_array().unwrap();
    assert!(!builds.is_empty());
    for b in builds {
        assert_eq!(b["win_rate"], Value::Null);
        assert_eq!(b["wins"], Value::Null);
        assert_eq!(b["performance_available"], false);
        assert!(b["pick_rate"].as_f64().unwrap() > 0.0);
    }
    let items: Vec<_> = builds
        .iter()
        .filter(|b| b["champion_id"] == 1 && b["rank"] == "ALL" && b["category"] == "item")
        .map(|b| b["selection"].clone())
        .collect();
    assert_eq!(items, vec![json!([1001]), json!([2000])]); // Le gagnant ne passe pas avant le perdant à fréquence égale.
}

#[test]
fn la_coop_conserve_les_joueurs_sans_compter_les_bots_adverses() {
    let mut g = game("EUW1_coop");
    g.queue_id = 880;
    g.detail["info"]["queueId"] = json!(880);
    g.detail["metadata"]["participants"] = json!((0..5)
        .map(|i| format!("fake-puuid-{i}"))
        .collect::<Vec<_>>());
    for p in g.detail["info"]["participants"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .skip(5)
    {
        p["puuid"] = json!("BOT");
    }
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&g);
    let r = acc.finish();
    assert_eq!(r.included_matches, 1);
    assert_eq!(r.coverage[0].counts.participations, 5);
    assert!(r.groups.iter().all(|g| g.wins == 1));
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json["coverage"][0]["excluded_bot_participations"], 5);
    g.detail["metadata"]["participants"] = json!(["fake-puuid-0"]);
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&g);
    assert_eq!(acc.finish().included_matches, 0);
}

#[test]
fn la_seconde_file_arena_trio_est_reconnue_sans_game_mode() {
    let mut g = game("EUW1_arena_trio2");
    g.queue_id = 1750;
    g.detail["info"]["queueId"] = json!(1750);
    g.detail["metadata"]["participants"] = json!((0..18)
        .map(|i| format!("synthetic-{i}"))
        .collect::<Vec<_>>());
    g.detail["info"]["participants"]=json!((0..18).map(|i|json!({"participantId":i+1,"teamId":if i<9 {100}else{200},"playerSubteamId":1+i/3,"championId":i+1,"win":i<9,"item0":1001,"item1":0,"item2":0,"item3":0,"item4":0,"item5":0,"item6":0})).collect::<Vec<_>>());
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&g);
    let r = acc.finish();
    assert_eq!(r.included_matches, 1);
    assert!(r.builds.iter().all(|b| !b.performance_available));
}

#[test]
fn le_rang_depend_de_l_ecart_a_la_partie_avec_une_borne_incluse() {
    let max = u64::from(DEFAULT_RANK_MAX_AGE_HOURS) * 3600;
    assert_eq!(DEFAULT_RANK_MAX_AGE_HOURS, 168);
    let mut g = game("EUW1_gap");
    g.ranks.insert(
        "fake-puuid-0".into(),
        observed("ranked", Some("GOLD"), 7200),
    );
    g.ranks.insert(
        "fake-puuid-1".into(),
        observed("ranked", Some("DIAMOND"), max),
    );
    g.ranks.insert(
        "fake-puuid-2".into(),
        observed("ranked", Some("SILVER"), max + 1),
    );
    g.ranks
        .insert("fake-puuid-3".into(), observed("unranked", None, 36_000));
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&g);
    let r = acc.finish();
    let ranks: Vec<_> = r.groups.iter().map(|g| g.key.rank.as_str()).collect();
    assert!(ranks.contains(&"GOLD") && ranks.contains(&"DIAMOND"));
    assert!(!ranks.contains(&"SILVER"));
    let c = &r.coverage[0].counts;
    assert_eq!(
        (
            c.ranked_participations,
            c.unranked_participations,
            c.unknown_rank_participations
        ),
        (2, 1, 7)
    );
    assert_eq!(c.unknown_rank_rate, Some(70.0));
    // Écarts retenus : 2 h, 10 h et 168 h ; l'observation trop éloignée est ignorée.
    assert_eq!(c.rank_gap_median_hours, Some(10.0));
    assert_eq!(c.rank_gap_max_hours, Some(168.0));
}

#[test]
fn l_age_maximal_est_configurable_et_borne() {
    let mut acc = Accumulator::new(1).unwrap();
    for invalid in [0, 8761] {
        assert!(matches!(
            acc.set_rank_max_age_hours(invalid),
            Err(AggregationError::InvalidRankMaxAge)
        ));
    }
    acc.set_rank_max_age_hours(3).unwrap();
    let mut g = game("EUW1_custom_age");
    g.ranks.insert(
        "fake-puuid-0".into(),
        observed("ranked", Some("GOLD"), 7200),
    );
    g.ranks.insert(
        "fake-puuid-1".into(),
        observed("ranked", Some("GOLD"), 3600),
    );
    g.ranks.insert(
        "fake-puuid-2".into(),
        observed("ranked", Some("DIAMOND"), 3 * 3600 + 1),
    );
    acc.add(&g);
    let r = acc.finish();
    assert_eq!(r.rank_max_age_hours, 3);
    assert!(!r.groups.iter().any(|g| g.key.rank == "DIAMOND"));
    let c = &r.coverage[0].counts;
    assert_eq!(c.ranked_participations, 2);
    assert_eq!(c.rank_gap_median_hours, Some(1.5));
    assert_eq!(c.rank_gap_max_hours, Some(2.0));
}

#[test]
fn les_files_non_classees_et_sans_observation_n_inventent_ni_part_ni_ecart() {
    let mut normal = game("EUW1_normal");
    normal.queue_id = 400;
    normal.detail["info"]["queueId"] = json!(400);
    normal
        .ranks
        .insert("fake-puuid-0".into(), observed("ranked", Some("GOLD"), 60));
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(&normal);
    acc.add(&game("EUW1_unobserved"));
    let r = acc.finish();
    let by_queue = |q: i32| {
        &r.coverage
            .iter()
            .find(|c| c.scope.queue_id == q)
            .unwrap()
            .counts
    };
    let normal = by_queue(400);
    assert_eq!(normal.unranked_mode_participations, 10);
    assert_eq!(normal.unknown_rank_rate, None);
    assert_eq!(normal.rank_gap_median_hours, None);
    let ranked = by_queue(420);
    assert_eq!(ranked.unknown_rank_rate, Some(100.0));
    assert_eq!(
        (ranked.rank_gap_median_hours, ranked.rank_gap_max_hours),
        (None, None)
    );
}

#[test]
fn une_couverture_publiee_avant_le_rang_fige_reste_lisible() {
    let mut legacy = serde_json::to_value(super::Coverage::default()).unwrap();
    for field in [
        "unknown_rank_rate",
        "rank_gap_median_hours",
        "rank_gap_max_hours",
    ] {
        legacy.as_object_mut().unwrap().remove(field).unwrap();
    }
    let coverage: super::Coverage = serde_json::from_value(legacy).unwrap();
    assert_eq!(coverage.unknown_rank_rate, None);
    assert_eq!(coverage.rank_gap_max_hours, None);
}

fn stage_catalog(version: &str) -> super::stages::ItemCatalog {
    let item = |price: u32, tags: Value, from: Value| {
        let mut fields =
            json!({"price_total":price,"purchasable":true,"categories":tags,"builds_from":from});
        for (_, value) in fields.as_object_mut().unwrap() {
            *value = json!({"value": value.take(), "status": "verified", "sources": []});
        }
        fields
    };
    let records = [
        ("1055", item(450, json!(["Lane"]), json!([]))),
        ("2003", item(50, json!(["Consumable"]), json!([]))),
        ("3006", item(1100, json!(["Boots"]), json!(["1001"]))),
        ("3031", item(3500, json!(["Damage"]), json!([]))),
        ("3089", item(3500, json!(["SpellDamage"]), json!([]))),
        ("6672", item(3000, json!(["Damage"]), json!([]))),
        ("3072", item(3400, json!(["Damage"]), json!([]))),
    ];
    super::stages::ItemCatalog::from_records(version, records.iter().map(|(id, f)| (*id, f)))
}

fn with_purchases(mut g: StoredMatch, items: &[(u32, u64)]) -> StoredMatch {
    let events: Vec<_> = items
        .iter()
        .map(|(id, at)| json!({"type":"ITEM_PURCHASED","participantId":1,"timestamp":at,"itemId":id}))
        .collect();
    g.timeline = Some(
        json!({"metadata":{"matchId":g.match_id},"info":{"participants":[{"participantId":1}],"frames":[{"events":events}]}}),
    );
    g
}

fn stage<'a>(r: &'a super::AggregationReport, category: &str) -> Vec<&'a super::BuildStats> {
    r.builds
        .iter()
        .filter(|b| b.key.rank == "ALL" && b.key.champion_id == 1 && b.category == category)
        .collect()
}

#[test]
fn les_etapes_d_achat_sont_agregees_separement_avec_le_catalogue_du_patch() {
    let mut acc = Accumulator::new(2).unwrap();
    acc.set_item_catalogs([("15.19".to_owned(), stage_catalog("15.19.1"))].into());
    let full = [
        (1055, 1_000),
        (2003, 1_500),
        (3006, 300_000),
        (3031, 600_000),
        (3089, 900_000),
        (6672, 1_200_000),
        (3072, 1_500_000),
    ];
    acc.add(&with_purchases(game("EUW1_stage1"), &full));
    acc.add(&with_purchases(game("EUW1_stage2"), &full));
    let mut lost = with_purchases(
        game("EUW1_stage3"),
        &[(1055, 1_000), (3089, 600_000), (3031, 900_000)],
    );
    reverse_winner(&mut lost);
    acc.add(&lost);
    let r = acc.finish();
    let starter = stage(&r, "starter");
    assert_eq!(starter.len(), 2);
    assert_eq!(starter[0].selection, vec![1055, 2003]);
    assert_eq!(
        (starter[0].games, starter[0].wins, starter[0].population),
        (2, Some(2), 3)
    );
    assert!((starter[0].win_rate_lower_bound.unwrap() - 34.23802275066532).abs() < 1e-9);
    // Sous le seuil : comptes visibles, aucune borne publiée.
    assert_eq!(
        (
            starter[1].selection.clone(),
            starter[1].win_rate_lower_bound
        ),
        (vec![1055], None)
    );
    let boots = stage(&r, "boots");
    assert_eq!(
        boots
            .iter()
            .map(|b| (b.selection.clone(), b.games))
            .collect::<Vec<_>>(),
        vec![(vec![3006], 2), (vec![], 1)]
    );
    // Le core n'a pour population que les parties ayant terminé trois objets.
    let core = stage(&r, "core");
    assert_eq!(core.len(), 1);
    assert_eq!(
        (core[0].selection.clone(), core[0].population),
        (vec![3031, 3089, 6672], 2)
    );
    assert_eq!(stage(&r, "item_slot_4")[0].selection, vec![3072]);
    assert!(stage(&r, "item_slot_5").is_empty());
    // Les empreintes exactes existantes restent publiées à l'identique.
    assert_eq!(stage(&r, "purchase_order").len(), 2);
    let coverage = &r.coverage[0].counts;
    assert_eq!(
        (
            coverage.item_stage_participations,
            coverage.missing_item_catalog_participations
        ),
        (3, 0)
    );
    assert_eq!(
        serde_json::to_value(&r.item_catalogs).unwrap(),
        json!([{"patch":"15.19","version":"15.19.1"}])
    );
    assert!(r.build_stage_method.contains("90000"));
}

#[test]
fn sans_catalogue_du_patch_aucune_etape_n_est_inventee() {
    let mut acc = Accumulator::new(1).unwrap();
    acc.set_item_catalogs([("15.18".to_owned(), stage_catalog("15.18.1"))].into());
    acc.add(&with_purchases(
        game("EUW1_nocatalog"),
        &[(1055, 1_000), (3031, 600_000)],
    ));
    let r = acc.finish();
    for category in ["starter", "boots", "core", "item_slot_4"] {
        assert!(stage(&r, category).is_empty());
    }
    assert_eq!(stage(&r, "purchase_order").len(), 1);
    let coverage = &r.coverage[0].counts;
    assert_eq!(
        (
            coverage.item_stage_participations,
            coverage.missing_item_catalog_participations
        ),
        (0, 1)
    );
}

#[test]
fn les_etapes_d_achat_arena_ne_publient_aucune_performance() {
    let mut g = game("EUW1_arena_stage");
    g.queue_id = 1740;
    g.detail["info"]["queueId"] = json!(1740);
    g.detail["info"]["gameMode"] = json!("CHERRY");
    g.detail["metadata"]["participants"] = json!((0..18)
        .map(|i| format!("synthetic-{i}"))
        .collect::<Vec<_>>());
    g.detail["info"]["participants"] = json!((0..18).map(|i| json!({"participantId":i+1,"teamId":if i<9 {100}else{200},"playerSubteamId":1+i/3,"championId":i+1,"win":i<9})).collect::<Vec<_>>());
    let g = with_purchases(
        g,
        &[
            (1055, 1_000),
            (3006, 2_000),
            (3031, 3_000),
            (3089, 4_000),
            (6672, 5_000),
            (3072, 6_000),
        ],
    );
    let mut acc = Accumulator::new(1).unwrap();
    acc.set_item_catalogs([("15.19".to_owned(), stage_catalog("15.19.1"))].into());
    acc.add(&g);
    let r = acc.finish();
    for category in ["starter", "boots", "core", "item_slot_4"] {
        let rows = stage(&r, category);
        assert_eq!(rows.len(), 1, "{category}");
        assert!(!rows[0].performance_available);
        assert_eq!(
            (rows[0].wins, rows[0].win_rate, rows[0].win_rate_lower_bound),
            (None, None, None)
        );
    }
}

#[test]
fn une_variante_publiee_avant_les_etapes_reste_lisible() {
    let legacy = json!({"patch":"15.19","platform_id":"EUW1","queue_id":420,"role":"TOP","rank":"ALL","champion_id":1,
        "category":"final_items","selection":[3031],"games":3,"wins":2,"performance_available":true,"population":3,"pick_rate":100.0,"win_rate":66.6});
    let build: super::BuildStats = serde_json::from_value(legacy).unwrap();
    assert_eq!(build.win_rate_lower_bound, None);
    let mut coverage = serde_json::to_value(super::Coverage::default()).unwrap();
    for field in [
        "item_stage_participations",
        "missing_item_catalog_participations",
    ] {
        coverage.as_object_mut().unwrap().remove(field).unwrap();
    }
    let coverage: super::Coverage = serde_json::from_value(coverage).unwrap();
    assert_eq!(coverage.item_stage_participations, 0);
}

#[test]
fn une_variante_sans_victoire_publie_une_borne_wilson_nulle_et_non_negative() {
    // Pour 0 victoire sur 118 parties, la formule de Wilson donne environ -1e-15 en
    // flottant : la borne publiée doit rester dans 0..=100 pour ne pas faire rejeter
    // toute la page de builds par le client desktop.
    let mut acc = Accumulator::new(118).unwrap();
    acc.set_item_catalogs([("15.19".to_owned(), stage_catalog("15.19.1"))].into());
    for i in 0..118 {
        let mut lost = with_purchases(
            game(&format!("EUW1_afk{i}")),
            &[(1055, 1_000), (3006, 300_000)],
        );
        reverse_winner(&mut lost);
        acc.add(&lost);
    }
    let r = acc.finish();
    let starter = stage(&r, "starter");
    assert_eq!((starter[0].games, starter[0].wins), (118, Some(0)));
    assert_eq!(starter[0].win_rate_lower_bound, Some(0.0));
    let champion = r
        .groups
        .iter()
        .find(|g| g.key.rank == "ALL" && g.key.champion_id == 1)
        .unwrap();
    assert_eq!(champion.win_rate_lower_bound, Some(0.0));
}

/// Partie classée dont tous les participants ont joué `played_s` secondes, durée `duration_s`.
fn timed(id: &str, duration_s: i32, played_s: u64) -> StoredMatch {
    let mut g = game(id);
    g.game_duration_s = duration_s;
    for p in g.detail["info"]["participants"].as_array_mut().unwrap() {
        p["timePlayed"] = json!(played_s);
    }
    g
}

fn aggregate_one(game: &StoredMatch) -> super::AggregationReport {
    let mut acc = Accumulator::new(1).unwrap();
    acc.add(game);
    acc.finish()
}

#[test]
fn exclut_les_parties_classees_tres_courtes_avec_leur_propre_compteur() {
    let report = aggregate_one(&timed("EUW1_1", 200, 200));
    assert_eq!(report.included_matches, 0);
    assert_eq!(report.source_matches, 1);
    assert_eq!(report.exclusions.get("short_game"), Some(&1));
    assert!(report.groups.is_empty(), "aucune contribution partielle");
    // Seuil inclus : exactement la durée minimale reste une partie valide.
    let at_threshold = timed("EUW1_2", DEFAULT_MIN_GAME_DURATION_S as i32, 300);
    assert_eq!(aggregate_one(&at_threshold).included_matches, 1);
    // Une reddition normale à 15 minutes n'est jamais écartée.
    let mut surrender = timed("EUW1_3", 900, 900);
    surrender.detail["info"]["participants"][0]["gameEndedInSurrender"] = json!(true);
    assert_eq!(aggregate_one(&surrender).included_matches, 1);
}

#[test]
fn exclut_les_parties_classees_avec_un_depart_precoce() {
    // 1440 s joués sur 1800 : exactement 80 %, conservé ; 1439 s : exclu.
    let mut at_limit = timed("EUW1_1", 1800, 1800);
    at_limit.detail["info"]["participants"][3]["timePlayed"] = json!(1440);
    assert_eq!(aggregate_one(&at_limit).included_matches, 1);
    let mut left = timed("EUW1_2", 1800, 1800);
    left.detail["info"]["participants"][3]["timePlayed"] = json!(1439);
    let report = aggregate_one(&left);
    assert_eq!(report.included_matches, 0);
    assert_eq!(report.exclusions.get("early_departure"), Some(&1));
    assert!(report.groups.is_empty());
}

#[test]
fn compte_une_partie_courte_avec_depart_une_seule_fois_sous_la_duree() {
    let mut both = timed("EUW1_1", 200, 200);
    both.detail["info"]["participants"][0]["timePlayed"] = json!(10);
    let report = aggregate_one(&both);
    assert_eq!(report.exclusions.get("short_game"), Some(&1));
    assert_eq!(report.exclusions.get("early_departure"), None);
    assert_eq!(report.exclusions.values().sum::<u64>(), 1);
}

#[test]
fn les_controles_de_qualite_ne_concernent_que_les_files_classees() {
    let mut short = timed("EUW1_1", 200, 200);
    short.queue_id = 400;
    short.detail["info"]["queueId"] = json!(400);
    short.detail["info"]["participants"][0]["timePlayed"] = json!(10);
    let report = aggregate_one(&short);
    assert_eq!(report.included_matches, 1);
    assert!(report.exclusions.is_empty());
    // Flex (440) est classée comme Solo/Duo.
    let mut flex = timed("EUW1_2", 200, 200);
    flex.queue_id = 440;
    flex.detail["info"]["queueId"] = json!(440);
    assert_eq!(aggregate_one(&flex).exclusions.get("short_game"), Some(&1));
}

#[test]
fn le_remake_et_l_invalidite_precedent_les_controles_de_qualite() {
    let mut remake = timed("EUW1_1", 200, 200);
    remake.is_remake = true;
    let report = aggregate_one(&remake);
    assert_eq!(report.exclusions.get("remake"), Some(&1));
    assert_eq!(report.exclusions.get("short_game"), None);
    let mut invalid = timed("EUW1_2", 200, 200);
    invalid.detail["info"]["participants"][0]["championId"] = json!(0);
    let report = aggregate_one(&invalid);
    assert_eq!(report.exclusions.get("invalid_match"), Some(&1));
    assert_eq!(report.exclusions.get("short_game"), None);
}

#[test]
fn un_temps_de_jeu_absent_n_est_pas_juge_et_un_type_invalide_est_refuse() {
    // Le fixture n'a pas de `timePlayed` : rien n'est deviné, la partie reste valide.
    assert_eq!(aggregate_one(&game("EUW1_1")).included_matches, 1);
    for bad in [json!("1800"), json!(-5), json!(1800.5), Value::Null] {
        let mut invalid = timed("EUW1_2", 1800, 1800);
        invalid.detail["info"]["participants"][2]["timePlayed"] = bad.clone();
        let report = aggregate_one(&invalid);
        assert_eq!(report.included_matches, 0, "{bad}");
        assert_eq!(report.exclusions.get("invalid_match"), Some(&1), "{bad}");
    }
}

#[test]
fn les_seuils_sont_configurables_publies_et_desactivables() {
    let report = Accumulator::new(1).unwrap().finish();
    assert_eq!(report.min_game_duration_s, DEFAULT_MIN_GAME_DURATION_S);
    assert_eq!(report.min_played_percent, DEFAULT_MIN_PLAYED_PERCENT);
    assert!(report.exclude_afk);
    let mut acc = Accumulator::new(1).unwrap();
    acc.set_quality_thresholds(&QualityThresholds {
        min_game_duration_s: 600,
        min_played_percent: 95,
        exclude_afk: true,
    })
    .unwrap();
    acc.add(&timed("EUW1_1", 500, 500));
    let mut left = timed("EUW1_2", 1800, 1800);
    left.detail["info"]["participants"][1]["timePlayed"] = json!(1700);
    acc.add(&left);
    let report = acc.finish();
    assert_eq!(
        (report.min_game_duration_s, report.min_played_percent),
        (600, 95)
    );
    assert_eq!(report.exclusions.get("short_game"), Some(&1));
    assert_eq!(report.exclusions.get("early_departure"), Some(&1));
    // Zéro désactive chaque contrôle indépendamment.
    let mut acc = Accumulator::new(1).unwrap();
    acc.set_quality_thresholds(&QualityThresholds {
        min_game_duration_s: 0,
        min_played_percent: 0,
        exclude_afk: false,
    })
    .unwrap();
    let mut left = timed("EUW1_3", 200, 200);
    left.detail["info"]["participants"][1]["timePlayed"] = json!(1);
    acc.add(&left);
    let report = acc.finish();
    assert_eq!(report.included_matches, 1);
    assert!(report.exclusions.is_empty());
    assert!(!report.exclude_afk);
}

#[test]
fn refuse_les_seuils_de_qualite_hors_bornes() {
    let mut acc = Accumulator::new(1).unwrap();
    assert!(matches!(
        acc.set_quality_thresholds(&QualityThresholds {
            min_game_duration_s: MAX_MIN_GAME_DURATION_S + 1,
            min_played_percent: 80,
            exclude_afk: true,
        }),
        Err(AggregationError::InvalidMinGameDuration)
    ));
    assert!(matches!(
        acc.set_quality_thresholds(&QualityThresholds {
            min_game_duration_s: 300,
            min_played_percent: 101,
            exclude_afk: true,
        }),
        Err(AggregationError::InvalidMinPlayedPercent)
    ));
    assert!(QualityThresholds::default().validate().is_ok());
}

#[test]
fn un_ancien_rapport_sans_seuils_se_relit_avec_des_controles_desactives() {
    let mut value = serde_json::to_value(Accumulator::new(1).unwrap().finish()).unwrap();
    value.as_object_mut().unwrap().remove("min_game_duration_s");
    value.as_object_mut().unwrap().remove("min_played_percent");
    value.as_object_mut().unwrap().remove("exclude_afk");
    let old: super::AggregationReport = serde_json::from_value(value).unwrap();
    assert_eq!((old.min_game_duration_s, old.min_played_percent), (0, 0));
    assert!(!old.exclude_afk);
}

/// Partie classée de durée normale dont le participant `index` porte `wasAfk = value`.
fn with_afk(id: &str, index: usize, value: Value) -> StoredMatch {
    let mut g = timed(id, 1800, 1800);
    g.detail["info"]["participants"][index]["wasAfk"] = value;
    g
}

#[test]
fn exclut_les_parties_classees_avec_un_participant_afk() {
    let report = aggregate_one(&with_afk("EUW1_1", 4, json!(true)));
    assert_eq!(report.included_matches, 0);
    assert_eq!(report.source_matches, 1);
    assert_eq!(report.exclusions.get("afk"), Some(&1));
    assert!(report.groups.is_empty(), "aucune contribution partielle");
    // Flex (440) est concernée comme Solo/Duo.
    let mut flex = with_afk("EUW1_2", 0, json!(true));
    flex.queue_id = 440;
    flex.detail["info"]["queueId"] = json!(440);
    assert_eq!(aggregate_one(&flex).exclusions.get("afk"), Some(&1));
    // `wasAfk = false` partout, ou clé absente : rien n'est écarté.
    assert_eq!(
        aggregate_one(&with_afk("EUW1_3", 4, json!(false))).included_matches,
        1
    );
    assert_eq!(
        aggregate_one(&timed("EUW1_4", 1800, 1800)).included_matches,
        1
    );
}

#[test]
fn un_indicateur_afk_non_booleen_rend_la_partie_incoherente() {
    for bad in [json!("true"), json!(1), Value::Null] {
        let report = aggregate_one(&with_afk("EUW1_1", 2, bad.clone()));
        assert_eq!(report.included_matches, 0, "{bad}");
        assert_eq!(report.exclusions.get("invalid_match"), Some(&1), "{bad}");
        assert_eq!(report.exclusions.get("afk"), None, "{bad}");
    }
    // Un type invalide l'emporte sur un AFK avéré d'un autre participant.
    let mut mixed = with_afk("EUW1_2", 0, json!(true));
    mixed.detail["info"]["participants"][1]["wasAfk"] = json!("oui");
    let report = aggregate_one(&mixed);
    assert_eq!(report.exclusions.get("invalid_match"), Some(&1));
    assert_eq!(report.exclusions.get("afk"), None);
}

#[test]
fn l_ordre_des_motifs_est_remake_invalide_courte_afk_depart_precoce() {
    // Courte et AFK : comptée `short_game` seulement.
    let mut short = with_afk("EUW1_1", 0, json!(true));
    short.game_duration_s = 200;
    let report = aggregate_one(&short);
    assert_eq!(report.exclusions.get("short_game"), Some(&1));
    assert_eq!(report.exclusions.get("afk"), None);
    // AFK et départ précoce : comptée `afk` seulement.
    let mut both = with_afk("EUW1_2", 0, json!(true));
    both.detail["info"]["participants"][1]["timePlayed"] = json!(10);
    let report = aggregate_one(&both);
    assert_eq!(report.exclusions.get("afk"), Some(&1));
    assert_eq!(report.exclusions.get("early_departure"), None);
    assert_eq!(report.exclusions.values().sum::<u64>(), 1);
    // Un remake reste un remake même avec un AFK.
    let mut remake = with_afk("EUW1_3", 0, json!(true));
    remake.is_remake = true;
    assert_eq!(aggregate_one(&remake).exclusions.get("remake"), Some(&1));
}

#[test]
fn le_controle_afk_ne_concerne_que_les_files_classees_et_se_desactive() {
    let mut normal = with_afk("EUW1_1", 0, json!(true));
    normal.queue_id = 400;
    normal.detail["info"]["queueId"] = json!(400);
    let report = aggregate_one(&normal);
    assert_eq!(report.included_matches, 1);
    assert!(report.exclusions.is_empty());
    // Désactivé : l'AFK est conservé et un type invalide n'est même pas lu.
    let mut acc = Accumulator::new(1).unwrap();
    acc.set_quality_thresholds(&QualityThresholds {
        exclude_afk: false,
        ..QualityThresholds::default()
    })
    .unwrap();
    acc.add(&with_afk("EUW1_2", 0, json!(true)));
    acc.add(&with_afk("EUW1_3", 1, json!("oui")));
    let report = acc.finish();
    assert_eq!(report.included_matches, 2);
    assert!(report.exclusions.is_empty());
    assert!(!report.exclude_afk);
}

/// Bans complets (5 par équipe) ; `-1` marque un emplacement sans ban.
fn set_bans(game: &mut StoredMatch, blue: &[i64], red: &[i64]) {
    let team = |id: u32, first_turn: usize, bans: &[i64]| {
        let list: Vec<Value> = (0..5)
            .map(|i| json!({"championId": bans.get(i).copied().unwrap_or(-1), "pickTurn": first_turn + i}))
            .collect();
        json!({"teamId": id, "bans": list})
    };
    game.detail["info"]["teams"] = json!([team(100, 1, blue), team(200, 6, red)]);
}

/// Rang observé et proche de la partie pour les joueurs `0..count`.
fn rank_first_players(game: &mut StoredMatch, count: usize, tier: &str, gap_s: u64) {
    for i in 0..count {
        game.ranks.insert(
            format!("fake-puuid-{i}"),
            observed("ranked", Some(tier), gap_s),
        );
    }
}

fn find_ban<'a>(
    report: &'a super::AggregationReport,
    rank: &str,
    champion_id: u32,
) -> Option<&'a super::BanStats> {
    report
        .bans
        .iter()
        .find(|b| b.rank == rank && b.champion_id == champion_id)
}

#[test]
fn les_bans_sont_publies_par_palier_de_partie_avec_leur_propre_denominateur() {
    let mut acc = Accumulator::new(1).unwrap();
    let mut gold = game("EUW1_gold");
    set_bans(&mut gold, &[99, 98], &[99]);
    rank_first_players(&mut gold, 6, "GOLD", 3600);
    acc.add(&gold);
    let mut diamond = game("EUW1_diamond");
    set_bans(&mut diamond, &[99], &[-1]);
    rank_first_players(&mut diamond, 6, "DIAMOND", 3600);
    acc.add(&diamond);
    // Cinq joueurs seulement : pas de palier de partie, la partie ne compte que dans ALL.
    let mut sparse = game("EUW1_sparse");
    set_bans(&mut sparse, &[99], &[]);
    rank_first_players(&mut sparse, 5, "GOLD", 3600);
    acc.add(&sparse);
    let report = acc.finish();

    let all_99 = find_ban(&report, "ALL", 99).unwrap();
    assert_eq!((all_99.banned_matches, all_99.draft_matches), (3, 3));
    assert_eq!(all_99.ban_rate, Some(100.0));
    let all_98 = find_ban(&report, "ALL", 98).unwrap();
    assert_eq!((all_98.banned_matches, all_98.draft_matches), (1, 3));
    let gold_99 = find_ban(&report, "GOLD", 99).unwrap();
    assert_eq!((gold_99.banned_matches, gold_99.draft_matches), (1, 1));
    assert_eq!(find_ban(&report, "GOLD", 98).unwrap().ban_rate, Some(100.0));
    let diamond_99 = find_ban(&report, "DIAMOND", 99).unwrap();
    assert_eq!(
        (diamond_99.banned_matches, diamond_99.draft_matches),
        (1, 1)
    );
    assert!(find_ban(&report, "DIAMOND", 98).is_none());
    // La partie sans palier suffisant est rangée sous UNKNOWN, jamais attribuée à un palier.
    let unknown_99 = find_ban(&report, "UNKNOWN", 99).unwrap();
    assert_eq!(
        (unknown_99.banned_matches, unknown_99.draft_matches),
        (1, 1)
    );
    // Chaque partie avec draft compte exactement une fois hors ALL.
    let per_rank_drafts: u64 = ["GOLD", "DIAMOND", "UNKNOWN"]
        .iter()
        .map(|rank| find_ban(&report, rank, 99).unwrap().draft_matches)
        .sum();
    assert_eq!(per_rank_drafts, report.coverage[0].counts.draft_matches);
    let counts = &report.coverage[0].counts;
    assert_eq!(counts.match_tier_matches, 2);
    assert_eq!(counts.unknown_match_tier_matches, 1);
}

#[test]
fn le_seuil_de_drafts_s_applique_a_chaque_palier() {
    let mut acc = Accumulator::new(2).unwrap();
    for (id, tier) in [
        ("EUW1_a", "GOLD"),
        ("EUW1_b", "GOLD"),
        ("EUW1_c", "DIAMOND"),
    ] {
        let mut g = game(id);
        set_bans(&mut g, &[99], &[]);
        rank_first_players(&mut g, 6, tier, 3600);
        acc.add(&g);
    }
    let report = acc.finish();
    assert_eq!(find_ban(&report, "ALL", 99).unwrap().ban_rate, Some(100.0));
    assert_eq!(find_ban(&report, "GOLD", 99).unwrap().ban_rate, Some(100.0));
    let diamond = find_ban(&report, "DIAMOND", 99).unwrap();
    assert_eq!((diamond.banned_matches, diamond.draft_matches), (1, 1));
    assert_eq!(diamond.ban_rate, None);
}

#[test]
fn un_rang_trop_eloigne_de_la_partie_ne_donne_pas_de_palier_de_partie() {
    let mut acc = Accumulator::new(1).unwrap();
    let mut g = game("EUW1_old");
    set_bans(&mut g, &[99], &[]);
    let too_old = u64::from(DEFAULT_RANK_MAX_AGE_HOURS) * 3600 + 1;
    rank_first_players(&mut g, 10, "GOLD", too_old);
    acc.add(&g);
    let report = acc.finish();
    assert!(find_ban(&report, "GOLD", 99).is_none());
    assert!(find_ban(&report, "UNKNOWN", 99).is_some());
}

#[test]
fn un_mode_sans_rang_publie_ses_bans_sous_all_et_unranked_mode() {
    let mut acc = Accumulator::new(1).unwrap();
    let mut draft = game("EUW1_draft");
    draft.queue_id = 400;
    draft.detail["info"]["queueId"] = json!(400);
    set_bans(&mut draft, &[99], &[]);
    // Même avec des rangs connus : hors Solo/Flex, aucun palier de partie.
    rank_first_players(&mut draft, 10, "GOLD", 3600);
    acc.add(&draft);
    let report = acc.finish();
    assert!(find_ban(&report, "ALL", 99).is_some());
    assert!(find_ban(&report, "UNRANKED_MODE", 99).is_some());
    assert!(find_ban(&report, "GOLD", 99).is_none());
    let counts = &report.coverage[0].counts;
    assert_eq!(
        (counts.match_tier_matches, counts.unknown_match_tier_matches),
        (0, 0)
    );
}

#[test]
fn le_rapport_publie_la_base_du_rang_des_bans() {
    let report = Accumulator::new(1).unwrap().finish();
    assert_eq!(report.ban_rank_basis, "match_median");
    assert_eq!(report.ban_rank_min_known_players, 6);
}

#[test]
fn un_ban_publie_avant_109_se_relit_sous_all() {
    let old: super::BanStats = serde_json::from_value(json!({
        "patch":"15.19","platform_id":"EUW1","queue_id":420,
        "champion_id":99,"banned_matches":1,"draft_matches":2,"ban_rate":50.0
    }))
    .unwrap();
    assert_eq!(old.rank, "ALL");
}
