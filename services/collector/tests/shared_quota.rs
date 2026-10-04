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
async fn limit(db: &TestDb) {
    sqlx::query("INSERT INTO riot_shared_quota(host,state) VALUES('europe.api.riotgames.com',$1)")
        .bind(json!({"app":{"windows":[{"limit":1,"period_ms":300,"sent":[]}],"blocked_until":0},"methods":{}})).execute(db.storage.pool()).await.unwrap();
}

#[tokio::test]
async fn deux_pools_partagent_les_reservations_avant_envoi() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    limit(&db).await;
    let fake = Recording::default();
    let first = CoordinatedTransport::new(fake.clone(), db.storage.clone());
    let other_storage = db.separate_storage().await;
    let second = CoordinatedTransport::new(fake.clone(), other_storage.clone());
    let request = Request::match_detail("EUW1_1");
    let (a, b) = tokio::join!(first.send(&request), second.send(&request));
    a.unwrap();
    b.unwrap();
    let sent = fake.sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 2);
    assert!(sent[1].duration_since(sent[0]) >= Duration::from_millis(250));
    other_storage.pool().close().await;
    db.cleanup().await;
}

#[tokio::test]
async fn annuler_un_envoi_ne_rend_pas_son_quota() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    limit(&db).await;
    let entered = Arc::new(Notify::new());
    let fake = Recording {
        entered: Some(entered.clone()),
        ..Recording::default()
    };
    let first = CoordinatedTransport::new(fake.clone(), db.storage.clone());
    let task = tokio::spawn(async move { first.send(&Request::match_detail("EUW1_1")).await });
    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    task.abort();
    let _ = task.await;
    let second = CoordinatedTransport::new(Recording::default(), db.storage.clone());
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
                ("retry-after".into(), "1".into()),
                ("x-rate-limit-type".into(), "application".into()),
            ],
            body: vec![],
        },
        ..Recording::default()
    };
    let first = CoordinatedTransport::new(fake, db.storage.clone());
    first.send(&Request::match_detail("EUW1_1")).await.unwrap();
    let next = Recording::default();
    let second = CoordinatedTransport::new(next.clone(), db.storage.clone());
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
    // 10 appels par 1,5 s : le collecteur plafonne à 8, deux appels restent réservés.
    sqlx::query("INSERT INTO riot_shared_quota(host,state) VALUES('europe.api.riotgames.com',$1)")
        .bind(json!({"app":{"windows":[{"limit":10,"period_ms":1500,"sent":[]}],"blocked_until":0},"methods":{}}))
        .execute(db.storage.pool())
        .await
        .unwrap();
    let background = Recording::default();
    let collector = Arc::new(CoordinatedTransport::new(
        background.clone(),
        db.storage.clone(),
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
    // La rafale a rempli la part du collecteur ; l'interactif arrive ensuite.
    tokio::time::timeout(Duration::from_secs(1), async {
        while background.sent.lock().unwrap().len() < 8 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(background.sent.lock().unwrap().len(), 8);
    let interactive = Recording::default();
    let api = CoordinatedTransport::new(interactive.clone(), db.storage.clone())
        .with_priority(Priority::Interactive);
    let started = Instant::now();
    api.send(&Request::match_detail("EUW1_api")).await.unwrap();
    // Sans part réservée, l'appel attendrait le glissement de la fenêtre (1,5 s).
    assert!(started.elapsed() < Duration::from_millis(700));
    assert_eq!(interactive.sent.lock().unwrap().len(), 1);
    for task in burst {
        task.await.unwrap().unwrap();
    }
    // Le plafond global n'a jamais été dépassé : 10 appels maximum par fenêtre de 1,5 s.
    let mut sent = background.sent.lock().unwrap().clone();
    sent.extend(interactive.sent.lock().unwrap().iter().copied());
    sent.sort();
    for (i, at) in sent.iter().enumerate() {
        let in_window = sent[i..]
            .iter()
            .take_while(|other| other.duration_since(*at) < Duration::from_millis(1490))
            .count();
        assert!(in_window <= 10, "plafond Riot dépassé");
    }
    db.cleanup().await;
}
