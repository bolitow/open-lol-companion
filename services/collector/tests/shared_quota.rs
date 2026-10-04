//! Coordination réelle de plusieurs transports et persistance après abandon.
mod common;
use common::TestDb;
use olc_collector::{
    riot_client::{RawResponse, Request, Transport, TransportError},
    shared_quota::{CoordinatedTransport, Priority},
};
use serde_json::json;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Notify;

#[derive(Clone, Default)]
struct Recording {
    sent: Arc<Mutex<Vec<Instant>>>,
    response: RawResponse,
    entered: Option<Arc<Notify>>,
}
impl Transport for Recording {
    async fn send(&self, _: &Request) -> Result<RawResponse, TransportError> {
        self.sent.lock().unwrap().push(Instant::now());
        if let Some(entered) = &self.entered {
            entered.notify_one();
            std::future::pending::<()>().await;
        }
        Ok(self.response.clone())
    }
}
/// Fenêtre de quota des tests, bien plus longue que toute attente du test.
const LONG_WINDOW_MS: u64 = 60_000;
/// Délai d'une attente « doit finir » : large, il ne sert qu'à éviter un test suspendu.
const GENEROUS: Duration = Duration::from_secs(30);

/// Un seul appel par fenêtre de `period_ms` sur le seau de l'application.
async fn limit(db: &TestDb, period_ms: u64) {
    sqlx::query("INSERT INTO riot_shared_quota(host,state) VALUES('europe.api.riotgames.com',$1)")
        .bind(json!({"app":{"windows":[{"limit":1,"period_ms":period_ms,"sent":[]}],"blocked_until":0},"methods":{}})).execute(db.storage.pool()).await.unwrap();
}

#[tokio::test]
async fn deux_pools_partagent_les_reservations_avant_envoi() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    // Un seul appel par fenêtre de 60 s : le second pool ne peut passer qu'après la fin du test.
    limit(&db, LONG_WINDOW_MS).await;
    let fake = Recording::default();
    let first = CoordinatedTransport::new(fake.clone(), db.storage.clone(), Priority::Background);
    let other_storage = db.separate_storage().await;
    let second =
        CoordinatedTransport::new(fake.clone(), other_storage.clone(), Priority::Background);
    let tasks = [
        tokio::spawn(async move { first.send(&Request::match_detail("EUW1_1")).await }),
        tokio::spawn(async move { second.send(&Request::match_detail("EUW1_2")).await }),
    ];
    let finished =
        |tasks: &[tokio::task::JoinHandle<_>]| tasks.iter().filter(|t| t.is_finished()).count();
    // Un seul des deux pools obtient la réservation et envoie ; la durée que cela prend
    // n'a aucune importance, seul compte le dénombrement.
    tokio::time::timeout(GENEROUS, async {
        while finished(&tasks) < 1 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    // L'autre pool voit la réservation de son pair en base : il attend la fenêtre (60 s).
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(finished(&tasks), 1);
    assert_eq!(fake.sent.lock().unwrap().len(), 1);
    for task in tasks {
        task.abort();
        let _ = task.await;
    }
    other_storage.pool().close().await;
    db.cleanup().await;
}

#[tokio::test]
async fn annuler_un_envoi_ne_rend_pas_son_quota() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    limit(&db, LONG_WINDOW_MS).await;
    let entered = Arc::new(Notify::new());
    let fake = Recording {
        entered: Some(entered.clone()),
        ..Recording::default()
    };
    let first = CoordinatedTransport::new(fake.clone(), db.storage.clone(), Priority::Background);
    let task = tokio::spawn(async move { first.send(&Request::match_detail("EUW1_1")).await });
    tokio::time::timeout(GENEROUS, entered.notified())
        .await
        .unwrap();
    task.abort();
    let _ = task.await;
    let second = CoordinatedTransport::new(
        Recording::default(),
        db.storage.clone(),
        Priority::Background,
    );
    assert!(tokio::time::timeout(
        Duration::from_millis(50),
        second.send(&Request::match_detail("EUW1_2"))
    )
    .await
    .is_err());
    let count: i64 = sqlx::query_scalar(
        "SELECT jsonb_array_length(state#>'{app,windows,0,sent}')::bigint FROM riot_shared_quota",
    )
    .fetch_one(db.storage.pool())
    .await
    .unwrap();
    assert_eq!(count, 1);
    db.cleanup().await;
}

#[tokio::test]
async fn un_429_est_partage_avec_un_nouveau_processus() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let fake = Recording {
        response: RawResponse {
            status: 429,
            headers: vec![
                ("retry-after".into(), "60".into()),
                ("x-rate-limit-type".into(), "application".into()),
            ],
            body: vec![],
        },
        ..Recording::default()
    };
    let first = CoordinatedTransport::new(fake, db.storage.clone(), Priority::Background);
    first.send(&Request::match_detail("EUW1_1")).await.unwrap();
    let next = Recording::default();
    let second = CoordinatedTransport::new(next.clone(), db.storage.clone(), Priority::Background);
    assert!(tokio::time::timeout(
        Duration::from_millis(100),
        second.send(&Request::match_detail("EUW1_2"))
    )
    .await
    .is_err());
    assert!(next.sent.lock().unwrap().is_empty());
    db.cleanup().await;
}

#[tokio::test]
async fn une_requete_interactive_passe_pendant_une_rafale_du_collecteur() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    // 10 appels par fenêtre de 60 s : le collecteur plafonne à 8, deux appels restent réservés.
    // La fenêtre est bien plus longue que le test : aucune marge temporelle ne dépend de la CI.
    sqlx::query("INSERT INTO riot_shared_quota(host,state) VALUES('europe.api.riotgames.com',$1)")
        .bind(json!({"app":{"windows":[{"limit":10,"period_ms":LONG_WINDOW_MS,"sent":[]}],"blocked_until":0},"methods":{}}))
        .execute(db.storage.pool())
        .await
        .unwrap();
    let background = Recording::default();
    let collector = Arc::new(CoordinatedTransport::new(
        background.clone(),
        db.storage.clone(),
        Priority::Background,
    ));
    let burst: Vec<_> = (0..12)
        .map(|i| {
            let collector = collector.clone();
            tokio::spawn(async move {
                collector
                    .send(&Request::match_detail(&format!("EUW1_{i}")))
                    .await
            })
        })
        .collect();
    // La rafale a rempli la part du collecteur ; les quatre autres appels attendent la fenêtre.
    tokio::time::timeout(GENEROUS, async {
        while background.sent.lock().unwrap().len() < 8 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let interactive = Recording::default();
    let api = CoordinatedTransport::new(
        interactive.clone(),
        db.storage.clone(),
        Priority::Interactive,
    );
    // Sans part réservée, ces appels attendraient le glissement de la fenêtre (60 s) et le
    // délai généreux expirerait : seul le contenu de la réserve les fait passer.
    for i in 0..2 {
        tokio::time::timeout(
            GENEROUS,
            api.send(&Request::match_detail(&format!("EUW1_api{i}"))),
        )
        .await
        .expect("la réserve interactive doit répondre sans attendre la fenêtre")
        .unwrap();
    }
    assert_eq!(interactive.sent.lock().unwrap().len(), 2);
    // Le plafond global de la clé est atteint : même l'interactif attend désormais.
    assert!(tokio::time::timeout(
        Duration::from_millis(200),
        api.send(&Request::match_detail("EUW1_api_extra"))
    )
    .await
    .is_err());
    // La rafale n'a rien pu envoyer de plus : 10 appels au total, jamais davantage.
    assert_eq!(background.sent.lock().unwrap().len(), 8);
    assert_eq!(interactive.sent.lock().unwrap().len(), 2);
    for task in burst {
        task.abort();
        let _ = task.await;
    }
    db.cleanup().await;
}
