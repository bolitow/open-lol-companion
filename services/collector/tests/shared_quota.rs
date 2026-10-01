//! Coordination réelle de plusieurs transports et persistance après abandon.
mod common;
use common::TestDb;
use olc_collector::{
    riot_client::{RawResponse, Request, Transport, TransportError},
    shared_quota::CoordinatedTransport,
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
