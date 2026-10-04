//! Tests des campagnes avec des bases jetables et des réponses Riot synthétiques.
mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{fast_options, status, FakeRiot, TestDb, DAY_MS};
use olc_collector::campaign::{self, CampaignStatus};
use olc_collector::collector::now_ms;
use olc_collector::config::{Division, RunParams, Tier, DEFAULT_CAMPAIGN_QUEUES};
use olc_collector::riot_client::{Endpoint, RawResponse, Request, Transport, TransportError};

fn params() -> RunParams {
    RunParams {
        target_matches: 1,
        queue_id: 0,
        tiers: vec![Tier::Gold],
        divisions: vec![Division::I],
        seeds_per_division: 1,
        max_matches_per_seed: 2,
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

fn fake() -> FakeRiot {
    let fake = FakeRiot::default();
    fake.league_page("GOLD", "I", 1, &["seed"]);
    fake.history("seed", &["EUW1_1", "NA1_1"]);
    fake.game_with("EUW1_1", "EUW1", 450, now_ms() - DAY_MS, true);
    fake.game_with("NA1_1", "NA1", 450, now_ms() - DAY_MS, true);
    fake
}

#[tokio::test]
async fn cree_atomiquement_les_runs_avec_une_fenetre_et_une_echeance_figees() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let now = now_ms();
    let id = campaign::start(
        &db.storage,
        &platforms(),
        &params(),
        now,
        Duration::from_secs(3600),
    )
    .await
    .unwrap();
    let windows: i64 = sqlx::query_scalar("SELECT count(DISTINCT (r.window_start, r.window_end)) FROM collection_runs r JOIN campaign_runs c ON c.run_id=r.id WHERE c.campaign_id=$1").bind(id).fetch_one(db.storage.pool()).await.unwrap();
    assert_eq!(windows, 1);
    assert_eq!(db.scalar("SELECT count(*) FROM campaign_runs").await, 2);
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_runs WHERE queue_id=0")
            .await,
        2
    );
    let deadline: i64 = sqlx::query_scalar("SELECT round(extract(epoch FROM deadline_at)*1000)::bigint FROM collection_campaigns WHERE id=$1").bind(id).fetch_one(db.storage.pool()).await.unwrap();
    assert_eq!(deadline, now + 3_600_000);
    assert!(campaign::start(
        &db.storage,
        &["EUW1".into(), "INVALID".into()],
        &params(),
        now,
        Duration::from_secs(3600)
    )
    .await
    .is_err());
    assert_eq!(db.scalar("SELECT count(*) FROM collection_runs").await, 2);
    assert!(campaign::start(
        &db.storage,
        &platforms(),
        &params(),
        now,
        Duration::from_secs(25 * 3600)
    )
    .await
    .is_err());
    db.cleanup().await;
}

#[tokio::test]
async fn termine_toutes_les_plateformes_et_ne_relance_pas_une_campagne_terminee() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let fake = fake();
    let id = campaign::start(
        &db.storage,
        &platforms(),
        &params(),
        now_ms(),
        Duration::from_secs(3600),
    )
    .await
    .unwrap();
    let result = campaign::execute(&db.storage, fake.clone(), fast_options(), id, never())
        .await
        .unwrap();
    assert_eq!(result.status, CampaignStatus::Finished);
    assert_eq!(result.completed_runs, 2);
    assert_eq!(result.total_runs, 2);
    let calls = fake.calls(Endpoint::Match);
    campaign::execute(&db.storage, fake.clone(), fast_options(), id, never())
        .await
        .unwrap();
    assert_eq!(fake.calls(Endpoint::Match), calls);
    db.cleanup().await;
}

#[tokio::test]
async fn une_cle_refusee_suspend_aussitot_toute_la_campagne() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let fake = fake();
    fake.script("league/GOLD/I/1", vec![Ok(status(403, &[]))]);
    let id = campaign::start(
        &db.storage,
        &platforms(),
        &params(),
        now_ms(),
        Duration::from_secs(3600),
    )
    .await
    .unwrap();
    let result = campaign::execute(&db.storage, fake.clone(), fast_options(), id, never())
        .await
        .unwrap();
    assert_eq!(result.status, CampaignStatus::Paused);
    assert_eq!(result.reason, "riot_auth_rejected");
    assert_eq!(fake.calls(Endpoint::LeagueEntries), 1);
    assert_eq!(fake.calls(Endpoint::Match), 0);
    db.cleanup().await;
}

#[tokio::test]
async fn une_echeance_depassee_ne_declenche_aucun_appel_et_ne_se_deplace_pas() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let fake = fake();
    let id = campaign::start(
        &db.storage,
        &platforms(),
        &params(),
        now_ms() - 5000,
        Duration::from_secs(1),
    )
    .await
    .unwrap();
    for _ in 0..2 {
        let result = campaign::execute(&db.storage, fake.clone(), fast_options(), id, never())
            .await
            .unwrap();
        assert_eq!(result.reason, "deadline_reached");
        assert_eq!(result.status, CampaignStatus::Paused);
    }
    assert_eq!(fake.calls(Endpoint::LeagueEntries), 0);
    db.cleanup().await;
}

#[tokio::test]
async fn des_budgets_epuises_ne_creent_pas_de_boucle_sans_fin() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let fake = fake();
    let id = campaign::start(
        &db.storage,
        &platforms(),
        &RunParams {
            call_budget: 1,
            ..params()
        },
        now_ms(),
        Duration::from_secs(3600),
    )
    .await
    .unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        campaign::execute(&db.storage, fake.clone(), fast_options(), id, never()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result.reason, "call_budgets_exhausted");
    assert_eq!(fake.calls(Endpoint::LeagueEntries), 2);
    assert_eq!(fake.calls(Endpoint::Match), 0);
    db.cleanup().await;
}

#[derive(Clone)]
struct SlowRiot {
    fake: FakeRiot,
    routes: Arc<Mutex<Vec<String>>>,
}
impl Transport for SlowRiot {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        self.routes.lock().unwrap().push(request.route.host());
        tokio::time::sleep(Duration::from_millis(30)).await;
        self.fake.send(request).await
    }
}

#[tokio::test]
async fn une_reprise_conserve_les_runs_et_passe_a_la_plateforme_suivante() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let slow = SlowRiot {
        fake: fake(),
        routes: Arc::new(Mutex::new(vec![])),
    };
    let id = campaign::start(
        &db.storage,
        &platforms(),
        &params(),
        now_ms(),
        Duration::from_secs(3600),
    )
    .await
    .unwrap();
    let mut options = fast_options();
    options.concurrency = 1;
    options.max_duration = Some(Duration::from_millis(5));
    let interrupted = campaign::execute(
        &db.storage,
        slow.clone(),
        options.clone(),
        id,
        tokio::time::sleep(Duration::from_millis(10)),
    )
    .await
    .unwrap();
    assert_eq!(interrupted.reason, "interrupted");
    let finished = campaign::execute(&db.storage, slow.clone(), options, id, never())
        .await
        .unwrap();
    assert_eq!(finished.status, CampaignStatus::Finished);
    assert_eq!(db.scalar("SELECT count(*) FROM collection_runs").await, 2);
    let routes = slow.routes.lock().unwrap().clone();
    assert_eq!(
        &routes[..2],
        &["euw1.api.riotgames.com", "na1.api.riotgames.com"]
    );
    db.cleanup().await;
}

#[tokio::test]
async fn une_erreur_de_creation_annule_aussi_les_runs_deja_crees() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    sqlx::query("CREATE FUNCTION reject_second_run() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.platform_id='NA1' THEN RAISE EXCEPTION 'panne simulée'; END IF; RETURN NEW; END $$")
        .execute(db.storage.pool()).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_second BEFORE INSERT ON collection_runs FOR EACH ROW EXECUTE FUNCTION reject_second_run()")
        .execute(db.storage.pool()).await.unwrap();
    assert!(campaign::start(
        &db.storage,
        &platforms(),
        &params(),
        now_ms(),
        Duration::from_secs(3600)
    )
    .await
    .is_err());
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_campaigns").await,
        0
    );
    assert_eq!(db.scalar("SELECT count(*) FROM collection_runs").await, 0);
    assert_eq!(db.scalar("SELECT count(*) FROM collection_jobs").await, 0);
    db.cleanup().await;
}

#[tokio::test]
async fn une_erreur_de_stockage_suspend_la_campagne_avant_la_region_suivante() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let fake = fake();
    let id = campaign::start(
        &db.storage,
        &platforms(),
        &params(),
        now_ms(),
        Duration::from_secs(3600),
    )
    .await
    .unwrap();
    sqlx::query("CREATE FUNCTION reject_seed() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'panne simulée'; END $$")
        .execute(db.storage.pool()).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_seed BEFORE INSERT ON seed_players FOR EACH ROW EXECUTE FUNCTION reject_seed()")
        .execute(db.storage.pool()).await.unwrap();
    assert!(
        campaign::execute(&db.storage, fake.clone(), fast_options(), id, never())
            .await
            .is_err()
    );
    assert_eq!(db.scalar("SELECT count(*) FROM collection_campaigns WHERE status='paused' AND status_reason='error'").await, 1);
    assert_eq!(fake.calls(Endpoint::LeagueEntries), 1);
    db.cleanup().await;
}

#[tokio::test]
async fn le_delai_global_interrompt_un_run_et_la_reprise_ne_repousse_pas_la_limite() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let slow = SlowRiot {
        fake: fake(),
        routes: Arc::new(Mutex::new(vec![])),
    };
    let id = campaign::start(
        &db.storage,
        &platforms(),
        &params(),
        now_ms(),
        Duration::from_millis(80),
    )
    .await
    .unwrap();
    let mut options = fast_options();
    options.concurrency = 1;
    let outcome = tokio::time::timeout(
        Duration::from_secs(2),
        campaign::execute(&db.storage, slow.clone(), options.clone(), id, never()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(outcome.reason, "deadline_reached");
    assert_eq!(outcome.status, CampaignStatus::Paused);
    let calls = slow.routes.lock().unwrap().len();
    let resumed = campaign::execute(&db.storage, slow.clone(), options, id, never())
        .await
        .unwrap();
    assert_eq!(resumed.reason, "deadline_reached");
    assert_eq!(slow.routes.lock().unwrap().len(), calls);
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_jobs WHERE state='running'")
            .await,
        0
    );
    db.cleanup().await;
}

#[derive(Clone)]
struct HangingTimeline {
    fake: FakeRiot,
    started: Arc<tokio::sync::Notify>,
}

impl Transport for HangingTimeline {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        if request.endpoint == Endpoint::Timeline {
            self.started.notify_one();
            std::future::pending::<()>().await;
        }
        self.fake.send(request).await
    }
}

#[tokio::test]
async fn le_drainage_ne_depasse_pas_quinze_secondes_apres_l_echeance() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let started = Arc::new(tokio::sync::Notify::new());
    let transport = HangingTimeline {
        fake: fake(),
        started: started.clone(),
    };
    let id = campaign::start(
        &db.storage,
        &["EUW1".into()],
        &params(),
        now_ms(),
        Duration::from_secs(60),
    )
    .await
    .unwrap();
    let storage = db.storage.clone();
    let mut task = tokio::spawn(async move {
        campaign::execute(&storage, transport, fast_options(), id, never()).await
    });
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    // On gèle seulement après les échanges PostgreSQL et l'envoi de la timeline.
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(76)).await;
    tokio::time::resume();
    let outcome = match tokio::time::timeout(Duration::from_secs(2), &mut task).await {
        Ok(result) => result.unwrap().unwrap(),
        Err(_) => {
            task.abort();
            let _ = task.await;
            db.cleanup().await;
            panic!("la requête bloquée a dépassé le drainage maximal");
        }
    };
    assert_eq!(outcome.reason, "deadline_reached");
    assert_eq!(outcome.status, CampaignStatus::Paused);
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_jobs WHERE state='running'")
            .await,
        0
    );
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_jobs WHERE kind='timeline' AND state='pending'")
            .await,
        1
    );
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_runs WHERE status='paused'")
            .await,
        1
    );
    db.cleanup().await;
}

#[tokio::test]
async fn le_refus_auth_prime_sur_le_dernier_appel_du_budget() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let fake = fake();
    fake.script("league/GOLD/I/1", vec![Ok(status(403, &[]))]);
    let params = RunParams {
        call_budget: 1,
        ..params()
    };
    let id = campaign::start(
        &db.storage,
        &platforms(),
        &params,
        now_ms(),
        Duration::from_secs(3600),
    )
    .await
    .unwrap();
    let result = campaign::execute(&db.storage, fake.clone(), fast_options(), id, never())
        .await
        .unwrap();
    assert_eq!(result.reason, "riot_auth_rejected");
    assert_eq!(fake.calls(Endpoint::LeagueEntries), 1);
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_jobs WHERE state='running'")
            .await,
        0
    );
    db.cleanup().await;
}

#[derive(Clone)]
struct DelayedRefusal(FakeRiot);
impl Transport for DelayedRefusal {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        self.0.send(request).await
    }
}
#[tokio::test]
async fn le_refus_auth_prime_sur_la_fin_de_tranche() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let fake = fake();
    fake.script("league/GOLD/I/1", vec![Ok(status(403, &[]))]);
    let id = campaign::start(
        &db.storage,
        &platforms(),
        &params(),
        now_ms(),
        Duration::from_secs(3600),
    )
    .await
    .unwrap();
    let mut runtime = fast_options();
    runtime.max_duration = Some(Duration::from_millis(200));
    let result = campaign::execute(
        &db.storage,
        DelayedRefusal(fake.clone()),
        runtime,
        id,
        never(),
    )
    .await
    .unwrap();
    assert_eq!(result.reason, "riot_auth_rejected");
    assert_eq!(fake.calls(Endpoint::LeagueEntries), 1);
    db.cleanup().await;
}

#[tokio::test]
async fn une_campagne_cree_une_execution_par_plateforme_et_par_file() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let now = now_ms();
    let hour = Duration::from_secs(3600);
    // Défaut : Solo et Flex, rangs observés, budget de la plateforme réparti.
    let id = campaign::start_for_queues(
        &db.storage,
        &platforms(),
        &DEFAULT_CAMPAIGN_QUEUES,
        &params(),
        now,
        hour,
    )
    .await
    .unwrap();
    assert_eq!(db.scalar("SELECT count(*) FROM campaign_runs").await, 4);
    for queue in [420, 440] {
        let runs: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM collection_runs
             WHERE queue_id = $1 AND (params->>'collect_ranks')::boolean AND (params->>'call_budget')::bigint = 50",
        )
        .bind(queue)
        .fetch_one(db.storage.pool())
        .await
        .unwrap();
        assert_eq!(runs, 2, "file {queue}");
    }
    let stored: serde_json::Value =
        sqlx::query_scalar("SELECT params FROM collection_campaigns WHERE id = $1")
            .bind(id)
            .fetch_one(db.storage.pool())
            .await
            .unwrap();
    assert_eq!(stored["queues"], serde_json::json!([420, 440]));

    // Une file explicite hors Solo/Flex est acceptée : pas de rangs, budget entier.
    campaign::start_for_queues(&db.storage, &platforms(), &[450], &params(), now, hour)
        .await
        .unwrap();
    let aram: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM collection_runs
         WHERE queue_id = 450 AND NOT (params->>'collect_ranks')::boolean AND (params->>'call_budget')::bigint = 100",
    )
    .fetch_one(db.storage.pool())
    .await
    .unwrap();
    assert_eq!(aram, 2);

    // Liste incohérente : aucune exécution créée.
    let before = db.scalar("SELECT count(*) FROM collection_runs").await;
    assert!(
        campaign::start_for_queues(&db.storage, &platforms(), &[0, 420], &params(), now, hour)
            .await
            .is_err()
    );
    assert_eq!(
        db.scalar("SELECT count(*) FROM collection_runs").await,
        before
    );
    db.cleanup().await;
}
