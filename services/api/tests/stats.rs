//! Lecture des instantanés #18 sur PostgreSQL réel, avec populations synthétiques.
mod common;

use common::TestDb;
use olc_api::error::ApiError;
use olc_api::query::StatsQuery;
use olc_api::stats::{builds, tierlist};
use olc_collector::aggregation::AggregationReport;
use serde_json::{json, Value};
use sqlx::PgPool;

macro_rules! db_or_skip {
    () => {
        match TestDb::create().await {
            Some(db) => db,
            None => return,
        }
    };
}

fn query() -> StatsQuery {
    StatsQuery {
        patch: "16.19".into(),
        platform: "EUW1".into(),
        queue: 420,
        role: "TOP".into(),
        rank: "ALL".into(),
        offset: 0,
        limit: 50,
    }
}

fn champion(id: u32, position: Option<u32>) -> Value {
    json!({
        "patch": "16.19", "platform_id": "EUW1", "queue_id": 420,
        "role": "TOP", "rank": "ALL", "champion_id": id,
        "games": 100, "wins": 60, "losses": 40, "population": 500,
        "win_rate": 60.0, "pick_rate": 20.0, "win_rate_lower_bound": 50.2,
        "position": position, "tier": "A", "most_picked_rank": "GOLD"
    })
}

fn variants() -> Vec<(&'static str, Value)> {
    vec![
        ("patch", json!("16.18")),
        ("platform_id", json!("KR")),
        ("queue_id", json!(440)),
        ("role", json!("JUNGLE")),
        ("rank", json!("GOLD")),
    ]
}

fn contaminated(values: &mut Vec<Value>, base: &Value) {
    for (field, value) in variants() {
        let mut other = base.clone();
        other[field] = value;
        values.push(other);
    }
}

fn report() -> Value {
    let mut insufficient = champion(4, None);
    for field in ["win_rate", "pick_rate", "win_rate_lower_bound", "tier"] {
        insufficient[field] = Value::Null;
    }
    insufficient["games"] = json!(3);
    insufficient["wins"] = json!(1);
    insufficient["losses"] = json!(2);
    let mut groups = vec![
        insufficient,
        champion(1, Some(2)),
        champion(3, Some(1)),
        champion(2, Some(1)),
    ];
    let mut another_population = champion(1, Some(1));
    another_population["games"] = json!(200);
    another_population["wins"] = json!(120);
    another_population["losses"] = json!(80);
    contaminated(&mut groups, &another_population);

    let mut bans: Vec<Value> = [1, 2, 3, 4, 999]
        .into_iter()
        .map(|id| {
            json!({
                "patch":"16.19", "platform_id":"EUW1", "queue_id":420,
                "champion_id":id, "banned_matches":20, "draft_matches":100, "ban_rate":20.0
            })
        })
        .collect();
    for (field, value) in variants().into_iter().take(3) {
        let mut other = bans[0].clone();
        other[field] = value;
        other["banned_matches"] = json!(99);
        bans.push(other);
    }

    let coverage = json!({
        "patch":"16.19", "platform_id":"EUW1", "queue_id":420,
        "matches":100, "participations":1000, "excluded_bot_participations":0,
        "ranked_participations":500, "unranked_participations":0,
        "unknown_rank_participations":500, "unranked_mode_participations":0,
        "unknown_role_participations":0, "timeline_matches":80,
        "timeline_participations":800, "invalid_timeline_participations":5,
        "unidentified_item_undos":3, "draft_matches":100,
        "unknown_rank_rate":50.0, "rank_gap_median_hours":12.5, "rank_gap_max_hours":160.0,
        "item_stage_participations":700, "missing_item_catalog_participations":10
    });
    let mut coverage_entries = vec![coverage.clone()];
    for (field, value) in variants().into_iter().take(3) {
        let mut other = coverage.clone();
        other[field] = value;
        other["matches"] = json!(999);
        coverage_entries.push(other);
    }

    let mut build_values: Vec<Value> = [1001, 1002, 1003]
        .into_iter()
        .map(|id| {
            json!({
                "patch":"16.19", "platform_id":"EUW1", "queue_id":420,
                "role":"TOP", "rank":"ALL", "champion_id":1,
                "category":"final_items", "selection":[id], "games":1500-id,
                "wins":200, "performance_available":true, "population":1000,
                "pick_rate":40.0, "win_rate":50.0
            })
        })
        .collect();
    let base = build_values[0].clone();
    contaminated(&mut build_values, &base);
    let mut other_champion = base;
    other_champion["champion_id"] = json!(2);
    build_values.push(other_champion);

    let skill = json!({
        "patch":"16.19", "platform_id":"EUW1", "queue_id":420,
        "role":"TOP", "rank":"ALL", "champion_id":1,
        "point":1, "slot":1, "games":90, "mean_timestamp_ms":32_000.0
    });
    let mut skills = vec![skill.clone()];
    contaminated(&mut skills, &skill);
    let mut other_skill = skill;
    other_skill["champion_id"] = json!(2);
    skills.push(other_skill);
    let item = json!({
        "patch":"16.19", "platform_id":"EUW1", "queue_id":420,
        "role":"TOP", "rank":"ALL", "champion_id":1,
        "event":"ITEM_PURCHASED", "item_id":1001, "minute":1, "events":42
    });
    let mut items = vec![item.clone()];
    contaminated(&mut items, &item);
    let mut other_item = item;
    other_item["champion_id"] = json!(2);
    items.push(other_item);

    json!({
        "schema_version":2, "rank_scope":"observed_rank_nearest_to_game_start_of_same_ranked_queue",
        "rank_max_age_hours":168, "pick_rate_definition":"participations_in_group",
        "tier_method":"wilson_lower_bound", "min_games":100,
        "filters":{"patches":["16.19","16.18"], "platforms":["EUW1","KR"], "queues":[420,440],
            "start_ms":1_000_000, "end_ms":2_000_000},
        "source_matches":100, "included_matches":99, "exclusions":{"remake":1},
        "coverage":coverage_entries, "groups":groups, "bans":bans,
        "builds":build_values, "skill_levels":skills, "item_events":items,
        "max_build_variants_per_category":20, "omitted_build_variants":7
    })
}

async fn publish(pool: &PgPool, report: Value) {
    // Valider la fixture contre le format du collecteur avant de tester son lecteur.
    let _: AggregationReport = serde_json::from_value(report.clone()).unwrap();
    sqlx::query(
        "INSERT INTO champion_stats_snapshot (id,source_snapshot_at,published_at,report)
        VALUES (1,to_timestamp(2000),to_timestamp(2001),$1)",
    )
    .bind(report)
    .execute(pool)
    .await
    .unwrap();
}

async fn snapshot_plan(pool: &PgPool, query: &StatsQuery, champion: Option<u32>) -> Value {
    // EXPLAIN porte sur la requête réellement exécutée par l'API, pas sur une copie simplifiée.
    let sql = format!(
        "EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {}",
        include_str!("../src/sql/stats_snapshot.sql")
    );
    let vars = json!({"patch":query.patch,"platform":query.platform,"queue":query.queue,
        "role":query.role,"rank":query.rank,"champion":champion});
    sqlx::query_scalar(&sql)
        .bind(vars)
        .bind("$[*] ? (@.patch == $patch && @.platform_id == $platform && @.queue_id == $queue && @.role == $role && @.rank == $rank && ($champion == null || @.champion_id == $champion))")
        .bind("$[*] ? (@.patch == $patch && @.platform_id == $platform && @.queue_id == $queue)")
        .bind(champion.is_some())
        .fetch_one(pool).await.unwrap()
}

fn selected_chunk_rows(plan: &Value) -> u64 {
    if plan["Relation Name"] == "champion_stats_snapshot_chunks" {
        return plan["Actual Rows"].as_u64().unwrap() * plan["Actual Loops"].as_u64().unwrap();
    }
    plan["Plans"]
        .as_array()
        .map_or(0, |children| children.iter().map(selected_chunk_rows).sum())
}

#[tokio::test]
async fn une_lecture_de_build_ne_parcourt_pas_les_morceaux_des_autres_populations() {
    let db = db_or_skip!();
    let mut source = report();
    publish(db.storage.pool(), source.clone()).await;
    let selected = source["builds"].clone();
    for section in [
        "coverage",
        "groups",
        "bans",
        "builds",
        "skill_levels",
        "item_events",
    ] {
        source.as_object_mut().unwrap().remove(section);
    }
    sqlx::query("UPDATE champion_stats_snapshot SET storage_version=2,report=$1 WHERE id=1")
        .bind(source)
        .execute(db.storage.pool())
        .await
        .unwrap();
    // Le morceau utile contient aussi d'autres populations : le filtre fin doit rester actif.
    sqlx::query("INSERT INTO champion_stats_snapshot_chunks(snapshot_id,section,chunk_index,items) VALUES (1,'builds',0,$1)")
        .bind(&selected).execute(db.storage.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO champion_stats_snapshot_chunks(snapshot_id,section,chunk_index,items)
        SELECT 1,'builds',n,jsonb_build_array($1::jsonb || jsonb_build_object('champion_id',1000+n))
        FROM generate_series(1,2048) n",
    )
    .bind(&selected[0])
    .execute(db.storage.pool())
    .await
    .unwrap();
    sqlx::query("ANALYZE champion_stats_snapshot_chunks")
        .execute(db.storage.pool())
        .await
        .unwrap();

    let requested = StatsQuery {
        limit: 1,
        ..query()
    };
    let response = builds(db.storage.pool(), requested.clone(), 1)
        .await
        .unwrap();
    assert_eq!(response.total, 3);
    assert_eq!(response.builds.len(), 1);
    for (request, champion, expected_chunks) in [
        (requested.clone(), Some(1), 1),
        (requested.clone(), Some(999_999), 0),
        (
            StatsQuery {
                patch: "16.17".into(),
                ..requested.clone()
            },
            Some(1),
            0,
        ),
        // TOP/GOLD et JUNGLE/ALL existent, mais pas JUNGLE/GOLD : ne pas croiser leurs dimensions.
        (
            StatsQuery {
                role: "JUNGLE".into(),
                rank: "GOLD".into(),
                ..requested.clone()
            },
            Some(1),
            0,
        ),
        (requested, None, 0),
    ] {
        let plan = snapshot_plan(db.storage.pool(), &request, champion).await;
        assert_eq!(
            plan[0]["Plan"]["Actual Rows"], 1,
            "la lecture doit retourner uniquement le morceau utile, ou la tête seule"
        );
        assert!(
            plan.to_string().contains("snapshot_chunk_populations_idx"),
            "les morceaux doivent être sélectionnés par index : {plan}"
        );
        assert_eq!(selected_chunk_rows(&plan[0]["Plan"]), expected_chunks);
    }
    db.cleanup().await;
}

#[tokio::test]
async fn la_migration_indexe_les_morceaux_existants_sans_changer_les_reponses() {
    let db = db_or_skip!();
    let mut source = report();
    publish(db.storage.pool(), source.clone()).await;
    let old_tiers =
        serde_json::to_value(tierlist(db.storage.pool(), query()).await.unwrap()).unwrap();
    let old_builds =
        serde_json::to_value(builds(db.storage.pool(), query(), 1).await.unwrap()).unwrap();
    // Reproduire une publication v2 antérieure à 0009, puis migrer ses morceaux déjà présents.
    sqlx::raw_sql(
        "ALTER TABLE champion_stats_snapshot_chunks DROP COLUMN populations;
        DROP FUNCTION snapshot_chunk_populations(TEXT, JSONB);",
    )
    .execute(db.storage.pool())
    .await
    .unwrap();
    let mut tx = db.storage.pool().begin().await.unwrap();
    for section in [
        "coverage",
        "groups",
        "bans",
        "builds",
        "skill_levels",
        "item_events",
    ] {
        let items = source.as_object_mut().unwrap().remove(section).unwrap();
        for (index, chunk) in items.as_array().unwrap().chunks(2).enumerate() {
            sqlx::query("INSERT INTO champion_stats_snapshot_chunks (snapshot_id,section,chunk_index,items) VALUES (1,$1,$2,$3)")
                .bind(section).bind(index as i32).bind(json!(chunk)).execute(&mut *tx).await.unwrap();
        }
    }
    sqlx::query("UPDATE champion_stats_snapshot SET storage_version=2,report=$1 WHERE id=1")
        .bind(source)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    sqlx::raw_sql(include_str!(
        "../../collector/migrations/0009_snapshot_chunk_populations.sql"
    ))
    .execute(db.storage.pool())
    .await
    .unwrap();
    assert_eq!(
        serde_json::to_value(tierlist(db.storage.pool(), query()).await.unwrap()).unwrap(),
        old_tiers
    );
    assert_eq!(
        serde_json::to_value(builds(db.storage.pool(), query(), 1).await.unwrap()).unwrap(),
        old_builds
    );
    // Une réécriture du JSON doit déplacer automatiquement les clés indexées.
    sqlx::query(
        "UPDATE champion_stats_snapshot_chunks SET items=(
        SELECT jsonb_agg(item || jsonb_build_object('champion_id',1000))
        FROM jsonb_array_elements(items) item)
        WHERE section='builds' AND chunk_index=0",
    )
    .execute(db.storage.pool())
    .await
    .unwrap();
    assert_eq!(
        builds(db.storage.pool(), query(), 1).await.unwrap().total,
        1
    );
    assert_eq!(
        builds(db.storage.pool(), query(), 1000)
            .await
            .unwrap()
            .total,
        2
    );
    db.cleanup().await;
}

#[tokio::test]
async fn instantane_absent_reste_indisponible_pour_les_deux_lecteurs() {
    let db = db_or_skip!();
    assert_eq!(
        tierlist(db.storage.pool(), query()).await.err(),
        Some(ApiError::Unavailable)
    );
    assert_eq!(
        builds(db.storage.pool(), query(), 1).await.err(),
        Some(ApiError::Unavailable)
    );
    db.cleanup().await;
}

#[tokio::test]
async fn tierlist_isole_la_population_pagine_et_garde_les_bans_de_la_page() {
    let db = db_or_skip!();
    let source = report();
    publish(db.storage.pool(), source.clone()).await;
    let query = StatsQuery {
        offset: 1,
        limit: 2,
        ..query()
    };
    let response = tierlist(db.storage.pool(), query.clone()).await.unwrap();
    assert_eq!(response.query, query);
    assert_eq!(response.total, 4);
    assert_eq!(
        response
            .entries
            .iter()
            .map(|e| e.key.champion_id)
            .collect::<Vec<_>>(),
        [3, 1]
    );
    assert!(response
        .entries
        .iter()
        .all(|e| e.games == 100 && e.population == 500));
    let mut banned: Vec<_> = response.bans.iter().map(|b| b.champion_id).collect();
    banned.sort_unstable();
    assert_eq!(banned, [1, 3]);
    assert!(response.bans.iter().all(|b| b.banned_matches == 20));
    let meta = serde_json::to_value(response.meta).unwrap();
    assert_eq!(meta["filters"], source["filters"]);
    assert_eq!(meta["coverage"], json!([source["coverage"][0].clone()]));
    assert_eq!(meta["min_games"], 100);
    assert_eq!(meta["rank_max_age_hours"], 168);
    assert_eq!(
        meta["rank_scope"],
        "observed_rank_nearest_to_game_start_of_same_ranked_queue"
    );
    assert_ne!(meta["source_snapshot_at"], meta["published_at"]);
    assert!(!meta["source_snapshot_at"].as_str().unwrap().is_empty());
    db.cleanup().await;
}

#[tokio::test]
async fn tierlist_conserve_les_valeurs_nulles_et_ne_somme_pas_all_avec_gold() {
    let db = db_or_skip!();
    publish(db.storage.pool(), report()).await;
    let response = tierlist(db.storage.pool(), query()).await.unwrap();
    assert_eq!(
        response
            .entries
            .iter()
            .map(|e| e.key.champion_id)
            .collect::<Vec<_>>(),
        [2, 3, 1, 4]
    );
    let insufficient = response.entries.last().unwrap();
    assert_eq!(insufficient.games, 3);
    assert_eq!(insufficient.wins, 1);
    assert_eq!(insufficient.population, 500);
    assert_eq!(insufficient.position, None);
    assert_eq!(insufficient.tier, None);
    assert_eq!(insufficient.win_rate, None);
    assert_eq!(insufficient.pick_rate, None);
    assert_eq!(insufficient.win_rate_lower_bound, None);
    let gold = tierlist(
        db.storage.pool(),
        StatsQuery {
            rank: "GOLD".into(),
            ..query()
        },
    )
    .await
    .unwrap();
    assert_eq!(gold.total, 1);
    assert_eq!(gold.entries[0].games, 200);
    assert_eq!(gold.entries[0].key.rank, "GOLD");
    let beyond = tierlist(
        db.storage.pool(),
        StatsQuery {
            offset: 20,
            ..query()
        },
    )
    .await
    .unwrap();
    assert_eq!(beyond.total, 4);
    assert!(beyond.entries.is_empty());
    assert!(beyond.bans.is_empty());
    db.cleanup().await;
}

#[tokio::test]
async fn builds_garde_le_champion_la_population_et_les_effectifs_avant_pagination() {
    let db = db_or_skip!();
    let source = report();
    publish(db.storage.pool(), source.clone()).await;
    let complete = builds(db.storage.pool(), query(), 1).await.unwrap();
    assert_eq!(complete.total, 3);
    assert_eq!(complete.builds.len(), 3);
    assert_eq!(complete.summary.as_ref().unwrap().games, 100);
    assert_eq!(complete.summary.as_ref().unwrap().population, 500);
    assert!(complete
        .builds
        .iter()
        .all(|b| b.population == 1000 && b.key.champion_id == 1 && b.key.rank == "ALL"));
    assert_eq!(complete.skill_levels.len(), 1);
    assert_eq!(complete.skill_levels[0].mean_timestamp_ms, 32_000.0);
    assert_eq!(complete.item_events.len(), 1);
    assert_eq!(complete.item_events[0].events, 42);
    let page_query = StatsQuery {
        offset: 1,
        limit: 1,
        ..query()
    };
    let page = builds(db.storage.pool(), page_query.clone(), 1)
        .await
        .unwrap();
    assert_eq!(page.query, page_query);
    assert_eq!(page.champion_id, 1);
    assert_eq!(page.total, complete.total);
    assert_eq!(page.builds, complete.builds[1..2]);
    assert_eq!(page.skill_levels, complete.skill_levels);
    assert_eq!(page.item_events, complete.item_events);
    assert_eq!(page.summary, complete.summary);
    assert_eq!(page.max_build_variants_per_category, 20);
    assert_eq!(page.omitted_build_variants, 7);
    let meta = serde_json::to_value(page.meta).unwrap();
    assert_eq!(meta["filters"], source["filters"]);
    assert_eq!(meta["coverage"], json!([source["coverage"][0].clone()]));
    db.cleanup().await;
}

#[tokio::test]
async fn requetes_invalides_sont_rejetees_avant_la_lecture_du_snapshot() {
    let db = db_or_skip!();
    let base = query();
    let invalid = [
        StatsQuery {
            patch: "16.19.1".into(),
            ..base.clone()
        },
        StatsQuery {
            platform: "EUROPE".into(),
            ..base.clone()
        },
        StatsQuery {
            queue: 0,
            ..base.clone()
        },
        StatsQuery {
            role: "MID".into(),
            ..base.clone()
        },
        StatsQuery {
            rank: "FAKE".into(),
            ..base.clone()
        },
        StatsQuery {
            offset: 10_001,
            ..base.clone()
        },
        StatsQuery {
            limit: 0,
            ..base.clone()
        },
        StatsQuery { limit: 201, ..base },
    ];
    for query in invalid {
        assert_eq!(
            tierlist(db.storage.pool(), query.clone()).await.err(),
            Some(ApiError::InvalidRequest)
        );
        assert_eq!(
            builds(db.storage.pool(), query, 1).await.err(),
            Some(ApiError::InvalidRequest)
        );
    }
    assert_eq!(
        builds(db.storage.pool(), query(), 0).await.err(),
        Some(ApiError::InvalidRequest)
    );
    db.cleanup().await;
}

#[tokio::test]
async fn builds_publie_les_etapes_d_achat_et_le_catalogue_du_patch() {
    let db = db_or_skip!();
    let mut source = report();
    let stage = |category: &str, selection: Value| {
        json!({
            "patch":"16.19", "platform_id":"EUW1", "queue_id":420,
            "role":"TOP", "rank":"ALL", "champion_id":1,
            "category":category, "selection":selection, "games":150,
            "wins":90, "performance_available":true, "population":600,
            "pick_rate":25.0, "win_rate":60.0, "win_rate_lower_bound":52.0
        })
    };
    let builds_list = source["builds"].as_array_mut().unwrap();
    builds_list.push(stage("core", json!([6672, 3031, 3089])));
    builds_list.push(stage("starter", json!([1055, 2003])));
    source["build_stage_method"] = json!("catalog #61 of the game patch");
    source["item_catalogs"] = json!([
        {"patch":"16.18", "version":"16.18.1"},
        {"patch":"16.19", "version":"16.19.1"}
    ]);
    publish(db.storage.pool(), source).await;
    let response = builds(db.storage.pool(), query(), 1).await.unwrap();
    assert_eq!(response.total, 5);
    let core = response
        .builds
        .iter()
        .find(|b| b.category == "core")
        .unwrap();
    // L'ordre du core est une donnée : il n'est jamais trié par la route.
    assert_eq!(core.selection, vec![6672, 3031, 3089]);
    assert_eq!(core.win_rate_lower_bound, Some(52.0));
    assert_eq!(response.item_catalog_version.as_deref(), Some("16.19.1"));
    assert_eq!(response.build_stage_method, "catalog #61 of the game patch");
    // Un instantané antérieur aux étapes reste lisible, sans catalogue annoncé.
    sqlx::query("DELETE FROM champion_stats_snapshot WHERE id=1")
        .execute(db.storage.pool())
        .await
        .unwrap();
    publish(db.storage.pool(), report()).await;
    let legacy = builds(db.storage.pool(), query(), 1).await.unwrap();
    assert_eq!(legacy.item_catalog_version, None);
    assert!(legacy
        .builds
        .iter()
        .all(|b| b.win_rate_lower_bound.is_none()));
    db.cleanup().await;
}
