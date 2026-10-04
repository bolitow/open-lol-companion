use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::super::model::{ObservedRank, ScopeKey, StoredMatch};
use super::super::storage::Settings;
use super::super::{AggregationOptions, QualityThresholds, DEFAULT_RANK_MAX_AGE_HOURS};
use super::*;
use crate::model::fixtures::match_detail;

fn game(id: &str, platform: &str, queue: i32, patch: &str) -> StoredMatch {
    let mut detail = match_detail(id, platform, queue, 1_000_000);
    detail["info"]["gameVersion"] = json!(format!("{patch}.1.2"));
    StoredMatch {
        match_id: id.into(),
        platform_id: platform.into(),
        queue_id: queue,
        patch: patch.into(),
        is_remake: false,
        game_duration_s: 1800,
        detail,
        timeline: None,
        ranks: BTreeMap::new(),
    }
}

fn reverse_winner(game: &mut StoredMatch) {
    for p in game.detail["info"]["participants"].as_array_mut().unwrap() {
        p["win"] = json!(!p["win"].as_bool().unwrap());
    }
}

fn observed(status: &str, tier: Option<&str>, gap_s: u64) -> ObservedRank {
    ObservedRank {
        status: status.into(),
        tier: tier.map(Into::into),
        gap_s,
    }
}

/// Timeline du participant 1 : achats nets et deux points de compétence.
fn with_timeline(mut g: StoredMatch, items: &[(u32, u64)]) -> StoredMatch {
    let mut events: Vec<Value> = items
        .iter()
        .map(|(id, at)| json!({"type":"ITEM_PURCHASED","participantId":1,"timestamp":at,"itemId":id}))
        .collect();
    for (slot, at) in [(1, 90_000), (3, 400_000)] {
        events.push(
            json!({"type":"SKILL_LEVEL_UP","participantId":1,"timestamp":at,
            "skillSlot":slot,"levelUpType":"NORMAL"}),
        );
    }
    g.timeline = Some(json!({"metadata":{"matchId":g.match_id},
        "info":{"participants":[{"participantId":1}],"frames":[{"events":events}]}}));
    g
}

fn with_bans(g: &mut StoredMatch, first: i64) {
    g.detail["info"]["teams"] = json!([
        {"teamId":100,"bans":(1..=5).map(|t|json!({"championId":first+t,"pickTurn":t})).collect::<Vec<_>>()},
        {"teamId":200,"bans":(6..=10).map(|t|json!({"championId":if t==6 {-1} else {first+t},"pickTurn":t})).collect::<Vec<_>>()}
    ]);
}

fn catalog(version: &str) -> ItemCatalog {
    let item = |price: u32, tags: Value| {
        let mut fields =
            json!({"price_total":price,"purchasable":true,"categories":tags,"builds_from":[]});
        for (_, value) in fields.as_object_mut().unwrap() {
            *value = json!({"value": value.take(), "status": "verified", "sources": []});
        }
        fields
    };
    let records = [
        ("1055", item(450, json!(["Lane"]))),
        ("3006", item(1100, json!(["Boots"]))),
        ("3031", item(3500, json!(["Damage"]))),
        ("3089", item(3500, json!(["SpellDamage"]))),
        ("6672", item(3000, json!(["Damage"]))),
    ];
    ItemCatalog::from_records(version, records.iter().map(|(id, f)| (*id, f)))
}

fn arena(id: &str) -> StoredMatch {
    let mut g = game(id, "EUW1", 1700, "15.19");
    g.detail["metadata"]["participants"] = json!((0..16)
        .map(|i| format!("synthetic-{i}"))
        .collect::<Vec<_>>());
    g.detail["info"]["participants"] = json!((0..16)
        .map(|i| json!({"participantId":i+1,
        "teamId":100,"playerSubteamId":1+i/2,"championId":1+i%4,"win":i<8}))
        .collect::<Vec<_>>());
    with_timeline(g, &[(3031, 600_000)])
}

fn coop(id: &str) -> StoredMatch {
    let mut g = game(id, "EUW1", 880, "15.19");
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
    g
}

/// Corpus couvrant toutes les sections : plusieurs périmètres d'une même plateforme et
/// d'un même patch, rangs observés (dont trop anciens), bans, plus de 20 variantes,
/// étapes avec et sans catalogue, Arena, coop avec bots et exclusions.
fn corpus() -> Vec<StoredMatch> {
    let mut games = vec![];
    for n in 0..24 {
        let mut g = game(&format!("EUW1_{n:02}"), "EUW1", 420, "15.19");
        let p = &mut g.detail["info"]["participants"][0];
        p["summoner1Id"] = json!(4);
        p["summoner2Id"] = json!(100 + n);
        for slot in 0..6 {
            p[format!("item{slot}")] = json!([3031, 3089, 6672, 0, 0, 0][slot]);
        }
        if n % 3 == 0 {
            reverse_winner(&mut g);
        }
        if n % 4 == 0 {
            for (i, tier) in [
                "GOLD", "GOLD", "SILVER", "PLATINUM", "GOLD", "GOLD", "EMERALD",
            ]
            .iter()
            .enumerate()
            {
                g.ranks.insert(
                    format!("fake-puuid-{i}"),
                    observed("ranked", Some(tier), 3600 * (i as u64 + 1)),
                );
            }
            g.ranks
                .insert("fake-puuid-8".into(), observed("unranked", None, 60));
            g.ranks.insert(
                "fake-puuid-9".into(),
                observed("ranked", Some("DIAMOND"), 3600 * 500),
            );
        }
        if n % 5 == 0 {
            with_bans(&mut g, 20 + n as i64);
        }
        if n % 2 == 0 {
            g = with_timeline(
                g,
                &[
                    (1055, 1_000),
                    (3006, 300_000),
                    (3031, 600_000),
                    (3089, 900_000),
                    (6672, 1_200_000),
                ],
            );
        }
        games.push(g);
    }
    let mut remake = game("EUW1_remake", "EUW1", 420, "15.19");
    remake.is_remake = true;
    let mut short = game("EUW1_short", "EUW1", 420, "15.19");
    short.game_duration_s = 200;
    let mut afk = game("EUW1_afk", "EUW1", 420, "15.19");
    afk.detail["info"]["participants"][3]["wasAfk"] = json!(true);
    games.extend([remake, short, afk]);
    for n in 0..3 {
        let next = game(&format!("EUW1_next{n}"), "EUW1", 420, "15.20");
        games.push(with_timeline(next, &[(1055, 1_000), (3031, 600_000)]));
        let mut aram = game(&format!("EUW1_aram{n}"), "EUW1", 450, "15.19");
        with_bans(&mut aram, 50);
        games.push(aram);
        let mut na = game(&format!("NA1_{n}"), "NA1", 420, "15.19");
        if n == 1 {
            reverse_winner(&mut na);
        }
        games.push(na);
        let mut flex = game(&format!("KR_{n}"), "KR", 440, "15.19");
        flex.ranks.insert(
            "fake-puuid-2".into(),
            observed("ranked", Some("MASTER"), 10),
        );
        games.push(flex);
    }
    let mut invalid = game("NA1_invalid", "NA1", 420, "15.19");
    invalid.detail["info"]["participants"][0]["championId"] = json!(0);
    games.extend([invalid, arena("EUW1_arena0"), arena("EUW1_arena1")]);
    games.extend([coop("EUW1_coop0"), coop("EUW1_coop1")]);
    // Ordre de lecture du recalcul complet : identifiant de partie, périmètres mêlés.
    games.sort_by(|a, b| a.match_id.cmp(&b.match_id));
    games
}

fn settings() -> Settings {
    Settings {
        min_games: 2,
        rank_max_age_hours: DEFAULT_RANK_MAX_AGE_HOURS,
        quality: QualityThresholds::default(),
        filters: AggregationOptions::default(),
    }
}

fn scope(g: &StoredMatch) -> ScopeKey {
    ScopeKey {
        patch: g.patch.clone(),
        platform_id: g.platform_id.clone(),
        queue_id: g.queue_id,
    }
}

#[test]
fn le_cumul_des_lots_reproduit_exactement_le_recalcul_complet() {
    let settings = settings();
    let catalogs = BTreeMap::from([("15.19".to_owned(), catalog("15.19.1"))]);
    let games = corpus();
    let mut full = settings.accumulator(catalogs.clone()).unwrap();
    for g in &games {
        full.add(g);
    }
    let full = full.finish();
    // Le corpus exerce bien chaque section et chaque compteur.
    assert!(full.omitted_build_variants > 0);
    assert!(full.exclusions.len() >= 4, "{:?}", full.exclusions);
    assert!(full.groups.iter().any(|g| g.tier.is_some()));
    assert!(full.bans.iter().any(|b| b.rank == "GOLD"));
    assert!(!full.skill_levels.is_empty() && !full.item_events.is_empty());
    assert!(full.builds.iter().any(|b| b.category == "core"));
    assert!(full.builds.iter().any(|b| !b.performance_available));
    assert!(full
        .coverage
        .iter()
        .any(|c| c.counts.missing_item_catalog_participations > 0));

    let mut lots = BTreeMap::<ScopeKey, Vec<&StoredMatch>>::new();
    for g in &games {
        lots.entry(scope(g)).or_default().push(g);
    }
    assert!(lots.len() >= 7);
    let mut merged = settings.accumulator(catalogs.clone()).unwrap().finish();
    for (scope, lot) in &lots {
        let mut accumulator = settings
            .accumulator(lot_catalogs(&catalogs, scope))
            .unwrap();
        for g in lot {
            accumulator.add(g);
        }
        merged.append(accumulator.finish());
    }
    assert_eq!(merged, full);
}

#[test]
fn les_compteurs_relus_d_un_lot_reconstituent_l_en_tete_du_recalcul_complet() {
    let settings = settings();
    let catalogs = BTreeMap::from([("15.19".to_owned(), catalog("15.19.1"))]);
    let games = corpus();
    let mut full = settings.accumulator(catalogs.clone()).unwrap();
    let mut lots = BTreeMap::<ScopeKey, Vec<&StoredMatch>>::new();
    for g in &games {
        full.add(g);
        lots.entry(scope(g)).or_default().push(g);
    }
    let mut expected = full.finish();
    expected.coverage.clear();
    expected.groups.clear();
    expected.bans.clear();
    expected.builds.clear();
    expected.skill_levels.clear();
    expected.item_events.clear();
    // Chemin publié : seuls les compteurs de chaque lot, stockés en JSON, rejoignent l'en-tête.
    let mut head = settings.accumulator(catalogs.clone()).unwrap().finish();
    for (scope, lot) in &lots {
        let mut accumulator = settings
            .accumulator(lot_catalogs(&catalogs, scope))
            .unwrap();
        for g in lot {
            accumulator.add(g);
        }
        let stored = serde_json::to_value(LotCounts::of(&accumulator.finish())).unwrap();
        serde_json::from_value::<LotCounts>(stored)
            .unwrap()
            .add_to(&mut head);
    }
    assert_eq!(head, expected);
}

#[test]
fn le_catalogue_d_un_lot_se_limite_a_son_patch() {
    let catalogs = BTreeMap::from([
        ("15.19".to_owned(), catalog("15.19.1")),
        ("15.20".to_owned(), catalog("15.20.1")),
    ]);
    let lot = |patch: &str| ScopeKey {
        patch: patch.into(),
        platform_id: "EUW1".into(),
        queue_id: 420,
    };
    let only = lot_catalogs(&catalogs, &lot("15.20"));
    assert_eq!(only.keys().collect::<Vec<_>>(), ["15.20"]);
    assert!(lot_catalogs(&catalogs, &lot("15.18")).is_empty());
}

#[test]
fn l_empreinte_du_catalogue_suit_son_classement_et_non_sa_seule_version() {
    let republished = |price: u64, name: &str| {
        let fields = json!({
            "price_total": {"value": price, "status": "verified", "sources": []},
            "purchasable": {"value": true, "status": "verified", "sources": []},
            "categories": {"value": ["Damage"], "status": "verified", "sources": []},
            "name": {"value": name, "status": "verified", "sources": []},
        });
        ItemCatalog::from_records("15.19.1", [("3031", &fields)]).fingerprint()
    };
    // Même version, même classement : une republication sans effet garde l'empreinte.
    assert_eq!(republished(3500, "A"), republished(3500, "B"));
    // Même version, objet devenu complet : les étapes changent, l'empreinte aussi.
    assert_ne!(republished(1500, "A"), republished(3500, "A"));
    // Même contenu, autre version : l'empreinte change (version publiée dans l'en-tête).
    assert_ne!(
        catalog("15.19.1").fingerprint(),
        catalog("15.19.2").fingerprint()
    );
    assert_eq!(
        catalog("15.19.1").fingerprint(),
        catalog("15.19.1").fingerprint()
    );
}
