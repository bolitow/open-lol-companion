//! Campagnes par file (#97) : bases jetables et réponses Riot synthétiques, aucun appel réel.
mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{fast_options, FakeRiot, TestDb, DAY_MS};
use olc_collector::campaign::{self, CampaignStatus};
use olc_collector::collector::now_ms;
use olc_collector::config::{Division, RunParams, Tier};
use olc_collector::riot_client::{Endpoint, RawResponse, Request, Transport, TransportError};

fn params() -> RunParams {
    RunParams {
        target_matches: 2,
        tiers: vec![Tier::Gold],
        divisions: vec![Division::I],
        seeds_per_division: 1,
        max_matches_per_seed: 5,
        call_budget: 100,
        ..RunParams::default()
    }
}

fn platforms() -> Vec<String> {
    vec!["EUW1".into(), "NA1".into()]
}

async fn never() {
    std::future::pending::<()>().await
}

/// Mémorise la file demandée à chaque historique, comme le ferait le filtre `queue` de Riot.
#[derive(Clone)]
struct Recorded {
    fake: FakeRiot,
    history_queues: Arc<Mutex<Vec<(String, String)>>>,
}

impl Transport for Recorded {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        if request.endpoint == Endpoint::MatchIdsByPuuid {
            let queue = request
                .query
                .iter()
                .find(|(name, _)| *name == "queue")
                .map(|(_, value)| value.clone())
                .unwrap_or_default();
            self.history_queues
                .lock()
                .unwrap()
                .push((request.route.host(), queue));
        }
        self.fake.send(request).await
    }
}

/// Un joueur par plateforme dont l'historique mêle une partie ARAM et une partie Swiftplay.
fn fake() -> FakeRiot {
    let fake = FakeRiot::default();
    fake.league_page("GOLD", "I", 1, &["seed"]);
    fake.history("seed", &["EUW1_1", "EUW1_2", "NA1_1", "NA1_2"]);
    for platform in ["EUW1", "NA1"] {
        fake.game_with(
            &format!("{platform}_1"),
            platform,
            450,
            now_ms() - DAY_MS,
            true,
        );
        fake.game_with(
            &format!("{platform}_2"),
            platform,
            480,
            now_ms() - DAY_MS,
            true,
        );
    }
    fake
}

#[tokio::test]
async fn cree_un_run_par_plateforme_et_par_file_dans_l_ordre_plateforme_puis_file() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let id = campaign::start_per_queue(
        &db.storage,
        &platforms(),
        &[450, 480],
        &params(),
        now_ms(),
        Duration::from_secs(3600),
    )
    .await
    .unwrap();
    let rows: Vec<(String, i32, i32)> = sqlx::query_as(
        "SELECT r.platform_id, r.queue_id, r.target_matches FROM collection_runs r
         JOIN campaign_runs c ON c.run_id = r.id WHERE c.campaign_id = $1 ORDER BY c.ordinal",
    )
    .bind(id)
    .fetch_all(db.storage.pool())
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![
            ("EUW1".into(), 450, 2),
            ("EUW1".into(), 480, 2),
            ("NA1".into(), 450, 2),
            ("NA1".into(), 480, 2),
        ]
    );
    let windows: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT (window_start, window_end)) FROM collection_runs",
    )
    .fetch_one(db.storage.pool())
    .await
    .unwrap();
    assert_eq!(windows, 1);
    let queues: serde_json::Value =
        sqlx::query_scalar("SELECT params->'queues' FROM collection_campaigns WHERE id = $1")
            .bind(id)
            .fetch_one(db.storage.pool())
            .await
            .unwrap();
    assert_eq!(queues, serde_json::json!([450, 480]));
    db.cleanup().await;
}

#[tokio::test]
async fn refuse_les_files_absentes_dupliquees_ou_inconnues_sans_rien_creer() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    for queues in [
        vec![],
        vec![450, 450],
        vec![0],
        vec![-1],
        vec![450, 710],
        vec![3130],
    ] {
        assert!(
            campaign::start_per_queue(
                &db.storage,
                &platforms(),
                &queues,
                &params(),
                now_ms(),
                Duration::from_secs(3600),
            )
            .await
            .is_err(),
            "{queues:?}"
        );
    }
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_campaigns").await,
        0
    );
    assert_eq!(db.scalar("SELECT count(*) FROM collection_runs").await, 0);
    db.cleanup().await;
}

#[tokio::test]
async fn chaque_run_demande_sa_file_et_la_couverture_suit_la_cible_par_file_et_plateforme() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let riot = Recorded {
        fake: fake(),
        history_queues: Arc::new(Mutex::new(vec![])),
    };
    let id = campaign::start_per_queue(
        &db.storage,
        &platforms(),
        &[450, 480],
        &params(),
        now_ms(),
        Duration::from_secs(3600),
    )
    .await
    .unwrap();
    let before = campaign::coverage(&db.storage, id).await.unwrap();
    assert_eq!(before.runs.len(), 4);
    assert!(before.runs.iter().all(|r| r.retained == 0 && r.target == 2));

    let result = campaign::execute(&db.storage, riot.clone(), fast_options(), id, never())
        .await
        .unwrap();
    assert_eq!(result.status, CampaignStatus::Finished);

    let requested = riot.history_queues.lock().unwrap().clone();
    assert_eq!(requested.len(), 4);
    for (host, queue) in &requested {
        assert!(["450", "480"].contains(&queue.as_str()), "{host} {queue}");
    }
    assert_eq!(requested.iter().filter(|(_, q)| q == "450").count(), 2);

    let coverage = campaign::coverage(&db.storage, id).await.unwrap();
    assert_eq!(coverage.campaign_id, id);
    assert_eq!(coverage.status, "finished");
    // Chaque run ne retient que sa file : l'autre partie de l'historique est exclue.
    let summary: Vec<(&str, i32, i64, i64)> = coverage
        .runs
        .iter()
        .map(|r| (r.platform_id.as_str(), r.queue_id, r.target, r.retained))
        .collect();
    assert_eq!(
        summary,
        vec![
            ("EUW1", 450, 2, 1),
            ("EUW1", 480, 2, 1),
            ("NA1", 450, 2, 1),
            ("NA1", 480, 2, 1),
        ]
    );
    // La cible n'est pas atteinte : les runs sont « incomplets », jamais présentés comme complets.
    assert!(coverage.runs.iter().all(|r| !r.complete));
    assert!(campaign::coverage(&db.storage, id + 99).await.is_err());
    db.cleanup().await;
}

#[tokio::test]
async fn le_lancement_historique_reste_en_toutes_files() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let id = campaign::start(
        &db.storage,
        &platforms(),
        &RunParams {
            queue_id: 0,
            ..params()
        },
        now_ms(),
        Duration::from_secs(3600),
    )
    .await
    .unwrap();
    let coverage = campaign::coverage(&db.storage, id).await.unwrap();
    assert_eq!(coverage.runs.len(), 2);
    assert!(coverage.runs.iter().all(|r| r.queue_id == 0));
    db.cleanup().await;
}
