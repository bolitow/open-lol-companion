//! Budget d'appels Riot (#90, suite) : ordre de réservation, cache négatif des parties
//! exclues et fermeture des demandes de rang inutiles. Tests PostgreSQL, faux Riot.

mod common;

use common::{fast_options, FakeRiot, TestDb, DAY_MS};
use olc_collector::collector::{now_ms, Collector};
use olc_collector::config::{Division, RunParams, Tier};
use olc_collector::riot_client::Endpoint;
use olc_collector::storage::JobKind;

macro_rules! db_or_skip {
    () => {
        match TestDb::create().await {
            Some(db) => db,
            None => return,
        }
    };
}

fn params(queue_id: i32) -> RunParams {
    RunParams {
        queue_id,
        target_matches: 10,
        tiers: vec![Tier::Gold],
        divisions: vec![Division::I],
        window_days: 14,
        seeds_per_division: 1,
        max_matches_per_seed: 10,
        call_budget: 1000,
        ..RunParams::default()
    }
}

fn recent() -> i64 {
    now_ms() - DAY_MS
}

async fn never() {
    std::future::pending::<()>().await
}

#[tokio::test]
async fn les_details_de_partie_sont_reserves_avant_les_rangs() {
    let db = db_or_skip!();
    let collector = Collector::new(db.storage.clone(), FakeRiot::default(), fast_options());
    let run_id = collector.start_run(&params(0), now_ms()).await.unwrap();
    // Plus de découverte en attente : seuls restent un rang, puis un détail de partie.
    sqlx::query("UPDATE collection_jobs SET state = 'done' WHERE run_id = $1")
        .bind(run_id)
        .execute(db.storage.pool())
        .await
        .unwrap();
    for (kind, key, payload) in [
        (
            "participant_rank",
            "puuid-a",
            serde_json::json!({"platform_id": "EUW1", "puuid": "puuid-a"}),
        ),
        (
            "match",
            "EUW1_1",
            serde_json::json!({"match_id": "EUW1_1", "seed_puuid": "puuid-a"}),
        ),
    ] {
        sqlx::query(
            "INSERT INTO collection_jobs (run_id, kind, job_key, payload) VALUES ($1, $2, $3, $4)",
        )
        .bind(run_id)
        .bind(kind)
        .bind(key)
        .bind(payload)
        .execute(db.storage.pool())
        .await
        .unwrap();
    }

    let first = db.storage.claim_next(run_id, 10).await.unwrap().unwrap();
    assert_eq!(first.kind, JobKind::Match);
    let second = db.storage.claim_next(run_id, 10).await.unwrap().unwrap();
    assert_eq!(second.kind, JobKind::ParticipantRank);
    db.cleanup().await;
}

#[tokio::test]
async fn une_partie_exclue_n_est_pas_retelechargee_par_une_autre_execution() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    fake.league_page("GOLD", "I", 1, &["seed-a"]);
    fake.history("seed-a", &["EUW1_flex", "EUW1_ok"]);
    fake.game_with("EUW1_flex", "EUW1", 440, recent(), true);
    fake.game("EUW1_ok", recent());
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let first = collector.start_run(&params(420), now_ms()).await.unwrap();
    collector.execute(first, never()).await.unwrap();
    assert_eq!(fake.calls_for(Endpoint::Match, "EUW1_flex"), 1);

    let second = collector.start_run(&params(420), now_ms()).await.unwrap();
    collector.execute(second, never()).await.unwrap();

    // Même verdict, sans nouvel appel : la partie reste exclue pour la file 420.
    assert_eq!(fake.calls_for(Endpoint::Match, "EUW1_flex"), 1);
    assert_eq!(
        db.scalar(&format!(
            "SELECT count(*) FROM collection_jobs
             WHERE run_id = {second} AND outcome = 'excluded:wrong_queue'"
        ))
        .await,
        1
    );
    // Aucun détail brut conservé pour une partie exclue : seulement ses faits.
    assert_eq!(
        db.scalar("SELECT count(*) FROM matches WHERE match_id = 'EUW1_flex'")
            .await,
        0
    );
    db.cleanup().await;
}

#[tokio::test]
async fn une_partie_exclue_d_un_perimetre_reste_telechargeable_dans_un_autre() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    fake.league_page("GOLD", "I", 1, &["seed-a"]);
    fake.history("seed-a", &["EUW1_flex"]);
    fake.game_with("EUW1_flex", "EUW1", 440, recent(), true);
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let solo = collector.start_run(&params(420), now_ms()).await.unwrap();
    collector.execute(solo, never()).await.unwrap();
    assert_eq!(db.scalar("SELECT count(*) FROM matches").await, 0);

    // Le verdict dépend du périmètre : la file 440 accepte cette partie.
    let flex = collector.start_run(&params(0), now_ms()).await.unwrap();
    collector.execute(flex, never()).await.unwrap();

    assert_eq!(fake.calls_for(Endpoint::Match, "EUW1_flex"), 2);
    assert_eq!(db.scalar("SELECT count(*) FROM matches").await, 1);
    db.cleanup().await;
}

/// Insère une partie retenue par `run_id`, dont les joueurs sont `<prefix>-0` à `<prefix>-9`.
async fn insert_linked_match(db: &TestDb, run_id: i64, match_id: &str, queue: i32, prefix: &str) {
    let detail = olc_collector::model::fixtures::match_detail(match_id, "EUW1", queue, recent());
    let detail: serde_json::Value =
        serde_json::from_str(&detail.to_string().replace("fake-puuid", prefix)).unwrap();
    sqlx::query(
        "INSERT INTO matches (match_id, platform_id, queue_id, game_version, patch, game_start,
                              game_duration_s, is_remake, detail, first_run_id)
         VALUES ($1, 'EUW1', $2, '15.19.1', '15.19', now(), 1800, false, $3, $4)",
    )
    .bind(match_id)
    .bind(queue)
    .bind(detail)
    .bind(run_id)
    .execute(db.storage.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO run_matches (run_id, match_id, seed_puuid, already_present)
         VALUES ($1, $2, '', false)",
    )
    .bind(run_id)
    .bind(match_id)
    .execute(db.storage.pool())
    .await
    .unwrap();
}

async fn insert_rank_job(db: &TestDb, run_id: i64, puuid: &str) {
    sqlx::query(
        "INSERT INTO collection_jobs (run_id, kind, job_key, payload)
         VALUES ($1, 'participant_rank', $2, $3)",
    )
    .bind(run_id)
    .bind(puuid)
    .bind(serde_json::json!({"platform_id": "EUW1", "puuid": puuid}))
    .execute(db.storage.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn la_maintenance_ferme_les_rangs_demandes_hors_files_classees() {
    let db = db_or_skip!();
    let collector = Collector::new(db.storage.clone(), FakeRiot::default(), fast_options());
    let run_id = collector.start_run(&params(0), now_ms()).await.unwrap();
    insert_linked_match(&db, run_id, "EUW1_aram", 450, "aram").await;
    insert_linked_match(&db, run_id, "EUW1_solo", 420, "solo").await;
    // Un joueur présent dans les deux parties reste utile ; un autre n'est lié à aucune.
    insert_linked_match(&db, run_id, "EUW1_aram2", 450, "solo").await;
    for puuid in ["aram-0", "aram-1", "solo-0", "orphelin"] {
        insert_rank_job(&db, run_id, puuid).await;
    }

    let dry = db
        .storage
        .close_unserved_rank_jobs(Some(run_id), false)
        .await
        .unwrap();
    assert_eq!(dry, 2);
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_jobs WHERE kind = 'participant_rank' AND state = 'pending'")
            .await,
        4
    );

    let closed = db
        .storage
        .close_unserved_rank_jobs(Some(run_id), true)
        .await
        .unwrap();
    assert_eq!(closed, 2);
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_jobs WHERE state = 'done' AND outcome = 'skipped:unserved_queue' AND job_key LIKE 'aram-%'")
            .await,
        2
    );
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_jobs WHERE kind = 'participant_rank' AND state = 'pending'")
            .await,
        2
    );
    // Idempotent : plus rien à fermer.
    assert_eq!(
        db.storage
            .close_unserved_rank_jobs(Some(run_id), true)
            .await
            .unwrap(),
        0
    );
    db.cleanup().await;
}

#[tokio::test]
async fn la_commande_de_maintenance_simule_par_defaut_et_n_applique_qu_avec_apply() {
    let db = db_or_skip!();
    let collector = Collector::new(db.storage.clone(), FakeRiot::default(), fast_options());
    let run_id = collector.start_run(&params(0), now_ms()).await.unwrap();
    insert_linked_match(&db, run_id, "EUW1_aram", 450, "aram").await;
    insert_rank_job(&db, run_id, "aram-0").await;
    let run = |extra: &[&str]| {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_olc-collector"))
            .arg("close-unserved-ranks")
            .args(extra)
            .env("DATABASE_URL", db.database_url())
            .env_remove("RIOT_API_KEY")
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        String::from_utf8(output.stdout).unwrap()
    };

    let dry: serde_json::Value = serde_json::from_str(&run(&["--json"])).unwrap();
    assert_eq!(dry, serde_json::json!({"applied": false, "jobs": 1}));
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_jobs WHERE outcome IS NOT NULL")
            .await,
        0
    );
    let applied: serde_json::Value = serde_json::from_str(&run(&[
        "--json",
        "--apply",
        "--run-id",
        &run_id.to_string(),
    ]))
    .unwrap();
    assert_eq!(applied, serde_json::json!({"applied": true, "jobs": 1}));
    db.cleanup().await;
}
