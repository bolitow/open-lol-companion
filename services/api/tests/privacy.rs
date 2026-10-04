//! Export et effacement RGPD par l'API (#99), sur PostgreSQL réel et données synthétiques.
mod common;

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use common::TestDb;
use olc_api::{
    auth::Auth,
    server::{router, AppState},
};
use olc_collector::config::RunParams;
use olc_collector::model::fixtures::match_detail;
use serde_json::Value;
use tower::ServiceExt;

async fn post(app: &Router, path: &str, token: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"puuid":"fake-puuid-1"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let body = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn un_operateur_exporte_puis_efface_les_donnees_d_un_joueur() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let pool = db.storage.pool();
    let run_id = db
        .storage
        .create_run(&RunParams::default(), 1_000_000, 2_000_000)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO matches (match_id, platform_id, queue_id, game_version, patch, game_start,
            game_duration_s, is_remake, detail, first_run_id)
         VALUES ('EUW1_7', 'EUW1', 420, '15.19.715.1234', '15.19', to_timestamp(1000), 1800, false, $1, $2)",
    )
    .bind(match_detail("EUW1_7", "EUW1", 420, 1_000_000))
    .bind(run_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO participant_rank_observations (platform_id, puuid, queue_id, status)
         VALUES ('EUW1', 'fake-puuid-1', 420, 'unranked')",
    )
    .execute(pool)
    .await
    .unwrap();

    let auth = Auth::new(
        b"synthetic-signing-material-for-privacy-tests",
        "test",
        "api",
    )
    .unwrap();
    let token = auth
        .issue("dpo", jsonwebtoken::get_current_timestamp(), 60)
        .unwrap();
    let mut state = AppState::new(pool.clone(), auth);
    state.privacy_operators.push("dpo".into());
    let app = router(state);

    let (status, export) = post(&app, "/v1/privacy/export", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(export["puuid"], "fake-puuid-1");
    assert_eq!(export["rank_observations"][0]["status"], "unranked");
    assert_eq!(export["matches"][0]["match_id"], "EUW1_7");
    assert_eq!(
        export["matches"][0]["participant"]["riotIdGameName"],
        "Joueur1"
    );
    assert!(!export.to_string().contains("fake-puuid-2"));

    let (status, erased) = post(&app, "/v1/privacy/erase", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(erased["matches"], 1);
    assert_eq!(erased["rank_observations"], 1);

    let (_, after) = post(&app, "/v1/privacy/export", &token).await;
    assert_eq!(after["matches"], serde_json::json!([]));
    assert_eq!(after["rank_observations"], serde_json::json!([]));
    db.cleanup().await;
}
