//! Recette des agrégats avec PostgreSQL réel et des parties synthétiques.
mod common;

use std::time::Duration;

use common::TestDb;
use olc_collector::aggregation::{
    recalculate, recalculate_filtered, AggregationError, AggregationOptions,
};
use olc_collector::config::RunParams;
use olc_collector::model::fixtures::match_detail;
use serde_json::{json, Value};

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

async fn published(db: &TestDb) -> Value {
    sqlx::query_scalar("SELECT report FROM champion_stats_snapshot WHERE id = 1")
        .fetch_one(db.storage.pool())
        .await
        .unwrap()
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
        .args(["aggregate", "--all-stored", "--min-games", "1", "--json"])
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
    assert_eq!(published(&db).await, report);
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
async fn aggregation_filtre_et_lit_les_observations_recentes_et_la_timeline() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_recent").await;
    insert_match(&db, run_id, "EUW1_old").await;
    sqlx::query("UPDATE matches SET patch='15.18',game_version='15.18.1',detail=jsonb_set(detail,'{info,gameVersion}','\"15.18.1\"') WHERE match_id='EUW1_old'")
        .execute(db.storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO participant_rank_observations(platform_id,puuid,queue_id,tier,division,league_points,status) VALUES ('EUW1','fake-puuid-0',420,'GOLD','I',50,'ranked')")
        .execute(db.storage.pool()).await.unwrap();
    // Un rang périmé ne devient jamais une observation actuelle.
    sqlx::query("INSERT INTO participant_rank_observations(platform_id,puuid,queue_id,tier,division,league_points,status,observed_at) VALUES ('EUW1','fake-puuid-1',420,'DIAMOND','I',50,'ranked',now()-interval '25 hours')")
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
    let report = recalculate_filtered(&db.storage, 1, &filters)
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
    assert_eq!(
        recalculate_filtered(&db.storage, 1, &filters)
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
async fn le_rang_le_plus_recent_prime_et_les_files_sont_separees() {
    let db = db_or_skip!();
    let run_id = run(&db).await;
    insert_match(&db, run_id, "EUW1_rank_history").await;
    sqlx::raw_sql("INSERT INTO participant_rank_observations(platform_id,puuid,queue_id,tier,division,league_points,status,observed_at) VALUES
        ('EUW1','fake-puuid-0',420,'GOLD','I',10,'ranked',now()-interval '1 hour'),
        ('EUW1','fake-puuid-0',440,'DIAMOND','I',10,'ranked',now()),
        ('EUW1','fake-puuid-0',420,NULL,NULL,NULL,'unranked',now());")
        .execute(db.storage.pool()).await.unwrap();
    let r = recalculate(&db.storage, 1).await.unwrap();
    assert!(!r
        .groups
        .iter()
        .any(|g| g.key.rank == "GOLD" || g.key.rank == "DIAMOND"));
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
        recalculate_filtered(&db.storage, 1, &filters).await,
        Err(AggregationError::InvalidFilters)
    ));
    assert_eq!(published(&db).await, before);
    db.cleanup().await;
}
