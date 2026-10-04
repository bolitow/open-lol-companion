//! Recette des agrégats avec PostgreSQL réel et des parties synthétiques.
mod common;

use std::time::Duration;

use common::TestDb;
use olc_collector::aggregation::{
    recalculate, recalculate_filtered, recalculate_with_quality, AggregationError,
    AggregationOptions, QualityThresholds, DEFAULT_RANK_MAX_AGE_HOURS,
};
use olc_collector::config::RunParams;
use olc_collector::model::fixtures::match_detail;
use serde_json::{json, Value};
use sqlx::Row;

macro_rules! db_or_skip {
    () => {
        match TestDb::create().await {
            Some(db) => db,
            None => return,
        }
    };
}

async fn run(db: &TestDb) -> i64 {
    db.storage
        .create_run(&RunParams::default(), 1_000_000, 2_000_000)
        .await
        .unwrap()
}

async fn insert_match(db: &TestDb, run_id: i64, id: &str) {
    sqlx::query("INSERT INTO matches (match_id, platform_id, queue_id, game_version, patch,
        game_start, game_duration_s, is_remake, detail, first_run_id)
        VALUES ($1, 'EUW1', 420, '15.19.715.1234', '15.19', to_timestamp(1000), 1800, false, $2, $3)")
        .bind(id).bind(match_detail(id, "EUW1", 420, 1_000_000)).bind(run_id)
        .execute(db.storage.pool()).await.unwrap();
}

/// Une partie classée valide, une très courte (durée stockée), une avec un départ précoce
/// et une avec un participant `wasAfk`.
async fn insert_quality_matches(db: &TestDb, run_id: i64) {
    for id in ["EUW1_ok", "EUW1_short", "EUW1_left", "EUW1_afk"] {
        insert_match(db, run_id, id).await;
    }
    sqlx::query("UPDATE matches SET game_duration_s=200 WHERE match_id='EUW1_short'")
        .execute(db.storage.pool())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE matches SET detail=jsonb_set(detail,'{info,participants,3,timePlayed}','1000')
        WHERE match_id='EUW1_left'",
    )
    .execute(db.storage.pool())
    .await
    .unwrap();
    sqlx::query(
        "UPDATE matches SET detail=jsonb_set(detail,'{info,participants,7,wasAfk}','true')
        WHERE match_id='EUW1_afk'",
    )
    .execute(db.storage.pool())
    .await
    .unwrap();
}

async fn published(db: &TestDb) -> Value {
    let rows = sqlx::query("SELECT s.storage_version,s.report,c.section,c.items
        FROM champion_stats_snapshot s LEFT JOIN champion_stats_snapshot_chunks c
        ON c.snapshot_id=s.id AND s.storage_version=2 WHERE s.id=1 ORDER BY c.section,c.chunk_index")
        .fetch_all(db.storage.pool())
        .await
        .unwrap();
    let mut report: Value = rows[0].get("report");
    if rows[0].get::<i16, _>("storage_version") == 2 {
        for section in [
            "coverage",
            "groups",
            "bans",
            "builds",
            "skill_levels",
            "item_events",
            "performance",
            "matchups",
        ] {
            report[section] = json!([]);
        }
        for row in rows {
            if let Some(section) = row.get::<Option<String>, _>("section") {
                let items: Value = row.get("items");
                report[&section]
                    .as_array_mut()
                    .unwrap()
                    .extend(items.as_array().unwrap().iter().cloned());
            }
        }
    }
    report
}

#[tokio::test]
async fn aggregation_stocke_ses_listes_en_morceaux_sans_perdre_de_donnees() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_chunks").await;
    let teams = json!([
        {"teamId":100,"bans":(1..=5).map(|turn|json!({"championId":100+turn,"pickTurn":turn})).collect::<Vec<_>>()},
        {"teamId":200,"bans":(6..=10).map(|turn|json!({"championId":100+turn,"pickTurn":turn})).collect::<Vec<_>>()}
    ]);
    sqlx::query("UPDATE matches SET detail=jsonb_set(detail,'{info,teams}',$1) WHERE match_id='EUW1_chunks'")
        .bind(teams).execute(db.storage.pool()).await.unwrap();
    let events: Vec<_> = (1..=700)
        .map(|item| {
            json!({
                "type":"ITEM_PURCHASED","timestamp":item,"participantId":1,"itemId":item
            })
        })
        .chain(std::iter::once(
            json!({"type":"SKILL_LEVEL_UP","timestamp":1000,
        "participantId":1,"skillSlot":2,"levelUpType":"NORMAL"}),
        ))
        .collect();
    let timeline = json!({"metadata":{"matchId":"EUW1_chunks"},"info":{
        "participants":(1..=10).map(|id|json!({"participantId":id})).collect::<Vec<_>>(),
        "frames":[{"timestamp":0,"events":events}]}});
    sqlx::query("INSERT INTO match_timelines(match_id,status,timeline) VALUES ('EUW1_chunks','available',$1)")
        .bind(timeline).execute(db.storage.pool()).await.unwrap();
    let report = recalculate(&db.storage, 1).await.unwrap();
    assert_eq!(
        db.scalar("SELECT storage_version::bigint FROM champion_stats_snapshot WHERE id=1")
            .await,
        2
    );
    assert!(
        db.scalar("SELECT count(*) FROM champion_stats_snapshot_chunks")
            .await
            > 0
    );
    assert!(
        db.scalar(
            "SELECT count(*) FROM champion_stats_snapshot_chunks WHERE section='item_events'"
        )
        .await
            > 1
    );
    // Moyennes de performance (#100) : section propre, publiée dans la même transaction.
    assert!(
        db.scalar(
            "SELECT count(*) FROM champion_stats_snapshot_chunks WHERE section='performance'"
        )
        .await
            > 0
    );
    assert!(!report.performance_method.is_empty());
    // Matchups de lane (#123) : section propre, publiée dans la même transaction.
    assert!(
        db.scalar("SELECT count(*) FROM champion_stats_snapshot_chunks WHERE section='matchups'")
            .await
            > 0
    );
    assert!(!report.matchup_method.is_empty());
    let complete = serde_json::to_value(report).unwrap();
    for section in [
        "coverage",
        "groups",
        "bans",
        "builds",
        "skill_levels",
        "item_events",
        "performance",
        "matchups",
    ] {
        assert!(
            !complete[section].as_array().unwrap().is_empty(),
            "{section}"
        );
    }
    assert_eq!(published(&db).await, complete);
    // Le binaire historique ne sait pas désigner la version de stockage : il doit
    // échouer plutôt que d'associer de nouvelles métadonnées aux anciens morceaux.
    assert!(
        sqlx::query("UPDATE champion_stats_snapshot SET report=$1 WHERE id=1")
            .bind(&complete)
            .execute(db.storage.pool())
            .await
            .is_err()
    );
    assert_eq!(published(&db).await, complete);
    db.cleanup().await;
}

#[tokio::test]
async fn aggregation_recalcule_sans_doubler_les_parties_liees_a_deux_runs() {
    let db = db_or_skip!();
    let first = run(&db).await;
    let second = run(&db).await;
    insert_match(&db, first, "EUW1_1").await;
    for run_id in [first, second] {
        sqlx::query(
            "INSERT INTO run_matches (run_id, match_id, seed_puuid, already_present)
            VALUES ($1, 'EUW1_1', 'synthetic-seed', false)",
        )
        .bind(run_id)
        .execute(db.storage.pool())
        .await
        .unwrap();
    }
    // Aucune timeline ni seed requise : seul le détail compte.
    let first_report = recalculate(&db.storage, 1).await.unwrap();
    assert_eq!(first_report.source_matches, 1);
    assert_eq!(first_report.groups[0].games, 1);
    assert_eq!(
        published(&db).await,
        serde_json::to_value(&first_report).unwrap()
    );
    assert_eq!(recalculate(&db.storage, 1).await.unwrap(), first_report);
    assert_eq!(
        db.scalar("SELECT count(*) FROM champion_stats_snapshot")
            .await,
        1
    );
    insert_match(&db, first, "EUW1_2").await;
    let updated = recalculate(&db.storage, 3).await.unwrap();
    assert_eq!(updated.included_matches, 2);
    assert_eq!(updated.groups[0].games, 2);
    assert!(updated.groups.iter().all(|g| g.position.is_none()));
    // Retirer toutes les parties éligibles remplace aussi les anciens groupes.
    sqlx::query("UPDATE matches SET is_remake = true")
        .execute(db.storage.pool())
        .await
        .unwrap();
    let empty = recalculate(&db.storage, 1).await.unwrap();
    assert_eq!(empty.exclusions.get("remake"), Some(&2));
    assert!(empty.groups.is_empty());
    assert_eq!(published(&db).await["groups"], json!([]));
    assert_eq!(
        db.scalar("SELECT count(*) FROM champion_stats_snapshot_chunks")
            .await,
        0
    );
    db.cleanup().await;
}

#[tokio::test]
async fn une_insertion_partielle_de_morceaux_conserve_l_ancienne_publication() {
    let db = db_or_skip!();
    let id = run(&db).await;
    insert_match(&db, id, "EUW1_before").await;
    recalculate(&db.storage, 1).await.unwrap();
    let before = published(&db).await;
    insert_match(&db, id, "EUW1_after").await;
    sqlx::raw_sql(
        "CREATE FUNCTION reject_chunk() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.section='groups' THEN RAISE EXCEPTION 'synthetic chunk failure'; END IF;
        RETURN NEW; END $$;
        CREATE TRIGGER reject_chunk BEFORE INSERT ON champion_stats_snapshot_chunks
        FOR EACH ROW EXECUTE FUNCTION reject_chunk();",
    )
    .execute(db.storage.pool())
    .await
    .unwrap();
    assert!(matches!(
        recalculate(&db.storage, 1).await,
        Err(AggregationError::Database(_))
    ));
    assert_eq!(published(&db).await, before);
    db.cleanup().await;
}

#[tokio::test]
async fn aggregation_annule_la_publication_en_cas_d_echec_au_commit() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_1").await;
    recalculate(&db.storage, 1).await.unwrap();
    let before = published(&db).await;
    insert_match(&db, run_id, "EUW1_2").await;
    sqlx::raw_sql(
        "CREATE FUNCTION reject_snapshot() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN RAISE EXCEPTION 'synthetic-private-detail'; END $$;
        CREATE CONSTRAINT TRIGGER reject_snapshot AFTER INSERT OR UPDATE ON champion_stats_snapshot
        DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_snapshot();",
    )
    .execute(db.storage.pool())
    .await
    .unwrap();
    let error = recalculate(&db.storage, 1).await.unwrap_err();
    assert!(matches!(error, AggregationError::Database(_)));
    assert!(!error.to_string().contains("synthetic-private-detail"));
    assert_eq!(published(&db).await, before);
    sqlx::query("DROP TRIGGER reject_snapshot ON champion_stats_snapshot")
        .execute(db.storage.pool())
        .await
        .unwrap();
    assert_eq!(
        recalculate(&db.storage, 1).await.unwrap().included_matches,
        2
    );
    db.cleanup().await;
}

async fn wait_for_publication_lock(db: &TestDb) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM pg_stat_activity
                WHERE datname = current_database() AND wait_event_type = 'Lock'
                AND query LIKE 'INSERT INTO champion_stats_snapshot%')",
            )
            .fetch_one(db.storage.pool())
            .await
            .unwrap();
            if waiting {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("le calcul atteint la publication bloquée");
}

#[tokio::test]
async fn aggregation_garde_un_instantane_stable_et_refuse_un_second_calcul() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    for n in 0..105 {
        insert_match(&db, run_id, &format!("EUW1_{n}")).await;
    }
    recalculate(&db.storage, 1).await.unwrap();
    let before = published(&db).await;
    // Bloquer la dernière écriture permet d'inspecter la transaction avant publication.
    let mut blocker = db.storage.pool().begin().await.unwrap();
    sqlx::query("SELECT id FROM champion_stats_snapshot WHERE id = 1 FOR UPDATE")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let other = db.separate_storage().await;
    let writer = other.clone();
    let task = tokio::spawn(async move { recalculate(&writer, 1).await });
    wait_for_publication_lock(&db).await;
    assert!(matches!(
        recalculate(&db.storage, 1).await,
        Err(AggregationError::Busy)
    ));
    insert_match(&db, run_id, "EUW1_new").await;
    assert_eq!(published(&db).await, before);
    blocker.commit().await.unwrap();
    let report = task.await.unwrap().unwrap();
    assert_eq!(report.included_matches, 105);
    assert!(report.groups.iter().all(|g| g.games == 105));
    assert_eq!(
        recalculate(&db.storage, 1).await.unwrap().included_matches,
        106
    );
    other.pool().close().await;
    db.cleanup().await;
}

#[tokio::test]
async fn aggregation_refuse_un_seuil_nul_sans_remplacer_la_publication() {
    let db = db_or_skip!();
    recalculate(&db.storage, 1).await.unwrap();
    let before = published(&db).await;
    assert!(matches!(
        recalculate(&db.storage, 0).await,
        Err(AggregationError::InvalidThreshold)
    ));
    assert_eq!(published(&db).await, before);
    db.cleanup().await;
}

#[tokio::test]
async fn aggregation_annule_avant_commit_et_libere_le_verrou() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_1").await;
    recalculate(&db.storage, 1).await.unwrap();
    let before = published(&db).await;
    insert_match(&db, run_id, "EUW1_2").await;
    let mut blocker = db.storage.pool().begin().await.unwrap();
    sqlx::query("SELECT id FROM champion_stats_snapshot WHERE id = 1 FOR UPDATE")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let other = db.separate_storage().await;
    let writer = other.clone();
    let task = tokio::spawn(async move { recalculate(&writer, 1).await });
    wait_for_publication_lock(&db).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    blocker.commit().await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), other.pool().close())
        .await
        .unwrap();
    assert_eq!(published(&db).await, before);
    assert_eq!(
        recalculate(&db.storage, 1).await.unwrap().included_matches,
        2
    );
    db.cleanup().await;
}

#[tokio::test]
async fn aggregation_n_ecrase_pas_une_publication_plus_recente_que_son_instantane() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_1").await;
    recalculate(&db.storage, 1).await.unwrap();
    let mut blocker = db.storage.pool().begin().await.unwrap();
    sqlx::query("SELECT id FROM champion_stats_snapshot WHERE id = 1 FOR UPDATE")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let other = db.separate_storage().await;
    let writer = other.clone();
    let task = tokio::spawn(async move { recalculate(&writer, 2).await });
    wait_for_publication_lock(&db).await;
    // Simule une publication dont le commit arrive après la prise de l'instantané.
    sqlx::query(
        "UPDATE champion_stats_snapshot SET report = jsonb_set(report, '{min_games}', '123')",
    )
    .execute(&mut *blocker)
    .await
    .unwrap();
    blocker.commit().await.unwrap();
    assert!(matches!(
        task.await.unwrap(),
        Err(AggregationError::Database(_))
    ));
    assert_eq!(published(&db).await["min_games"], 123);
    other.pool().close().await;
    db.cleanup().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn aggregation_cli_publie_du_json_sans_cle_riot_et_refuse_un_seuil_nul() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_cli").await;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_olc-collector"))
        .args([
            "aggregate",
            "--all-stored",
            "--min-games",
            "1",
            "--rank-max-age-hours",
            "48",
            "--json",
        ])
        .env("DATABASE_URL", db.database_url())
        .env("RIOT_API_KEY", "")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["included_matches"], 1);
    assert_eq!(report["groups"][0]["games"], 1);
    assert_eq!(report["rank_max_age_hours"], 48);
    assert_eq!(published(&db).await, report);
    let invalid_age = std::process::Command::new(env!("CARGO_BIN_EXE_olc-collector"))
        .args(["aggregate", "--rank-max-age-hours", "0"])
        .env("DATABASE_URL", "postgres://invalid.invalid/test")
        .env("RIOT_API_KEY", "")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert_eq!(invalid_age.status.code(), Some(2));
    let invalid = std::process::Command::new(env!("CARGO_BIN_EXE_olc-collector"))
        .args(["aggregate", "--min-games", "0"])
        .env("DATABASE_URL", "postgres://invalid.invalid/test")
        .env("RIOT_API_KEY", "")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("--min-games"));
    assert_eq!(published(&db).await, report);
    db.cleanup().await;
}

#[tokio::test]
async fn aggregation_filtre_et_lit_les_observations_proches_de_la_partie_et_la_timeline() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_recent").await;
    insert_match(&db, run_id, "EUW1_old").await;
    sqlx::query("UPDATE matches SET patch='15.18',game_version='15.18.1',detail=jsonb_set(detail,'{info,gameVersion}','\"15.18.1\"') WHERE match_id='EUW1_old'")
        .execute(db.storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO participant_rank_observations(platform_id,puuid,queue_id,tier,division,league_points,status,observed_at) VALUES ('EUW1','fake-puuid-0',420,'GOLD','I',50,'ranked',to_timestamp(1000)+interval '2 hours')")
        .execute(db.storage.pool()).await.unwrap();
    // Une observation trop éloignée de la partie ne lui attribue jamais de rang.
    sqlx::query("INSERT INTO participant_rank_observations(platform_id,puuid,queue_id,tier,division,league_points,status,observed_at) VALUES ('EUW1','fake-puuid-1',420,'DIAMOND','I',50,'ranked',to_timestamp(1000)+interval '169 hours')")
        .execute(db.storage.pool()).await.unwrap();
    let timeline = json!({"metadata":{"matchId":"EUW1_recent"},"info":{"participants":(1..=10).map(|id|json!({"participantId":id})).collect::<Vec<_>>(),"frames":[{"timestamp":0,"events":[
        {"type":"SKILL_LEVEL_UP","timestamp":1000,"participantId":1,"skillSlot":2,"levelUpType":"NORMAL"},
        {"type":"ITEM_PURCHASED","timestamp":2000,"participantId":1,"itemId":1055}
    ]}]}});
    sqlx::query("INSERT INTO match_timelines(match_id,status,timeline) VALUES ('EUW1_recent','available',$1)")
        .bind(timeline).execute(db.storage.pool()).await.unwrap();
    let filters = AggregationOptions {
        patches: vec!["15.19".into()],
        platforms: vec!["EUW1".into()],
        queues: vec![420],
        start_ms: Some(1_000_000),
        end_ms: Some(2_000_000),
    };
    let report = recalculate_filtered(&db.storage, 1, DEFAULT_RANK_MAX_AGE_HOURS, &filters)
        .await
        .unwrap();
    assert_eq!(report.included_matches, 1);
    assert!(report
        .groups
        .iter()
        .any(|g| g.key.champion_id == 1 && g.key.rank == "GOLD"));
    assert!(!report.groups.iter().any(|g| g.key.rank == "DIAMOND"));
    assert!(report
        .builds
        .iter()
        .any(|b| b.category == "skill_order" && b.selection == vec![2]));
    assert_eq!(report.coverage[0].counts.ranked_participations, 1);
    assert_eq!(report.filters, filters);
    assert_eq!(published(&db).await, serde_json::to_value(&report).unwrap());
    assert_eq!(
        recalculate_filtered(&db.storage, 1, DEFAULT_RANK_MAX_AGE_HOURS, &filters)
            .await
            .unwrap(),
        report
    );
    db.cleanup().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cli_selection_explicite_du_patch_et_de_la_file() {
    let db = db_or_skip!();
    let id = run(&db).await;
    insert_match(&db, id, "EUW1_cli_filter").await;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_olc-collector"))
        .args([
            "aggregate",
            "--patches",
            "15.19",
            "--platforms",
            "euw1",
            "--queues",
            "420",
            "--min-games",
            "1",
            "--json",
        ])
        .env("DATABASE_URL", db.database_url())
        .env("RIOT_API_KEY", "")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let data: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(data["filters"]["patches"], json!(["15.19"]));
    assert_eq!(data["included_matches"], 1);
    db.cleanup().await;
}

#[tokio::test]
async fn le_rang_le_plus_proche_de_la_partie_prime_et_les_files_sont_separees() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_rank_history").await;
    // La plus récente n'est pas la plus proche ; l'autre file ne compte jamais.
    sqlx::raw_sql("INSERT INTO participant_rank_observations(platform_id,puuid,queue_id,tier,division,league_points,status,observed_at) VALUES
        ('EUW1','fake-puuid-0',420,'GOLD','I',10,'ranked',to_timestamp(1000)+interval '30 hours'),
        ('EUW1','fake-puuid-0',420,'SILVER','I',10,'ranked',to_timestamp(1000)-interval '5 hours'),
        ('EUW1','fake-puuid-0',440,'DIAMOND','I',10,'ranked',to_timestamp(1000)+interval '1 hour'),
        ('EUW1','fake-puuid-0',420,NULL,NULL,NULL,'unranked',to_timestamp(1000)+interval '2 hours');")
        .execute(db.storage.pool()).await.unwrap();
    let r = recalculate(&db.storage, 1).await.unwrap();
    assert!(!r
        .groups
        .iter()
        .any(|g| ["GOLD", "SILVER", "DIAMOND"].contains(&g.key.rank.as_str())));
    assert!(r
        .groups
        .iter()
        .any(|g| g.key.champion_id == 1 && g.key.rank == "UNRANKED"));
    db.cleanup().await;
}

#[tokio::test]
async fn un_filtre_de_plateforme_invalide_preserve_le_rapport_publie() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_valid_platform").await;
    recalculate(&db.storage, 1).await.unwrap();
    let before = published(&db).await;
    let filters = AggregationOptions {
        platforms: vec!["EUW".into()],
        ..Default::default()
    };
    assert!(matches!(
        recalculate_filtered(&db.storage, 1, DEFAULT_RANK_MAX_AGE_HOURS, &filters).await,
        Err(AggregationError::InvalidFilters)
    ));
    assert_eq!(published(&db).await, before);
    db.cleanup().await;
}

#[tokio::test]
async fn le_rang_reste_fige_a_la_partie_quel_que_soit_l_heure_du_calcul() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_frozen_rank").await;
    sqlx::query("UPDATE matches SET game_start=now()-interval '30 days'")
        .execute(db.storage.pool())
        .await
        .unwrap();
    // Observée il y a 30 jours, deux heures après la partie : le rang reste valable.
    // Observée il y a une heure, un mois après la partie : il ne lui est pas attribué.
    sqlx::raw_sql("INSERT INTO participant_rank_observations(platform_id,puuid,queue_id,tier,division,league_points,status,observed_at)
        SELECT 'EUW1','fake-puuid-0',420,'GOLD','I',10,'ranked',game_start+interval '2 hours' FROM matches
        UNION ALL SELECT 'EUW1','fake-puuid-1',420,'DIAMOND','I',10,'ranked',now()-interval '1 hour';")
        .execute(db.storage.pool()).await.unwrap();
    let r = recalculate(&db.storage, 1).await.unwrap();
    assert!(r
        .groups
        .iter()
        .any(|g| g.key.champion_id == 1 && g.key.rank == "GOLD"));
    assert!(!r.groups.iter().any(|g| g.key.rank == "DIAMOND"));
    let c = &r.coverage[0].counts;
    assert_eq!(
        (c.ranked_participations, c.unknown_rank_participations),
        (1, 9)
    );
    assert_eq!(c.unknown_rank_rate, Some(90.0));
    assert_eq!(
        (c.rank_gap_median_hours, c.rank_gap_max_hours),
        (Some(2.0), Some(2.0))
    );
    assert_eq!(r.rank_max_age_hours, DEFAULT_RANK_MAX_AGE_HOURS);
    assert_eq!(recalculate(&db.storage, 1).await.unwrap(), r);
    let strict = recalculate_filtered(&db.storage, 1, 1, &AggregationOptions::default())
        .await
        .unwrap();
    assert!(!strict.groups.iter().any(|g| g.key.rank == "GOLD"));
    assert_eq!(strict.rank_max_age_hours, 1);
    assert!(matches!(
        recalculate_filtered(&db.storage, 1, 0, &AggregationOptions::default()).await,
        Err(AggregationError::InvalidRankMaxAge)
    ));
    db.cleanup().await;
}

#[tokio::test]
async fn aggregation_exclut_les_parties_classees_courtes_ou_avec_depart_precoce() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_quality_matches(&db, run_id).await;
    let report = recalculate(&db.storage, 1).await.unwrap();
    assert_eq!((report.source_matches, report.included_matches), (4, 1));
    assert_eq!(report.exclusions.get("short_game"), Some(&1));
    assert_eq!(report.exclusions.get("early_departure"), Some(&1));
    assert_eq!(report.exclusions.get("afk"), Some(&1));
    assert_eq!(report.coverage[0].counts.matches, 1);
    let snapshot = published(&db).await;
    assert_eq!(snapshot["min_game_duration_s"], 300);
    assert_eq!(snapshot["min_played_percent"], 80);
    assert_eq!(snapshot["exclude_afk"], true);
    assert_eq!(
        snapshot["exclusions"],
        json!({"short_game": 1, "early_departure": 1, "afk": 1})
    );
    // Seuils assouplis : 1000 s sur 1800 (56 %) passe à 50 %, 200 s reste court à 300 s.
    let relaxed = recalculate_with_quality(
        &db.storage,
        1,
        DEFAULT_RANK_MAX_AGE_HOURS,
        &AggregationOptions::default(),
        &QualityThresholds {
            min_game_duration_s: 300,
            min_played_percent: 50,
            exclude_afk: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(relaxed.included_matches, 2);
    assert_eq!(relaxed.exclusions.get("early_departure"), None);
    // AFK conservé seul : la partie avec `wasAfk` revient, les autres motifs restent actifs.
    let keep_afk = recalculate_with_quality(
        &db.storage,
        1,
        DEFAULT_RANK_MAX_AGE_HOURS,
        &AggregationOptions::default(),
        &QualityThresholds {
            exclude_afk: false,
            ..QualityThresholds::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(keep_afk.included_matches, 2);
    assert_eq!(keep_afk.exclusions.get("afk"), None);
    assert!(!keep_afk.exclude_afk);
    assert_eq!(published(&db).await["exclude_afk"], false);
    // Contrôles désactivés : toutes les parties valides sont conservées.
    let off = recalculate_with_quality(
        &db.storage,
        1,
        DEFAULT_RANK_MAX_AGE_HOURS,
        &AggregationOptions::default(),
        &QualityThresholds {
            min_game_duration_s: 0,
            min_played_percent: 0,
            exclude_afk: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(off.included_matches, 4);
    assert!(off.exclusions.is_empty());
    let invalid = recalculate_with_quality(
        &db.storage,
        1,
        DEFAULT_RANK_MAX_AGE_HOURS,
        &AggregationOptions::default(),
        &QualityThresholds {
            min_game_duration_s: 300,
            min_played_percent: 101,
            exclude_afk: true,
        },
    )
    .await;
    assert!(matches!(
        invalid,
        Err(AggregationError::InvalidMinPlayedPercent)
    ));
    db.cleanup().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn aggregation_cli_configure_les_seuils_de_qualite() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_quality_matches(&db, run_id).await;
    let aggregate = |extra: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_olc-collector"))
            .args(["aggregate", "--all-stored", "--min-games", "1", "--json"])
            .args(extra)
            .env("DATABASE_URL", db.database_url())
            .env("RIOT_API_KEY", "")
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap()
    };
    let default = aggregate(&[]);
    assert!(default.status.success());
    let report: Value = serde_json::from_slice(&default.stdout).unwrap();
    assert_eq!(report["included_matches"], 1);
    assert_eq!(report["min_game_duration_s"], 300);
    assert_eq!(report["exclude_afk"], true);
    assert_eq!(report["exclusions"]["afk"], 1);
    let keep_afk = aggregate(&["--keep-afk"]);
    assert!(keep_afk.status.success());
    let report: Value = serde_json::from_slice(&keep_afk.stdout).unwrap();
    assert_eq!(report["included_matches"], 2);
    assert_eq!(report["exclude_afk"], false);
    let lenient = aggregate(&[
        "--min-game-duration-s",
        "0",
        "--min-played-percent",
        "0",
        "--keep-afk",
    ]);
    assert!(lenient.status.success());
    let report: Value = serde_json::from_slice(&lenient.stdout).unwrap();
    assert_eq!(report["included_matches"], 4);
    assert_eq!(report["min_played_percent"], 0);
    for bad in [
        ["--min-game-duration-s", "901"],
        ["--min-played-percent", "101"],
    ] {
        assert_eq!(aggregate(&bad).status.code(), Some(2), "{bad:?}");
    }
    db.cleanup().await;
}

async fn publish_item_catalog(
    db: &TestDb,
    publication: &str,
    version: &str,
    items: &[(&str, Value)],
) {
    sqlx::query("INSERT INTO game_catalog_publications(id,version,normalizer_version,manifest) VALUES ($1,$2,1,'{}')")
        .bind(publication).bind(version).execute(db.storage.pool()).await.unwrap();
    for (id, fields) in items {
        let mut normalized = serde_json::Map::new();
        for (name, value) in fields.as_object().unwrap() {
            normalized.insert(
                name.clone(),
                json!({"value":value,"unit":null,"status":"verified","sources":[]}),
            );
        }
        // Une fiche FR identique ne doit pas être comptée deux fois.
        for locale in ["en_US", "fr_FR"] {
            sqlx::query("INSERT INTO game_catalog_entries(publication_id,kind,id,namespace,locale,name,data) VALUES ($1,'item',$2,'standard',$3,$2,$4)")
                .bind(publication).bind(id).bind(locale).bind(json!({"fields": normalized}))
                .execute(db.storage.pool()).await.unwrap();
        }
    }
    sqlx::query("INSERT INTO game_catalog_current(version,publication_id) VALUES ($1,$2)")
        .bind(version)
        .bind(publication)
        .execute(db.storage.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn aggregation_joint_le_catalogue_du_patch_pour_les_etapes_d_achat() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_stages").await;
    let completed =
        |price: u32| json!({"price_total":price,"purchasable":true,"categories":["Damage"]});
    publish_item_catalog(&db, "pub-15-19-1", "15.19.1", &[
        ("1055", json!({"price_total":450,"purchasable":true,"categories":["Lane"]})),
        ("3006", json!({"price_total":1100,"purchasable":true,"categories":["Boots"],"builds_from":["1001"]})),
        ("3031", completed(3500)), ("6672", completed(3000)), ("3072", completed(3400)),
    ]).await;
    // Une autre version du même patch, plus ancienne, ne doit pas être retenue.
    publish_item_catalog(&db, "pub-15-19-0", "15.19.0", &[("3031", completed(3500))]).await;
    let events: Vec<_> = [
        (1055, 1_000),
        (3006, 200_000),
        (3031, 600_000),
        (6672, 900_000),
        (3072, 1_200_000),
    ]
    .iter()
    .map(|(id, at)| json!({"type":"ITEM_PURCHASED","participantId":1,"timestamp":at,"itemId":id}))
    .collect();
    let timeline = json!({"metadata":{"matchId":"EUW1_stages"},"info":{"participants":(1..=10).map(|id|json!({"participantId":id})).collect::<Vec<_>>(),"frames":[{"timestamp":0,"events":events}]}});
    sqlx::query("INSERT INTO match_timelines(match_id,status,timeline) VALUES ('EUW1_stages','available',$1)")
        .bind(timeline).execute(db.storage.pool()).await.unwrap();
    let report = recalculate(&db.storage, 1).await.unwrap();
    let stage = |category: &str| {
        report
            .builds
            .iter()
            .find(|b| b.key.champion_id == 1 && b.key.rank == "ALL" && b.category == category)
            .map(|b| b.selection.clone())
    };
    assert_eq!(stage("starter"), Some(vec![1055]));
    assert_eq!(stage("boots"), Some(vec![3006]));
    assert_eq!(stage("core"), Some(vec![3031, 6672, 3072]));
    assert_eq!(
        serde_json::to_value(&report.item_catalogs).unwrap(),
        json!([{"patch":"15.19","version":"15.19.1"}])
    );
    assert_eq!(report.coverage[0].counts.item_stage_participations, 1);
    let published = published(&db).await;
    assert_eq!(published, serde_json::to_value(&report).unwrap());
    assert_eq!(published["item_catalogs"][0]["version"], "15.19.1");
    db.cleanup().await;
}
