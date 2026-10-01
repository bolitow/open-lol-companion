//! Échanges WebSocket sur une socket locale réelle, sans client LoL ni Riot.
use futures_util::{SinkExt, StreamExt};
use olc_api::{
    auth::Auth,
    realtime::Publication,
    server::{router, AppState},
};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::test]
async fn authentifie_notifie_une_publication_et_ferme_a_expiration() {
    let auth = Auth::new(b"synthetic-signing-material-for-ws-tests", "test", "api").unwrap();
    let token = auth
        .issue("session", jsonwebtoken::get_current_timestamp(), 2)
        .unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let state = AppState::new(pool, auth);
    let publications = state.publications.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    let (mut ws, _) = connect_async(format!("ws://{address}/v1/ws"))
        .await
        .unwrap();
    ws.send(Message::Text(
        serde_json::json!({"type":"authenticate","token":token})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    let first = tokio::time::timeout(Duration::from_secs(1), ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let first: serde_json::Value = serde_json::from_str(first.to_text().unwrap()).unwrap();
    assert_eq!(first["type"], "data.updated");
    publications.send_replace(Publication {
        stats_version: Some("new-snapshot".into()),
        available: true,
        ..Publication::default()
    });
    let second = tokio::time::timeout(Duration::from_secs(1), ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(second.to_text().unwrap().contains("new-snapshot"));
    let close = tokio::time::timeout(Duration::from_secs(4), ws.next())
        .await
        .unwrap();
    assert!(matches!(close, Some(Ok(Message::Close(_))) | None));
    server.abort();
}

#[tokio::test]
async fn un_jeton_invalide_ne_recoit_aucune_publication() {
    let state = AppState::new(
        PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
        Auth::new(b"synthetic-signing-material-for-ws-tests", "test", "api").unwrap(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    let (mut ws, _) = connect_async(format!("ws://{address}/v1/ws"))
        .await
        .unwrap();
    ws.send(Message::Text(
        r#"{"type":"authenticate","token":"invalid"}"#.into(),
    ))
    .await
    .unwrap();
    let reply = tokio::time::timeout(Duration::from_secs(2), ws.next())
        .await
        .unwrap();
    assert!(matches!(reply, Some(Ok(Message::Close(_))) | None));
    server.abort();
}
