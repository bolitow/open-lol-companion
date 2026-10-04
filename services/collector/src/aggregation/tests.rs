use serde_json::{json, Value};

use super::model::{Accumulator, ObservedRank, StoredMatch};
use super::{AggregationError, Role, DEFAULT_RANK_MAX_AGE_HOURS};
use crate::model::fixtures::match_detail;

fn game(id: &str) -> StoredMatch {
    StoredMatch {
        match_id: id.into(),
        platform_id: "EUW1".into(),
        queue_id: 420,
        patch: "15.19".into(),
        is_remake: false,
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
    assert_eq!(report["groups"][0]["pick_rate"], 50.0);
    assert_eq!(report["bans"][0]["banned_matches"], 1);
    assert_eq!(report["bans"][0]["draft_matches"], 1);
    assert_eq!(report["bans"][0]["ban_rate"], 100.0);
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

#[test]
fn le_classement_wilson_et_les_tiers_demandent_assez_de_champions() {
    let mut acc = Accumulator::new(1).unwrap();
    for n in 0..6 {
        let mut g = game(&format!("EUW1_tier{n}"));
        g.detail["info"]["participants"][0]["championId"] = json!(100 + n);
        acc.add(&g);
    }
    let r = acc.finish();
    let top: Vec<_> = r
        .groups
        .iter()
        .filter(|g| g.key.rank == "ALL" && g.key.role == Role::Top)
        .collect();
    assert_eq!(top.len(), 7);
    assert_eq!(top[0].tier.as_deref(), Some("S"));
    assert!(top
        .iter()
        .all(|g| g.win_rate_lower_bound.is_some() && g.tier.is_some()));
    assert!(r
        .groups
        .iter()
        .filter(|g| g.key.role == Role::Jungle)
        .all(|g| g.tier.is_none()));
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

fn spell_games(acc: &mut Accumulator, prefix: &str, first: u32, second: u32, count: u32) {
    for n in 0..count {
        let mut g = game(&format!("{prefix}{n}"));
        let p = &mut g.detail["info"]["participants"][0];
        p["summoner1Id"] = json!(first);
        p["summoner2Id"] = json!(second);
        acc.add(&g);
    }
}

fn spell_selections(report: &super::AggregationReport) -> Vec<(Vec<u32>, u64)> {
    report
        .builds
        .iter()
        .filter(|b| {
            b.key.rank == "ALL" && b.key.champion_id == 1 && b.category == "summoner_spells"
        })
        .map(|b| (b.selection.clone(), b.games))
        .collect()
}

#[test]
fn les_sorts_publient_l_orientation_d_f_majoritaire_meme_sans_flash() {
    let mut acc = Accumulator::new(1).unwrap();
    // Téléportation (12) en D, Fantôme (6) en F : majoritaire malgré l'ordre numérique,
    // et une seule variante (paire non ordonnée) malgré les deux orientations.
    spell_games(&mut acc, "EUW1_d", 6, 12, 1);
    spell_games(&mut acc, "EUW1_e", 12, 6, 2);
    assert_eq!(spell_selections(&acc.finish()), vec![(vec![12, 6], 3)]);
}

#[test]
fn les_sorts_a_egalite_d_orientation_suivent_l_ordre_numerique() {
    let mut acc = Accumulator::new(1).unwrap();
    spell_games(&mut acc, "EUW1_f", 12, 6, 2);
    spell_games(&mut acc, "EUW1_g", 6, 12, 2);
    assert_eq!(spell_selections(&acc.finish()), vec![(vec![6, 12], 4)]);
}

#[test]
fn l_orientation_majoritaire_s_applique_aussi_aux_paires_avec_flash() {
    let mut acc = Accumulator::new(1).unwrap();
    spell_games(&mut acc, "EUW1_h", 14, 4, 3);
    spell_games(&mut acc, "EUW1_i", 4, 14, 1);
    assert_eq!(spell_selections(&acc.finish()), vec![(vec![14, 4], 4)]);
}

#[test]
fn l_orientation_ne_modifie_ni_le_classement_ni_les_compteurs_des_variantes() {
    let mut acc = Accumulator::new(1).unwrap();
    spell_games(&mut acc, "EUW1_j", 12, 6, 2);
    spell_games(&mut acc, "EUW1_k", 4, 14, 3);
    let r = acc.finish();
    // Classement par effectif ; population commune aux deux paires.
    assert_eq!(
        spell_selections(&r),
        vec![(vec![4, 14], 3), (vec![12, 6], 2)]
    );
    assert!(r
        .builds
        .iter()
        .filter(|b| b.category == "summoner_spells" && b.key.rank == "ALL")
        .all(|b| b.population == 5));
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

#[test]
fn une_file_inconnue_est_isolee_par_une_exclusion_explicite() {
    // 710 et 3130 sont observées en recette sans que leur sens soit vérifiable hors ligne :
    // elles ne contribuent à aucun groupe, mais leur nombre reste visible.
    let mut acc = Accumulator::new(1).unwrap();
    for (id, queue) in [("EUW1_710", 710), ("EUW1_3130", 3130), ("EUW1_9999", 9999)] {
        let mut g = game(id);
        g.queue_id = queue;
        g.detail["info"]["queueId"] = json!(queue);
        acc.add(&g);
    }
    acc.add(&game("EUW1_ranked"));
    let r = acc.finish();
    assert_eq!((r.source_matches, r.included_matches), (4, 1));
    assert_eq!(r.exclusions.get("unknown_queue"), Some(&3));
    assert!(r.groups.iter().all(|g| g.key.queue_id == 420));
    assert!(r.coverage.iter().all(|c| c.scope.queue_id == 420));
}

#[test]
fn les_files_identifiees_hors_classe_restent_agregees() {
    // ARAM, Swiftplay et Arena (1700 et variantes observées) ne sont jamais isolées.
    for queue in [450, 480, 1700, 1710, 1740, 1750] {
        assert!(
            crate::queues::is_identified(queue),
            "la file {queue} doit être identifiée"
        );
    }
    for queue in [0, -1, 710, 3130, 9999] {
        assert!(
            !crate::queues::is_identified(queue),
            "la file {queue} ne doit pas être identifiée"
        );
    }
}
