//! Tests d'intégration de la chaîne complète contre PostgreSQL, avec un faux Riot.

mod common;

use std::sync::Arc;

use common::{fast_options, status, FakeRiot, TestDb, DAY_MS};
use olc_collector::collector::{now_ms, Collector, StopReason};
use olc_collector::config::{Division, RunParams, RuntimeOptions, Tier};
use olc_collector::model::fixtures;
use olc_collector::report;
use olc_collector::riot_client::{Endpoint, TransportError};
use olc_collector::storage::{RunStatus, StorageError};
use tokio::sync::Notify;

fn params(target: u32, seeds: u32) -> RunParams {
    RunParams {
        target_matches: target,
        tiers: vec![Tier::Gold],
        divisions: vec![Division::I],
        window_days: 14,
        seeds_per_division: seeds,
        max_matches_per_seed: 10,
        call_budget: 1000,
    }
}

fn recent() -> i64 {
    now_ms() - DAY_MS
}

async fn never() {
    std::future::pending::<()>().await
}

macro_rules! db_or_skip {
    () => {
        match TestDb::create().await {
            Some(db) => db,
            None => return,
        }
    };
}

/// Trois joueurs, sept mentions, cinq parties distinctes.
fn three_seeds(fake: &FakeRiot) {
    fake.league_page("GOLD", "I", 1, &["seed-a", "seed-b", "seed-c"]);
    fake.history("seed-a", &["EUW1_1", "EUW1_2", "EUW1_3"]);
    fake.history("seed-b", &["EUW1_2", "EUW1_3", "EUW1_4"]);
    fake.history("seed-c", &["EUW1_5"]);
    for i in 1..=5 {
        fake.game(&format!("EUW1_{i}"), recent());
    }
}

#[tokio::test]
async fn une_partie_decouverte_par_plusieurs_joueurs_n_est_stockee_qu_une_fois() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    three_seeds(&fake);
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let run_id = collector.start_run(&params(10, 3), now_ms()).await.unwrap();
    let outcome = collector.execute(run_id, never()).await.unwrap();

    // Cible de 10 non atteinte : sources épuisées, résultat incomplet explicite.
    assert_eq!(outcome.status, RunStatus::Incomplete);
    assert_eq!(outcome.reason, StopReason::Finished);
    assert_eq!(outcome.retained, 5);
    assert_eq!(db.scalar("SELECT count(*) FROM matches").await, 5);
    for i in 1..=5 {
        assert_eq!(fake.calls_for(Endpoint::Match, &format!("EUW1_{i}")), 1);
    }
    let r = report::generate(&db.storage, run_id).await.unwrap();
    assert_eq!(r.discovery.sightings, 7);
    assert_eq!(r.discovery.distinct_matches, 5);
    assert_eq!(r.discovery.duplicates, 2);
    assert_eq!(r.timelines.available, 5);
    assert!(!r.complete);
    assert_eq!(r.calls_made, 1 + 3 + 5 + 5);
    assert!(r.render().contains("INCOMPLET"));
    db.cleanup().await;
}

#[tokio::test]
async fn la_cible_est_respectee_exactement() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    three_seeds(&fake);
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let run_id = collector.start_run(&params(3, 3), now_ms()).await.unwrap();
    let outcome = collector.execute(run_id, never()).await.unwrap();

    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(outcome.retained, 3);
    assert_eq!(fake.calls(Endpoint::Match), 3);
    assert_eq!(fake.calls(Endpoint::Timeline), 3);
    let r = report::generate(&db.storage, run_id).await.unwrap();
    assert!(r.complete);
    assert_eq!(r.matches.not_fetched, 2);
    db.cleanup().await;
}

#[tokio::test]
async fn l_echantillon_alterne_les_strates_et_suit_la_pagination() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    // Division I : 3 joueurs répartis sur 2 pages. Division II : 1 joueur.
    fake.league_page("GOLD", "I", 1, &["i-1", "i-2"]);
    fake.league_page("GOLD", "I", 2, &["i-3", "i-4"]);
    fake.league_page("GOLD", "II", 1, &["ii-1"]);
    for (seed, games) in [
        ("i-1", ["EUW1_11", "EUW1_12"]),
        ("i-2", ["EUW1_21", "EUW1_22"]),
        ("i-3", ["EUW1_31", "EUW1_32"]),
        ("ii-1", ["EUW1_41", "EUW1_42"]),
    ] {
        fake.history(seed, &games);
        for g in games {
            fake.game(g, recent());
        }
    }
    let p = RunParams {
        divisions: vec![Division::I, Division::II],
        ..params(4, 3)
    };
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let run_id = collector.start_run(&p, now_ms()).await.unwrap();
    collector.execute(run_id, never()).await.unwrap();

    // Trois joueurs retenus en division I : la page 3 n'est jamais demandée.
    assert_eq!(
        db.scalar("SELECT count(*) FROM seed_players WHERE division = 'I'")
            .await,
        3
    );
    assert_eq!(fake.calls_for(Endpoint::LeagueEntries, "GOLD/I/3"), 0);
    // Première partie de chaque joueur avant les deuxièmes : les 4 parties retenues
    // viennent de 4 joueurs différents, dont celui de la division II.
    assert_eq!(
        db.scalar("SELECT count(DISTINCT seed_puuid) FROM run_matches")
            .await,
        4
    );
    assert_eq!(
        db.scalar("SELECT count(*) FROM run_matches WHERE seed_puuid = 'ii-1'")
            .await,
        1
    );
    db.cleanup().await;
}

#[tokio::test]
async fn les_parties_hors_perimetre_sont_exclues_avec_leur_raison() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    fake.league_page("GOLD", "I", 1, &["seed-a"]);
    fake.history(
        "seed-a",
        &["EUN1_9", "EUW1_flex", "EUW1_old", "EUW1_other", "EUW1_ok"],
    );
    fake.game_with("EUW1_flex", "EUW1", 440, recent(), true);
    fake.game_with("EUW1_old", "EUW1", 420, now_ms() - 30 * DAY_MS, true);
    // Transfert de compte : identifiant EUW1 mais plateforme différente.
    fake.game_with("EUW1_other", "EUN1", 420, recent(), true);
    fake.game("EUW1_ok", recent());
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let run_id = collector.start_run(&params(10, 1), now_ms()).await.unwrap();
    collector.execute(run_id, never()).await.unwrap();

    let r = report::generate(&db.storage, run_id).await.unwrap();
    assert_eq!(r.matches.retained, 1);
    assert_eq!(r.matches.excluded.get("wrong_platform"), Some(&2));
    assert_eq!(r.matches.excluded.get("wrong_queue"), Some(&1));
    assert_eq!(r.matches.excluded.get("out_of_window"), Some(&1));
    // Préfixe d'une autre plateforme : exclue sans appel.
    assert_eq!(fake.calls_for(Endpoint::Match, "EUN1_9"), 0);
    assert_eq!(db.scalar("SELECT count(*) FROM matches").await, 1);
    db.cleanup().await;
}

#[tokio::test]
async fn une_timeline_404_est_revalidee_puis_declaree_indisponible() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    fake.league_page("GOLD", "I", 1, &["seed-a"]);
    fake.history("seed-a", &["EUW1_1"]);
    fake.game_with("EUW1_1", "EUW1", 420, recent(), false);
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let run_id = collector.start_run(&params(1, 1), now_ms()).await.unwrap();
    let outcome = collector.execute(run_id, never()).await.unwrap();

    assert_eq!(outcome.status, RunStatus::Completed);
    // Un 404 initial puis deux revalidations.
    assert_eq!(fake.calls(Endpoint::Timeline), 3);
    let r = report::generate(&db.storage, run_id).await.unwrap();
    assert_eq!(r.timelines.unavailable, 1);
    assert_eq!(r.timelines.available, 0);
    // Le détail reste acquis.
    assert_eq!(db.scalar("SELECT count(*) FROM matches").await, 1);
    db.cleanup().await;
}

#[tokio::test]
async fn erreurs_serveur_timeout_et_429_sont_reessayes() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    fake.league_page("GOLD", "I", 1, &["seed-a"]);
    fake.history("seed-a", &["EUW1_1"]);
    fake.game("EUW1_1", recent());
    fake.script(
        "matches/EUW1_1",
        vec![
            Ok(status(503, &[])),
            Err(TransportError::Timeout),
            Ok(status(
                429,
                &[("Retry-After", "0"), ("X-Rate-Limit-Type", "application")],
            )),
        ],
    );
    // 429 sans aucun en-tête sur la timeline.
    fake.script("matches/EUW1_1/timeline", vec![Ok(status(429, &[]))]);
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let run_id = collector.start_run(&params(1, 1), now_ms()).await.unwrap();
    let outcome = collector.execute(run_id, never()).await.unwrap();

    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(fake.calls(Endpoint::Match), 4);
    assert_eq!(fake.calls(Endpoint::Timeline), 2);
    // Les 429 ne comptent pas comme des tentatives (max_attempts = 3).
    assert_eq!(
        db.scalar("SELECT attempts::bigint FROM collection_jobs WHERE kind = 'match'")
            .await,
        2
    );
    db.cleanup().await;
}

#[tokio::test]
async fn une_reponse_invalide_n_est_jamais_comptee_comme_acquise() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    fake.league_page("GOLD", "I", 1, &["seed-a"]);
    fake.history("seed-a", &["EUW1_1"]);
    let garbage = || {
        Ok(olc_collector::riot_client::RawResponse {
            status: 200,
            headers: vec![],
            body: b"{\"metadata\":".to_vec(),
        })
    };
    fake.script("matches/EUW1_1", vec![garbage(), garbage(), garbage()]);
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let run_id = collector.start_run(&params(1, 1), now_ms()).await.unwrap();
    let outcome = collector.execute(run_id, never()).await.unwrap();

    assert_eq!(outcome.status, RunStatus::Incomplete);
    assert_eq!(db.scalar("SELECT count(*) FROM matches").await, 0);
    let r = report::generate(&db.storage, run_id).await.unwrap();
    assert_eq!(r.matches.failed, 1);
    assert!(r.errors[0].error.contains("réponse invalide"));
    db.cleanup().await;
}

#[tokio::test]
async fn une_cle_refusee_suspend_la_collecte_et_la_reprise_continue() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    three_seeds(&fake);
    fake.script("matches/EUW1_1/timeline", vec![Ok(status(403, &[]))]);
    let options = RuntimeOptions {
        concurrency: 1,
        ..fast_options()
    };
    let collector = Collector::new(db.storage.clone(), fake.clone(), options.clone());
    let run_id = collector.start_run(&params(5, 3), now_ms()).await.unwrap();
    let outcome = collector.execute(run_id, never()).await.unwrap();
    assert_eq!(outcome.status, RunStatus::Paused);
    assert_eq!(outcome.reason, StopReason::AuthRejected(403));
    let kept = db.scalar("SELECT count(*) FROM matches").await;
    assert!(kept >= 1);

    // Nouvelle clé, nouveau processus : rien de déjà acquis n'est redemandé.
    let fake2 = FakeRiot::default();
    three_seeds(&fake2);
    let collector2 = Collector::new(db.storage.clone(), fake2.clone(), options);
    let outcome = collector2.execute(run_id, never()).await.unwrap();
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(outcome.retained, 5);
    assert_eq!(fake2.calls(Endpoint::LeagueEntries), 0);
    assert_eq!(fake2.calls(Endpoint::Match) as i64, 5 - kept);
    db.cleanup().await;
}

#[tokio::test]
async fn apres_un_arret_brutal_la_timeline_reprend_sans_retelecharger_le_detail() {
    let db = db_or_skip!();
    let notify = Arc::new(Notify::new());
    let fake = FakeRiot::with_hang(notify.clone());
    fake.league_page("GOLD", "I", 1, &["seed-a"]);
    fake.history("seed-a", &["EUW1_1"]);
    fake.game("EUW1_1", recent());
    // Le processus « tué » a son propre pool, comme un vrai processus.
    let crashed = db.separate_storage().await;
    let collector = Collector::new(crashed.clone(), fake.clone(), fast_options());
    let run_id = collector.start_run(&params(1, 1), now_ms()).await.unwrap();
    let task = {
        let collector = collector.clone();
        tokio::spawn(async move { collector.execute(run_id, never()).await })
    };
    // Le détail est enregistré, la timeline est en cours : on tue le processus,
    // ce qui ferme aussi ses connexions (et annule ses transactions ouvertes).
    notify.notified().await;
    task.abort();
    let _ = task.await;
    drop(collector);
    crashed.pool().close().await;
    assert_eq!(db.scalar("SELECT count(*) FROM matches").await, 1);
    assert_eq!(
        db.scalar(
            "SELECT count(*) FROM collection_jobs WHERE kind = 'timeline' AND state = 'running'"
        )
        .await,
        1
    );

    let fake2 = FakeRiot::default();
    fake2.game("EUW1_1", recent());
    let collector2 = Collector::new(db.storage.clone(), fake2.clone(), fast_options());
    let outcome = collector2.execute(run_id, never()).await.unwrap();
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(fake2.calls(Endpoint::Match), 0);
    assert_eq!(fake2.calls(Endpoint::Timeline), 1);
    assert_eq!(
        db.scalar("SELECT count(*) FROM match_timelines WHERE status = 'available'")
            .await,
        1
    );
    db.cleanup().await;
}

#[tokio::test]
async fn relancer_une_execution_terminee_ne_fait_aucun_appel() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    three_seeds(&fake);
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let run_id = collector.start_run(&params(3, 3), now_ms()).await.unwrap();
    collector.execute(run_id, never()).await.unwrap();
    let calls_before = collector.calls();

    let outcome = collector.execute(run_id, never()).await.unwrap();
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(collector.calls(), calls_before);
    assert_eq!(db.scalar("SELECT count(*) FROM matches").await, 3);
    db.cleanup().await;
}

#[tokio::test]
async fn une_nouvelle_execution_reutilise_les_parties_deja_en_base() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    three_seeds(&fake);
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let first = collector.start_run(&params(10, 3), now_ms()).await.unwrap();
    collector.execute(first, never()).await.unwrap();

    let fake2 = FakeRiot::default();
    three_seeds(&fake2);
    let collector2 = Collector::new(db.storage.clone(), fake2.clone(), fast_options());
    let second = collector2
        .start_run(&params(10, 3), now_ms())
        .await
        .unwrap();
    let outcome = collector2.execute(second, never()).await.unwrap();

    assert_eq!(outcome.retained, 5);
    assert_eq!(fake2.calls(Endpoint::Match), 0);
    assert_eq!(fake2.calls(Endpoint::Timeline), 0);
    let r = report::generate(&db.storage, second).await.unwrap();
    assert_eq!(r.matches.already_present, 5);
    assert_eq!(r.matches.downloaded, 0);
    assert_eq!(db.scalar("SELECT count(*) FROM matches").await, 5);
    db.cleanup().await;
}

#[tokio::test]
async fn le_budget_d_appels_arrete_la_collecte_en_gardant_la_progression() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    three_seeds(&fake);
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let p = RunParams {
        call_budget: 4,
        ..params(5, 3)
    };
    let run_id = collector.start_run(&p, now_ms()).await.unwrap();
    let outcome = collector.execute(run_id, never()).await.unwrap();
    assert_eq!(outcome.status, RunStatus::Paused);
    assert_eq!(outcome.reason, StopReason::CallBudget);
    assert!(collector.calls() <= 4);

    db.storage.set_call_budget(run_id, 100).await.unwrap();
    let outcome = collector.execute(run_id, never()).await.unwrap();
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(fake.calls(Endpoint::LeagueEntries), 1);
    db.cleanup().await;
}

#[tokio::test]
async fn exact_call_budget_finishes_completed_collection() {
    for concurrency in [1, 2] {
        let db = db_or_skip!();
        let fake = FakeRiot::default();
        fake.league_page("GOLD", "I", 1, &["seed-a"]);
        fake.history("seed-a", &["EUW1_1", "EUW1_2"]);
        fake.game("EUW1_1", recent());
        fake.game("EUW1_2", recent());
        let collector = Collector::new(
            db.storage.clone(),
            fake,
            RuntimeOptions {
                concurrency,
                ..fast_options()
            },
        );
        let p = RunParams {
            call_budget: 4,
            ..params(1, 1)
        };
        let run_id = collector.start_run(&p, now_ms()).await.unwrap();
        let outcome = collector.execute(run_id, never()).await.unwrap();
        let r = report::generate(&db.storage, run_id).await.unwrap();
        let resumed = collector.execute(run_id, never()).await.unwrap();
        let calls = collector.calls();
        db.cleanup().await;

        assert_eq!(outcome.status, RunStatus::Completed);
        assert_eq!(outcome.reason, StopReason::Finished);
        assert_eq!(r.matches.retained, 1);
        assert_eq!(r.timelines.available, 1);
        // La candidate restante n'empêche pas la clôture une fois la cible atteinte.
        assert_eq!(r.matches.not_fetched, 1);
        assert_eq!(resumed.status, RunStatus::Completed);
        assert_eq!(calls, 4);
    }
}

#[tokio::test]
async fn exact_call_budget_finishes_exhausted_sources() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    fake.league_page("GOLD", "I", 1, &["seed-a"]);
    fake.history("seed-a", &[]);
    let collector = Collector::new(db.storage.clone(), fake, fast_options());
    let p = RunParams {
        call_budget: 2,
        ..params(1, 1)
    };
    let run_id = collector.start_run(&p, now_ms()).await.unwrap();
    let outcome = collector.execute(run_id, never()).await.unwrap();
    let calls = collector.calls();
    db.cleanup().await;

    assert_eq!(outcome.status, RunStatus::Incomplete);
    assert_eq!(outcome.reason, StopReason::Finished);
    assert_eq!(outcome.retained, 0);
    assert_eq!(calls, 2);
}

#[tokio::test]
async fn l_arret_demande_met_l_execution_en_pause() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    three_seeds(&fake);
    let collector = Collector::new(db.storage.clone(), fake.clone(), fast_options());
    let run_id = collector.start_run(&params(5, 3), now_ms()).await.unwrap();
    let outcome = collector.execute(run_id, async {}).await.unwrap();
    assert_eq!(outcome.status, RunStatus::Paused);
    assert_eq!(outcome.reason, StopReason::Interrupted);
    let outcome = collector.execute(run_id, never()).await.unwrap();
    assert_eq!(outcome.status, RunStatus::Completed);
    db.cleanup().await;
}

#[tokio::test]
async fn un_seul_collecteur_peut_travailler_sur_la_base() {
    let db = db_or_skip!();
    let lock = db.storage.lock_collector().await.unwrap();
    assert!(matches!(
        db.storage.lock_collector().await,
        Err(StorageError::Busy)
    ));
    lock.release().await;
    assert!(db.storage.lock_collector().await.is_ok());
    db.cleanup().await;
}

#[tokio::test]
async fn les_contraintes_d_unicite_sont_imposees_par_postgresql() {
    let db = db_or_skip!();
    let fake = FakeRiot::default();
    let collector = Collector::new(db.storage.clone(), fake, fast_options());
    let run_id = collector.start_run(&params(1, 1), now_ms()).await.unwrap();
    let insert_job = format!(
        "INSERT INTO collection_jobs (run_id, kind, job_key, payload) VALUES ({run_id}, 'match', 'EUW1_1', '{{}}')"
    );
    sqlx::query(&insert_job)
        .execute(db.storage.pool())
        .await
        .unwrap();
    assert!(sqlx::query(&insert_job)
        .execute(db.storage.pool())
        .await
        .is_err());

    let detail = serde_json::to_string(&fixtures::match_detail("EUW1_1", "EUW1", 420, 0)).unwrap();
    let insert_match = format!(
        "INSERT INTO matches (match_id, platform_id, queue_id, game_version, patch, game_start,
                              game_duration_s, is_remake, detail, first_run_id)
         VALUES ('EUW1_1', 'EUW1', 420, '15.19.1', '15.19', now(), 1800, false, '{detail}', {run_id})"
    );
    sqlx::query(&insert_match)
        .execute(db.storage.pool())
        .await
        .unwrap();
    assert!(sqlx::query(&insert_match)
        .execute(db.storage.pool())
        .await
        .is_err());
    // Une timeline « disponible » sans contenu est refusée.
    assert!(sqlx::query(
        "INSERT INTO match_timelines (match_id, status) VALUES ('EUW1_1', 'available')"
    )
    .execute(db.storage.pool())
    .await
    .is_err());
    db.cleanup().await;
}

#[tokio::test]
async fn apres_une_panne_reseau_les_travaux_en_echec_peuvent_etre_relances() {
    let db = db_or_skip!();
    let down = FakeRiot::default();
    let net = || Err(TransportError::Network("proxy refusé".into()));
    down.script("league/GOLD/I/1", vec![net(), net(), net()]);
    let collector = Collector::new(db.storage.clone(), down, fast_options());
    let run_id = collector.start_run(&params(3, 3), now_ms()).await.unwrap();
    let outcome = collector.execute(run_id, never()).await.unwrap();
    assert_eq!(outcome.status, RunStatus::Incomplete);
    let r = report::generate(&db.storage, run_id).await.unwrap();
    assert_eq!(r.failed_jobs, 1);
    assert!(r.errors[0].error.contains("proxy refusé"));

    assert_eq!(db.storage.requeue_failed(run_id).await.unwrap(), 1);
    let up = FakeRiot::default();
    three_seeds(&up);
    let collector = Collector::new(db.storage.clone(), up, fast_options());
    let outcome = collector.execute(run_id, never()).await.unwrap();
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(outcome.retained, 3);
    db.cleanup().await;
}
