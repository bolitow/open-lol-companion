//! Notifications de publication ; les clients relisent les ressources REST concernées.
use crate::{error::ApiError, server::AppState};
use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::{HeaderMap, Uri},
    response::Response,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::time::Duration;
use tokio::sync::OwnedSemaphorePermit;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Publication {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub stats_version: Option<String>,
    pub static_version: Option<String>,
    pub available: bool,
}
impl Default for Publication {
    fn default() -> Self {
        Self {
            kind: "data.updated",
            stats_version: None,
            static_version: None,
            available: false,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum ClientMessage {
    #[serde(rename = "authenticate")]
    Authenticate { token: String },
}

pub(crate) async fn upgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    // Pas de jeton dans l'URL ; les navigateurs s'authentifient dans le premier message.
    if uri.query().is_some()
        || headers
            .get("origin")
            .is_some_and(|origin| !state.allowed_origins.contains(origin))
    {
        return Err(ApiError::InvalidRequest);
    }
    let permit = state
        .ws_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::RateLimited)?;
    Ok(ws
        .max_message_size(8192)
        .max_frame_size(8192)
        .write_buffer_size(0)
        .max_write_buffer_size(16384)
        .on_upgrade(move |socket| session(socket, state, permit)))
}

async fn send(socket: &mut WebSocket, message: Message) -> bool {
    matches!(
        tokio::time::timeout(Duration::from_secs(5), socket.send(message)).await,
        Ok(Ok(()))
    )
}
async fn publish(socket: &mut WebSocket, publication: &Publication) -> bool {
    match serde_json::to_string(publication) {
        Ok(text) => send(socket, Message::Text(text.into())).await,
        Err(_) => false,
    }
}
async fn close(socket: &mut WebSocket, code: u16, reason: &'static str) {
    send(
        socket,
        Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })),
    )
    .await;
}
async fn session(mut socket: WebSocket, state: AppState, _permit: OwnedSemaphorePermit) {
    let first = tokio::time::timeout(Duration::from_secs(5), socket.recv()).await;
    let claims = match first {
        Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str::<ClientMessage>(&text) {
            Ok(ClientMessage::Authenticate { token }) => state.auth.verify(&token).ok(),
            Err(_) => None,
        },
        _ => None,
    };
    let Some(claims) = claims else {
        close(&mut socket, 1008, "unauthorized").await;
        return;
    };
    let mut updates = state.publications.subscribe();
    let mut shutdown = state.shutdown.subscribe();
    if *shutdown.borrow() {
        close(&mut socket, 1001, "shutdown").await;
        return;
    }
    let initial = updates.borrow_and_update().clone();
    if !publish(&mut socket, &initial).await {
        return;
    }
    let expires = tokio::time::sleep(Duration::from_secs(
        claims
            .exp
            .saturating_sub(jsonwebtoken::get_current_timestamp()),
    ));
    tokio::pin!(expires);
    let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    heartbeat.tick().await;
    loop {
        tokio::select! {
            biased;
            _ = &mut expires => { close(&mut socket, 1008, "expired").await; break; }
            _ = shutdown.changed() => { close(&mut socket, 1001, "shutdown").await; break; }
            changed = updates.changed() => {
                if changed.is_err() { break; }
                let update = updates.borrow_and_update().clone();
                if !publish(&mut socket, &update).await { break; }
            }
            received = socket.recv() => match received {
                Some(Ok(Message::Pong(_))) => {},
                Some(Ok(Message::Ping(data))) => { if !send(&mut socket, Message::Pong(data)).await { break; } },
                Some(Ok(Message::Text(_) | Message::Binary(_))) => { close(&mut socket, 1008, "unsupported_message").await; break; },
                _ => break,
            },
            _ = heartbeat.tick() => { if !send(&mut socket, Message::Ping(Vec::new().into())).await { break; } },
        }
    }
}

/// Une seule lecture des dates de publication toutes les cinq secondes pour tous les clients.
pub fn start_observer(state: &AppState) -> tokio::task::JoinHandle<()> {
    let state = state.clone();
    tokio::spawn(async move {
        let mut shutdown = state.shutdown.subscribe();
        let mut ticks = tokio::time::interval(Duration::from_secs(5));
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => break,
                _ = ticks.tick() => {
                    let publication = tokio::time::timeout(Duration::from_secs(3), read_publication(&state.pool)).await.ok().and_then(Result::ok).unwrap_or_default();
                    state.publications.send_if_modified(|current| {
                        if *current == publication { false } else { *current = publication; true }
                    });
                }
            }
        }
    })
}

async fn read_publication(pool: &sqlx::PgPool) -> Result<Publication, sqlx::Error> {
    let row = sqlx::query("SELECT (SELECT published_at::text FROM champion_stats_snapshot WHERE id=1) AS stats_version, (SELECT checked_at::text FROM static_data_manifest WHERE id=1) AS static_version").fetch_one(pool).await?;
    Ok(Publication {
        kind: "data.updated",
        stats_version: row.try_get("stats_version")?,
        static_version: row.try_get("static_version")?,
        available: true,
    })
}
