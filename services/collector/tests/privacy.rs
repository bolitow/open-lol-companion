//! Rétention, export et effacement des données personnelles (#99) sur PostgreSQL réel.
mod common;

use common::TestDb;
use olc_collector::aggregation::recalculate;
use olc_collector::config::RunParams;
use olc_collector::model::fixtures::match_detail;
use olc_collector::privacy::{erase_subject, export_subject, purge, PrivacyError, RetentionPolicy};
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

/// Exécution synthétique, rendue inactive depuis `idle_days` jours.
async fn run(db: &TestDb, status: &str, idle_days: i32) -> i64 {
    let id = db
        .storage
        .create_run(&RunParams::default(), 1_000_000, 2_000_000)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE collection_runs SET status = $2, updated_at = now() - make_interval(days => $3)
         WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .bind(idle_days)
    .execute(db.storage.pool())
    .await
    .unwrap();
    id
}

fn timeline(id: &str) -> Value {
    json!({
        "metadata": {"matchId": id, "participants": (0..10).map(|i| format!("fake-puuid-{i}")).collect::<Vec<_>>()},
        "info": {
            "participants": (0..10).map(|i| json!({"participantId": i + 1, "puuid": format!("fake-puuid-{i}")})).collect::<Vec<_>>(),
            "frames": [{"timestamp": 0, "events": [], "participantFrames": {}}]
        }
    })
}

/// Partie et timeline enregistrées il y a `age_days` jours, liées à `run_id`.
async fn stored_match(db: &TestDb, run_id: i64, id: &str, age_days: i32, seed: &str) {
    let pool = db.storage.pool();
    sqlx::query(
        "INSERT INTO matches (match_id, platform_id, queue_id, game_version, patch, game_start,
            game_duration_s, is_remake, detail, first_run_id, fetched_at)
         VALUES ($1, 'EUW1', 420, '15.19.715.1234', '15.19', to_timestamp(1000), 1800, false, $2, $3,
            now() - make_interval(days => $4))",
    )
    .bind(id)
    .bind(match_detail(id, "EUW1", 420, 1_000_000))
    .bind(run_id)
    .bind(age_days)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO match_timelines (match_id, status, timeline, fetched_at)
         VALUES ($1, 'available', $2, now() - make_interval(days => $3))",
    )
    .bind(id)
    .bind(timeline(id))
    .bind(age_days)
    .execute(pool)
    .await
    .unwrap();
    for sql in [
        "INSERT INTO run_matches (run_id, match_id, seed_puuid, already_present) VALUES ($1, $2, $3, false)",
        "INSERT INTO run_discoveries (run_id, match_id, seed_puuid) VALUES ($1, $2, $3)",
    ] {
        sqlx::query(sql)
            .bind(run_id)
            .bind(id)
            .bind(seed)
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn seed(db: &TestDb, run_id: i64, puuid: &str) {
    sqlx::query(
        "INSERT INTO seed_players (run_id, puuid, platform_id, tier, division, league_points, source_page, seed_index)
         VALUES ($1, $2, 'EUW1', 'GOLD', 'II', 42, 1, 0)",
    )
    .bind(run_id)
    .bind(puuid)
    .execute(db.storage.pool())
    .await
    .unwrap();
}

async fn observation(db: &TestDb, puuid: &str, age_days: i32) {
    sqlx::query(
        "INSERT INTO participant_rank_observations
            (platform_id, puuid, queue_id, tier, division, league_points, status, observed_at)
         VALUES ('EUW1', $1, 420, 'GOLD', 'II', 10, 'ranked', now() - make_interval(days => $2))",
    )
    .bind(puuid)
    .bind(age_days)
    .execute(db.storage.pool())
    .await
    .unwrap();
}

async fn job(db: &TestDb, run_id: i64, kind: &str, key: &str, payload: Value, state: &str) {
    sqlx::query(
        "INSERT INTO collection_jobs (run_id, kind, job_key, payload, state) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(run_id)
    .bind(kind)
    .bind(key)
    .bind(payload)
    .bind(state)
    .execute(db.storage.pool())
    .await
    .unwrap();
}

async fn count(db: &TestDb, sql: &str) -> i64 {
    sqlx::query_scalar(sql)
        .fetch_one(db.storage.pool())
        .await
        .unwrap()
}

async fn document(db: &TestDb, sql: &str, id: &str) -> String {
    let value: Value = sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(db.storage.pool())
        .await
        .unwrap();
    value.to_string()
}

#[tokio::test]
async fn la_purge_applique_les_deux_durees_sans_toucher_aux_donnees_recentes() {
    let db = db_or_skip!();
    let old_run = run(&db, "completed", 40).await;
    let recent_run = run(&db, "running", 0).await;
    seed(&db, old_run, "fake-puuid-0").await;
    seed(&db, recent_run, "fake-puuid-1").await;
    stored_match(&db, old_run, "EUW1_100", 100, "fake-puuid-0").await;
    stored_match(&db, old_run, "EUW1_40", 40, "fake-puuid-0").await;
    stored_match(&db, recent_run, "EUW1_1", 1, "fake-puuid-1").await;
    // Partie ancienne encore liée à une collecte active : conservée jusqu'à sa fin.
    stored_match(&db, recent_run, "EUW1_101", 101, "fake-puuid-1").await;
    observation(&db, "fake-puuid-2", 40).await;
    observation(&db, "fake-puuid-3", 0).await;
    job(
        &db,
        old_run,
        "match_ids",
        "fake-puuid-0:0",
        json!({"puuid": "fake-puuid-0", "start": 0, "seed_sort": 0}),
        "pending",
    )
    .await;
    job(
        &db,
        recent_run,
        "match_ids",
        "fake-puuid-1:0",
        json!({"puuid": "fake-puuid-1", "start": 0, "seed_sort": 0}),
        "pending",
    )
    .await;

    let report = purge(&db.storage, RetentionPolicy::new(30, 90).unwrap())
        .await
        .unwrap();
    assert_eq!(report.matches_deleted, 1);
    assert_eq!(report.timelines_deleted, 1);
    // EUW1_101 garde ses données brutes mais perd ses identifiants comme EUW1_40.
    assert_eq!(report.matches_redacted, 2);
    assert_eq!(report.timelines_redacted, 2);
    assert_eq!(report.seed_players_deleted, 1);
    assert_eq!(report.discoveries_deleted, 2);
    assert_eq!(report.sampled_match_seeds_cleared, 1);
    assert_eq!(report.rank_observations_deleted, 1);
    assert!(report.jobs_deleted >= 1);

    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM matches WHERE match_id = 'EUW1_100'"
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM matches WHERE match_id = 'EUW1_101'"
        )
        .await,
        1
    );
    for (table, sql) in [
        ("matches", "SELECT detail FROM matches WHERE match_id = $1"),
        (
            "match_timelines",
            "SELECT timeline FROM match_timelines WHERE match_id = $1",
        ),
    ] {
        let old = document(&db, sql, "EUW1_40").await;
        assert!(
            !old.contains("fake-puuid") && !old.contains("Joueur"),
            "{table}"
        );
        assert!(
            document(&db, sql, "EUW1_1").await.contains("fake-puuid-1"),
            "{table}"
        );
    }
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM matches WHERE identifiers_redacted_at IS NOT NULL"
        )
        .await,
        2
    );
    assert_eq!(
        count(
            &db,
            &format!("SELECT count(*) FROM seed_players WHERE run_id = {old_run}")
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &db,
            &format!("SELECT count(*) FROM seed_players WHERE run_id = {recent_run}")
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &db,
            &format!("SELECT count(*) FROM collection_jobs WHERE run_id = {old_run}")
        )
        .await,
        0
    );
    assert!(
        count(
            &db,
            &format!("SELECT count(*) FROM collection_jobs WHERE run_id = {recent_run}")
        )
        .await
            > 1
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM run_matches WHERE seed_puuid = 'fake-puuid-0'"
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM run_matches WHERE seed_puuid = 'fake-puuid-1'"
        )
        .await,
        2
    );
    assert_eq!(
        count(&db, "SELECT count(*) FROM participant_rank_observations").await,
        1
    );

    let again = purge(&db.storage, RetentionPolicy::new(30, 90).unwrap())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&again).unwrap(),
        json!({
            "identifier_days": 30, "raw_match_days": 90,
            "matches_deleted": 0, "timelines_deleted": 0,
            "matches_redacted": 0, "timelines_redacted": 0,
            "seed_players_deleted": 0, "discoveries_deleted": 0,
            "sampled_match_seeds_cleared": 0, "jobs_deleted": 0,
            "rank_observations_deleted": 0
        }),
        "une deuxième purge n'a plus rien à faire"
    );

    // Les parties pseudonymisées restent exploitables par l'agrégation.
    let aggregated = recalculate(&db.storage, 1).await.unwrap();
    assert_eq!(aggregated.included_matches, 3);
    db.cleanup().await;
}

#[tokio::test]
async fn l_export_ne_contient_que_les_donnees_du_joueur_demande() {
    let db = db_or_skip!();
    let run_id = run(&db, "completed", 0).await;
    seed(&db, run_id, "fake-puuid-1").await;
    stored_match(&db, run_id, "EUW1_2", 1, "fake-puuid-1").await;
    observation(&db, "fake-puuid-1", 0).await;
    observation(&db, "fake-puuid-2", 0).await;
    job(
        &db,
        run_id,
        "participant_rank",
        "fake-puuid-1",
        json!({"platform_id": "EUW1", "puuid": "fake-puuid-1"}),
        "pending",
    )
    .await;

    let export = export_subject(&db.storage, "fake-puuid-1").await.unwrap();
    assert_eq!(export.puuid, "fake-puuid-1");
    assert_eq!(export.seed_entries.len(), 1);
    assert_eq!(export.seed_entries[0].tier, "GOLD");
    assert_eq!(export.rank_observations.len(), 1);
    assert_eq!(export.discoveries.len(), 1);
    assert_eq!(export.sampled_matches.len(), 1);
    assert_eq!(export.matches.len(), 1);
    assert_eq!(export.matches[0].match_id, "EUW1_2");
    assert_eq!(
        export.matches[0].participant.as_ref().unwrap()["riotIdGameName"],
        "Joueur1"
    );
    assert_eq!(export.timeline_match_ids, vec!["EUW1_2".to_owned()]);
    assert_eq!(export.collection_jobs, 1);
    let text = serde_json::to_string(&export).unwrap();
    for other in ["fake-puuid-0", "fake-puuid-2", "Joueur2"] {
        assert!(!text.contains(other), "{other} ne doit pas être exporté");
    }

    let unknown = export_subject(&db.storage, "inconnu").await.unwrap();
    assert!(unknown.matches.is_empty() && unknown.seed_entries.is_empty());
    assert!(matches!(
        export_subject(&db.storage, "BOT").await,
        Err(PrivacyError::InvalidSubject)
    ));
    db.cleanup().await;
}

#[tokio::test]
async fn l_effacement_retire_le_joueur_partout_et_laisse_les_autres() {
    let db = db_or_skip!();
    let run_id = run(&db, "running", 0).await;
    seed(&db, run_id, "fake-puuid-1").await;
    stored_match(&db, run_id, "EUW1_3", 1, "fake-puuid-1").await;
    observation(&db, "fake-puuid-1", 0).await;
    observation(&db, "fake-puuid-2", 0).await;
    job(
        &db,
        run_id,
        "participant_rank",
        "fake-puuid-1",
        json!({"platform_id": "EUW1", "puuid": "fake-puuid-1"}),
        "pending",
    )
    .await;
    job(
        &db,
        run_id,
        "match",
        "EUW1_9",
        json!({"match_id": "EUW1_9", "seed_puuid": "fake-puuid-1"}),
        "running",
    )
    .await;

    let erased = erase_subject(&db.storage, "fake-puuid-1").await.unwrap();
    assert_eq!(erased.seed_entries, 1);
    assert_eq!(erased.rank_observations, 1);
    assert_eq!(erased.discoveries, 1);
    assert_eq!(erased.sampled_matches, 1);
    assert_eq!(erased.jobs, 1);
    assert_eq!(erased.jobs_in_flight, 1);
    assert_eq!(erased.matches, 1);
    assert_eq!(erased.timelines, 1);

    let detail = document(
        &db,
        "SELECT detail FROM matches WHERE match_id = $1",
        "EUW1_3",
    )
    .await;
    assert!(!detail.contains("fake-puuid-1\"") && !detail.contains("Joueur1"));
    assert!(detail.contains("fake-puuid-2") && detail.contains("Joueur2"));
    let timeline = document(
        &db,
        "SELECT timeline FROM match_timelines WHERE match_id = $1",
        "EUW1_3",
    )
    .await;
    assert!(!timeline.contains("fake-puuid-1\"") && timeline.contains("fake-puuid-2"));
    assert_eq!(
        count(&db, "SELECT count(*) FROM participant_rank_observations").await,
        1
    );

    let after = export_subject(&db.storage, "fake-puuid-1").await.unwrap();
    assert!(after.seed_entries.is_empty() && after.rank_observations.is_empty());
    assert!(after.discoveries.is_empty() && after.sampled_matches.is_empty());
    assert!(after.matches.is_empty() && after.timeline_match_ids.is_empty());
    // Seul le travail en cours reste : il est signalé pour relancer l'effacement.
    assert_eq!(after.collection_jobs, 1);

    let row = sqlx::query("SELECT count(*) AS n FROM matches")
        .fetch_one(db.storage.pool())
        .await
        .unwrap();
    assert_eq!(row.get::<i64, _>("n"), 1);
    assert_eq!(
        recalculate(&db.storage, 1).await.unwrap().included_matches,
        1
    );
    let twice = erase_subject(&db.storage, "fake-puuid-1").await.unwrap();
    assert_eq!((twice.matches, twice.seed_entries, twice.jobs), (0, 0, 0));
    assert!(matches!(
        erase_subject(&db.storage, "").await,
        Err(PrivacyError::InvalidSubject)
    ));
    db.cleanup().await;
}
