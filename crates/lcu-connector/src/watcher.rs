use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};

use crate::client::{tls_config, ClientError, LcuClient};
use crate::wamp::{parse_event, subscribe_message};
use crate::{discover, Credentials, GameflowPhase};

/// Délai entre deux tentatives de détection du client.
pub const RETRY_DELAY: Duration = Duration::from_secs(2);

/// Ce que le connecteur signale à l'app. Sérialisé en `{"type": "phaseChanged", "phase": "Lobby"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LcuEvent {
    Connected { port: u16 },
    Disconnected,
    PhaseChanged { phase: GameflowPhase },
}

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Surveille le client LoL tant que `tx` est ouvert : détection, connexion,
/// suivi de la phase, puis reconnexion après une fermeture ou un redémarrage du client.
pub async fn watch(tx: mpsc::Sender<LcuEvent>) {
    while !tx.is_closed() {
        // `discover` lit des fichiers et lance un processus : hors du runtime async.
        if let Ok(Ok(creds)) = tokio::task::spawn_blocking(discover).await {
            run_session(&creds, &tx).await;
        }
        tokio::time::sleep(RETRY_DELAY).await;
    }
}

/// Une session complète avec un client lancé. Rien n'est émis si la connexion échoue
/// (client en cours de démarrage, lockfile périmé) : `watch` réessaiera.
async fn run_session(creds: &Credentials, tx: &mpsc::Sender<LcuEvent>) {
    let Ok((mut socket, phase)) = open_session(creds).await else {
        return;
    };
    let _ = tx.send(LcuEvent::Connected { port: creds.port }).await;
    let _ = tx.send(LcuEvent::PhaseChanged { phase }).await;

    while let Some(Ok(message)) = socket.next().await {
        let Message::Text(text) = message else {
            continue;
        };
        if let Some(phase) = parse_event(&text).and_then(|e| GameflowPhase::from_event(&e)) {
            if tx.send(LcuEvent::PhaseChanged { phase }).await.is_err() {
                return;
            }
        }
    }
    let _ = tx.send(LcuEvent::Disconnected).await;
}

/// Ouvre le WebSocket et s'abonne avant de lire la phase courante, pour ne rater aucun changement.
async fn open_session(creds: &Credentials) -> Result<(Socket, GameflowPhase), ClientError> {
    let mut request = creds.websocket_url().into_client_request()?;
    let mut auth = HeaderValue::from_str(&creds.authorization_header())
        .expect("en-tête base64 toujours valide");
    auth.set_sensitive(true);
    request.headers_mut().insert("Authorization", auth);

    let connector = Connector::Rustls(tls_config());
    let (mut socket, _) =
        tokio_tungstenite::connect_async_tls_with_config(request, None, false, Some(connector))
            .await?;
    socket
        .send(Message::text(subscribe_message(GameflowPhase::ENDPOINT)))
        .await?;

    let phase = LcuClient::new(creds)?.gameflow_phase().await?;
    Ok((socket, phase))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

    #[test]
    fn serialise_les_evenements_pour_l_interface() {
        let json = serde_json::to_string(&LcuEvent::PhaseChanged {
            phase: GameflowPhase::Lobby,
        });
        assert_eq!(json.unwrap(), r#"{"type":"phaseChanged","phase":"Lobby"}"#);
        let json = serde_json::to_string(&LcuEvent::Connected { port: 1 });
        assert_eq!(json.unwrap(), r#"{"type":"connected","port":1}"#);
    }

    /// Faux client LoL en clair (sans TLS) : WebSocket puis requête HTTP sur le même port.
    // Le type d'erreur du rappel est imposé par tungstenite.
    #[allow(clippy::result_large_err)]
    #[tokio::test]
    async fn suit_la_phase_puis_signale_la_deconnexion() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let creds =
            Credentials::from_lockfile(&format!("LeagueClient:1:{port}:secret:http")).unwrap();
        let expected_auth = creds.authorization_header();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws =
                tokio_tungstenite::accept_hdr_async(stream, |req: &Request, res: Response| {
                    assert_eq!(req.headers()["authorization"], expected_auth.as_str());
                    Ok(res)
                })
                .await
                .unwrap();
            let subscribe = ws.next().await.unwrap().unwrap();
            assert_eq!(
                subscribe.to_text().unwrap(),
                subscribe_message(GameflowPhase::ENDPOINT)
            );

            let (mut http, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 2048];
            let n = http.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_lowercase();
            assert!(request.starts_with("get /lol-gameflow/v1/gameflow-phase "));
            assert!(request.contains(&format!("authorization: {}", expected_auth.to_lowercase())));
            let body = r#""Lobby""#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            http.write_all(response.as_bytes()).await.unwrap();

            let event = r#"[8,"OnJsonApiEvent_lol-gameflow_v1_gameflow-phase",{"data":"ChampSelect","eventType":"Update","uri":"/lol-gameflow/v1/gameflow-phase"}]"#;
            ws.send(Message::text(event)).await.unwrap();
            ws.close(None).await.unwrap();
        });

        let (tx, mut rx) = mpsc::channel(8);
        run_session(&creds, &tx).await;
        server.await.unwrap();
        drop(tx);

        let mut events = Vec::new();
        while let Some(e) = rx.recv().await {
            events.push(e);
        }
        assert_eq!(
            events,
            [
                LcuEvent::Connected { port },
                LcuEvent::PhaseChanged {
                    phase: GameflowPhase::Lobby
                },
                LcuEvent::PhaseChanged {
                    phase: GameflowPhase::ChampSelect
                },
                LcuEvent::Disconnected,
            ]
        );
    }

    #[tokio::test]
    async fn n_emet_rien_si_le_client_ne_repond_pas() {
        // Port fermé : lockfile périmé après un crash du client.
        let port = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let creds = Credentials::from_lockfile(&format!("LeagueClient:1:{port}:pw:http")).unwrap();
        let (tx, mut rx) = mpsc::channel(8);
        run_session(&creds, &tx).await;
        drop(tx);
        assert_eq!(rx.recv().await, None);
    }
}
