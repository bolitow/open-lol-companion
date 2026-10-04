//! Canal WebSocket `/v1/ws` : annonce des nouvelles publications de données.
//!
//! Le serveur ne transporte que des dates de version ; l'app relit ensuite les
//! ressources REST. Aucun contenu de jeu, aucune donnée personnelle sur ce flux.

use crate::BuildClient;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_tungstenite::{
    tungstenite::{
        client::IntoClientRequest,
        protocol::{frame::coding::CloseCode, WebSocketConfig},
        Error as WsError, Message,
    },
    Connector,
};

/// Connexion, authentification et premier message : au-delà, le serveur est considéré injoignable.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Même plafond que le serveur : un message annonce deux dates, jamais de contenu.
const MAX_MESSAGE_BYTES: usize = 8192;
const MAX_VERSION_LEN: usize = 64;

/// Message serveur, miroir de `Publication` dans @olc/shared.
/// Les champs inconnus sont ignorés : le serveur peut en ajouter sans casser le desktop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Publication {
    #[serde(rename = "type")]
    pub kind: String,
    pub stats_version: Option<String>,
    pub static_version: Option<String>,
    pub available: bool,
}

/// État de la connexion, miroir de `PublicationStatus` dans @olc/shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationStatus {
    NotConfigured,
    Connecting,
    Connected,
    Reconnecting,
    Unauthorized,
}

/// État lu par l'interface, miroir de `PublicationState` dans @olc/shared.
/// `revision` change à chaque nouvelle publication observée : l'interface relit alors ses builds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublicationState {
    pub revision: u64,
    pub status: PublicationStatus,
    pub publication: Option<Publication>,
}

/// Changement remonté par la boucle de connexion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicationEvent {
    Status(PublicationStatus),
    Received(Publication),
}

/// Délais de reconnexion : repli exponentiel borné, jamais d'attente illimitée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectPolicy {
    pub initial: Duration,
    pub max: Duration,
    /// Silence toléré avant de considérer la connexion perdue (le serveur envoie un ping toutes les 30 s).
    pub idle: Duration,
}

impl Default for PublicationState {
    fn default() -> Self {
        Self {
            revision: 0,
            status: PublicationStatus::NotConfigured,
            publication: None,
        }
    }
}
impl PublicationState {
    /// Applique un événement ; renvoie `true` si l'état visible a changé.
    pub fn apply(&mut self, event: PublicationEvent) -> bool {
        match event {
            PublicationEvent::Status(status) => {
                let changed = self.status != status;
                self.status = status;
                changed
            }
            PublicationEvent::Received(publication) => {
                let new_version = self.publication.as_ref() != Some(&publication);
                let changed = new_version || self.status != PublicationStatus::Connected;
                if new_version {
                    self.revision = self.revision.saturating_add(1);
                    self.publication = Some(publication);
                }
                self.status = PublicationStatus::Connected;
                changed
            }
        }
    }
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial: Duration::from_secs(1),
            max: Duration::from_secs(60),
            idle: Duration::from_secs(90),
        }
    }
}
impl ReconnectPolicy {
    /// Attente avant la tentative suivante (`attempt` = échecs consécutifs déjà subis, dès 0).
    pub fn delay(&self, attempt: u32) -> Duration {
        self.initial
            .saturating_mul(2u32.saturating_pow(attempt.min(31)))
            .min(self.max)
    }
}

/// Décision pure du délai de reconnexion, appelée par la boucle après chaque session.
#[derive(Debug, Clone, Copy)]
struct Backoff {
    policy: ReconnectPolicy,
    /// Échecs consécutifs déjà subis.
    failures: u32,
}
impl Backoff {
    fn new(policy: ReconnectPolicy) -> Self {
        Self {
            policy,
            failures: 0,
        }
    }

    /// Délai avant la tentative suivante, `session` étant la durée de la session qui vient de se terminer.
    /// Une connexion tenue au moins `max` remet le repli à zéro ; une qui s'écroule aussitôt non.
    fn next_delay(&mut self, session: Duration) -> Duration {
        if session >= self.policy.max {
            self.failures = 0;
        }
        let delay = self.policy.delay(self.failures);
        self.failures = self.failures.saturating_add(1);
        delay
    }
}

/// Ne remonte un statut que s'il change, pour ne pas répéter « reconnexion » à chaque échec.
struct Emitter<F> {
    emit: F,
    last: Option<PublicationStatus>,
}
impl<F: FnMut(PublicationEvent)> Emitter<F> {
    fn status(&mut self, status: PublicationStatus) {
        if self.last != Some(status) {
            self.last = Some(status);
            (self.emit)(PublicationEvent::Status(status));
        }
    }
    fn received(&mut self, publication: Publication) {
        self.last = Some(PublicationStatus::Connected);
        (self.emit)(PublicationEvent::Received(publication));
    }
}

enum Ended {
    /// Coupure ou message invalide : on retente avec repli.
    Lost,
    /// Jeton refusé ou expiré : il est fixe, réessayer ne servirait à rien.
    Unauthorized,
}

fn parse(text: &str) -> Option<Publication> {
    let publication: Publication = serde_json::from_str(text).ok()?;
    let valid_version =
        |v: &Option<String>| v.as_ref().map_or(true, |v| v.len() <= MAX_VERSION_LEN);
    (publication.kind == "data.updated"
        && valid_version(&publication.stats_version)
        && valid_version(&publication.static_version))
    .then_some(publication)
}

impl BuildClient {
    /// `https` devient `wss`, le loopback `http` devient `ws` ; jamais de jeton dans l'URL.
    fn websocket_url(&self) -> Option<String> {
        let mut url = self.base.join("v1/ws").ok()?;
        url.set_scheme(if self.base.scheme() == "https" {
            "wss"
        } else {
            "ws"
        })
        .ok()?;
        Some(url.into())
    }

    /// Écoute les publications jusqu'au refus du jeton ; sinon reconnecte sans fin,
    /// avec un délai doublé à chaque échec jusqu'au plafond de la politique.
    pub async fn watch_publications(
        &self,
        policy: ReconnectPolicy,
        on_event: impl FnMut(PublicationEvent) + Send,
    ) {
        let mut out = Emitter {
            emit: on_event,
            last: None,
        };
        out.status(PublicationStatus::Connecting);
        let mut backoff = Backoff::new(policy);
        loop {
            let started = Instant::now();
            if let Ended::Unauthorized = self.session(&policy, &mut out).await {
                out.status(PublicationStatus::Unauthorized);
                return;
            }
            out.status(PublicationStatus::Reconnecting);
            tokio::time::sleep(backoff.next_delay(started.elapsed())).await;
        }
    }

    async fn session(
        &self,
        policy: &ReconnectPolicy,
        out: &mut Emitter<impl FnMut(PublicationEvent)>,
    ) -> Ended {
        let Some(request) = self
            .websocket_url()
            .and_then(|url| url.into_client_request().ok())
        else {
            return Ended::Lost;
        };
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_MESSAGE_BYTES))
            .max_frame_size(Some(MAX_MESSAGE_BYTES));
        let connection = tokio::time::timeout(
            CONNECT_TIMEOUT,
            tokio_tungstenite::connect_async_tls_with_config(
                request,
                Some(config),
                false,
                Some(Connector::Rustls(self.tls.clone())),
            ),
        )
        .await;
        let mut socket = match connection {
            Ok(Ok((socket, _))) => socket,
            Ok(Err(WsError::Http(response))) if matches!(response.status().as_u16(), 401 | 403) => {
                return Ended::Unauthorized
            }
            _ => return Ended::Lost,
        };
        let authenticate = serde_json::json!({"type": "authenticate", "token": self.token});
        let sent = tokio::time::timeout(
            CONNECT_TIMEOUT,
            socket.send(Message::Text(authenticate.to_string().into())),
        )
        .await;
        if !matches!(sent, Ok(Ok(()))) {
            return Ended::Lost;
        }
        let mut first = true;
        loop {
            let wait = if first { CONNECT_TIMEOUT } else { policy.idle };
            let Ok(Some(Ok(message))) = tokio::time::timeout(wait, socket.next()).await else {
                return Ended::Lost;
            };
            match message {
                Message::Text(text) => {
                    let Some(publication) = parse(&text) else {
                        return Ended::Lost;
                    };
                    first = false;
                    out.received(publication);
                }
                // Les pings du serveur sont acquittés par tungstenite à la lecture.
                Message::Ping(_) | Message::Pong(_) => {}
                Message::Close(frame) => {
                    return match frame {
                        Some(frame) if frame.code == CloseCode::Policy => Ended::Unauthorized,
                        _ => Ended::Lost,
                    }
                }
                _ => return Ended::Lost,
            }
        }
    }
}

#[cfg(test)]
mod tests;
