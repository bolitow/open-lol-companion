use crate::account::{read_account, ACCOUNT_ENDPOINT, REGION_ENDPOINT};
use crate::draft::{DraftMode, FLOW_ENDPOINT};
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::{BoxFuture, OptionFuture};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};

use crate::client::{auth_header, tls_config, ClientError, LcuClient};
use crate::wamp::{parse_event, subscribe_message};
use crate::{
    discover, Credentials, DraftSession, GameflowPhase, RunePage, DRAFT_ENDPOINT, RUNES_ENDPOINT,
};

/// Délai entre deux tentatives de détection du client.
pub const RETRY_DELAY: Duration = Duration::from_secs(2);

/// Ce que le connecteur signale à l'app. Sérialisé en `{"type": "phaseChanged", "phase": "Lobby"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LcuEvent {
    Connected { port: u16 },
    AccountChanged { account: Option<crate::LcuAccount> },
    Disconnected,
    DraftChanged { draft: Option<DraftSession> },
    PhaseChanged { phase: GameflowPhase },
    RunePageChanged { page: Option<RunePage> },
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
    let Ok(account_client) = LcuClient::new(creds) else {
        let _ = tx.send(LcuEvent::Disconnected).await;
        return;
    };
    let mut account = read_account(&account_client).await;
    let _ = tx
        .send(LcuEvent::AccountChanged {
            account: account.clone(),
        })
        .await;
    // Rattrape une identité indisponible au login ou un événement perdu, sans polling de l'API publique.
    let mut account_retry = tokio::time::interval(Duration::from_secs(30));
    account_retry.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    account_retry.tick().await;
    let _ = tx
        .send(LcuEvent::RunePageChanged {
            page: read_runes(creds).await,
        })
        .await;
    let mut draft_mode = DraftMode::Unsupported;
    if phase == GameflowPhase::ChampSelect {
        draft_mode = read_draft_mode(creds).await;
        let _ = tx
            .send(LcuEvent::DraftChanged {
                draft: read_draft(creds, draft_mode).await,
            })
            .await;
    }

    // La lecture périodique reste en vol pendant que les événements sont traités.
    // Une mise à jour remplace la lecture ; Delete et la fin de session l'annulent.
    let mut account_read: OptionFuture<BoxFuture<'_, Option<crate::LcuAccount>>> = None.into();
    let mut refreshing = false;
    loop {
        let message = tokio::select! {
            _ = tx.closed() => return,
            _ = account_retry.tick() => {
                if !refreshing {
                    account_read = Some(Box::pin(read_account(&account_client)) as BoxFuture<'_, _>).into();
                    refreshing = true;
                }
                continue;
            },
            result = &mut account_read, if refreshing => {
                refreshing = false;
                account_read = None.into();
                let next = result.flatten();
                if next != account {
                    account = next;
                    if tx.send(LcuEvent::AccountChanged { account: account.clone() }).await.is_err() { return; }
                }
                continue;
            },
            message = socket.next() => match message { Some(Ok(message)) => message, _ => break },
        };
        if matches!(message, Message::Close(_)) {
            break;
        }
        let Message::Text(text) = message else {
            continue;
        };
        let Some(event) = parse_event(&text) else {
            continue;
        };
        if event.uri == ACCOUNT_ENDPOINT || event.uri == REGION_ENDPOINT {
            match event.event_type.as_str() {
                "Delete" => {
                    account_read = None.into();
                    refreshing = false;
                    if account.take().is_some()
                        && tx
                            .send(LcuEvent::AccountChanged { account: None })
                            .await
                            .is_err()
                    {
                        return;
                    }
                }
                "Create" | "Update" => {
                    // Une lecture antérieure ne doit pas publier une identité périmée.
                    account_read =
                        Some(Box::pin(read_account(&account_client)) as BoxFuture<'_, _>).into();
                    refreshing = true;
                }
                _ => {}
            }
        } else if let Some(phase) = GameflowPhase::from_event(&event) {
            if tx.send(LcuEvent::PhaseChanged { phase }).await.is_err() {
                return;
            }
            draft_mode = if phase == GameflowPhase::ChampSelect {
                read_draft_mode(creds).await
            } else {
                DraftMode::Unsupported
            };
            if phase == GameflowPhase::ChampSelect {
                let _ = tx
                    .send(LcuEvent::RunePageChanged {
                        page: read_runes(creds).await,
                    })
                    .await;
                let _ = tx
                    .send(LcuEvent::DraftChanged {
                        draft: read_draft(creds, draft_mode).await,
                    })
                    .await;
            }
        } else if event.uri == FLOW_ENDPOINT {
            let next = DraftMode::from_flow(&event.data);
            if next != draft_mode {
                draft_mode = next;
                let _ = tx
                    .send(LcuEvent::DraftChanged {
                        draft: read_draft(creds, draft_mode).await,
                    })
                    .await;
            }
        } else if event.uri == RUNES_ENDPOINT {
            let page = match event.event_type.as_str() {
                "Delete" => None,
                "Create" | "Update" => RunePage::parse(event.data),
                _ => continue,
            };
            if tx.send(LcuEvent::RunePageChanged { page }).await.is_err() {
                return;
            }
        } else if event.uri == DRAFT_ENDPOINT {
            let draft = if event.event_type == "Delete" {
                None
            } else {
                DraftSession::parse_for_mode(event.data, draft_mode)
            };
            if tx.send(LcuEvent::DraftChanged { draft }).await.is_err() {
                return;
            }
        }
    }
    let _ = tx.send(LcuEvent::Disconnected).await;
}

/// Lecture seule ; absence ou erreur signifie indisponible, jamais une page par défaut.
async fn read_runes(creds: &Credentials) -> Option<RunePage> {
    let client = LcuClient::new(creds).ok()?;
    let value = tokio::time::timeout(Duration::from_secs(3), client.get_json(RUNES_ENDPOINT))
        .await
        .ok()?
        .ok()?;
    RunePage::parse(value)
}

async fn read_draft_mode(creds: &Credentials) -> DraftMode {
    let Ok(client) = LcuClient::new(creds) else {
        return DraftMode::Unsupported;
    };
    match tokio::time::timeout(Duration::from_secs(3), client.get_json(FLOW_ENDPOINT)).await {
        Ok(Ok(flow)) => DraftMode::from_flow(&flow),
        _ => DraftMode::Unsupported,
    }
}

/// Un 404 signifie que la sélection a disparu entre les deux lectures.
async fn read_draft(creds: &Credentials, mode: DraftMode) -> Option<DraftSession> {
    let client = LcuClient::new(creds).ok()?;
    let value = tokio::time::timeout(Duration::from_secs(3), client.get_json(DRAFT_ENDPOINT))
        .await
        .ok()?
        .ok()?;
    DraftSession::parse_for_mode(value, mode)
}

/// Ouvre le WebSocket et s'abonne avant de lire la phase courante, pour ne rater aucun changement.
async fn open_session(creds: &Credentials) -> Result<(Socket, GameflowPhase), ClientError> {
    let mut request = creds.websocket_url().into_client_request()?;
    request
        .headers_mut()
        .insert(AUTHORIZATION, auth_header(creds)?);

    let connector = Connector::Rustls(Arc::new(tls_config()?));
    let (mut socket, _) =
        tokio_tungstenite::connect_async_tls_with_config(request, None, false, Some(connector))
            .await?;
    // Le client Riot utilise le topic global et distribue ensuite par URI.
    socket.send(Message::text(subscribe_message(""))).await?;
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
            assert_eq!(
                ws.next().await.unwrap().unwrap().to_text().unwrap(),
                subscribe_message("")
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

            let (mut account_http, _) = listener.accept().await.unwrap();
            let n = account_http.read(&mut buf).await.unwrap();
            assert!(String::from_utf8_lossy(&buf[..n])
                .starts_with("GET /lol-summoner/v1/current-summoner "));
            account_http
                .write_all(
                    b"HTTP/1.1 404 Not Found\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}",
                )
                .await
                .unwrap();

            for iteration in 0..2 {
                if iteration == 1 {
                    let (mut flow_http, _) = listener.accept().await.unwrap();
                    let n = flow_http.read(&mut buf).await.unwrap();
                    assert!(String::from_utf8_lossy(&buf[..n])
                        .starts_with("GET /lol-gameflow/v1/session "));
                    let body = r#"{}"#; // Contexte pas encore disponible ; l’événement le précisera.
                    let response=format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",body.len());
                    flow_http.write_all(response.as_bytes()).await.unwrap();
                }
                let (mut perks_http, _) = listener.accept().await.unwrap();
                let n = perks_http.read(&mut buf).await.unwrap();
                assert!(String::from_utf8_lossy(&buf[..n])
                    .starts_with("GET /lol-perks/v1/currentpage "));
                let body = r#"{"primaryStyleId":8000,"subStyleId":8200,"selectedPerkIds":[],"isValid":false,"isTemporary":false}"#;
                let response=format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",body.len());
                perks_http.write_all(response.as_bytes()).await.unwrap();
                // La seconde lecture est déclenchée par l'entrée en sélection.
                if iteration == 0 {
                    ws.send(Message::text(r#"[8,"OnJsonApiEvent",{"data":"ChampSelect","eventType":"Update","uri":"/lol-gameflow/v1/gameflow-phase"}]"#)).await.unwrap();
                }
            }
            let (mut draft_http, _) = listener.accept().await.unwrap();
            let n = draft_http.read(&mut buf).await.unwrap();
            assert!(
                String::from_utf8_lossy(&buf[..n]).starts_with("GET /lol-champ-select/v1/session ")
            );
            let body = include_str!("../tests/fixtures/champ-select-public.json");
            let response=format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",body.len());
            draft_http.write_all(response.as_bytes()).await.unwrap();
            // Le gameflow peut préciser le mode après l'entrée en sélection.
            let custom = serde_json::json!({"phase":"ChampSelect","gameData":{"isCustomGame":true,"queue":{"id":3100}},"map":{"id":11,"gameMode":"CLASSIC"}});
            let update = serde_json::json!([8,"OnJsonApiEvent",{"uri":FLOW_ENDPOINT,"eventType":"Update","data":custom}]);
            ws.send(Message::text(update.to_string())).await.unwrap();
            let (mut refresh, _) = listener.accept().await.unwrap();
            let n = refresh.read(&mut buf).await.unwrap();
            assert!(
                String::from_utf8_lossy(&buf[..n]).starts_with("GET /lol-champ-select/v1/session ")
            );
            refresh.write_all(response.as_bytes()).await.unwrap();
            // Sortir du lobby invalide aussi la classification avant un événement tardif.
            ws.send(Message::text(r#"[8,"OnJsonApiEvent",{"data":"Lobby","eventType":"Update","uri":"/lol-gameflow/v1/gameflow-phase"}]"#)).await.unwrap();
            let late = serde_json::json!([8,"OnJsonApiEvent",{"uri":DRAFT_ENDPOINT,"eventType":"Update","data":serde_json::from_str::<serde_json::Value>(body).unwrap()}]);
            ws.send(Message::text(late.to_string())).await.unwrap();
            let deleted = serde_json::json!([8,"OnJsonApiEvent_lol-champ-select_v1_session",{"uri":DRAFT_ENDPOINT,"eventType":"Delete","data":null}]);
            ws.send(Message::text(deleted.to_string())).await.unwrap();
            let updated = serde_json::json!([8,"OnJsonApiEvent",{"uri":RUNES_ENDPOINT,"eventType":"Update","data":{
                "primaryStyleId":8100,"subStyleId":8200,"selectedPerkIds":[8112,8126,8140,8106,8210,8236,5008,5008,5011],"isValid":true,"isTemporary":true
            }}]);
            ws.send(Message::text(updated.to_string())).await.unwrap();
            let deleted = serde_json::json!([8,"OnJsonApiEvent",{"uri":RUNES_ENDPOINT,"eventType":"Delete","data":null}]);
            ws.send(Message::text(deleted.to_string())).await.unwrap();
            ws.close(None).await.unwrap();
        });

        let (tx, mut rx) = mpsc::channel(32);
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
                LcuEvent::AccountChanged { account: None },
                LcuEvent::RunePageChanged {
                    page: RunePage::parse(
                        serde_json::json!({"primaryStyleId":8000,"subStyleId":8200,"selectedPerkIds":[],"isValid":false,"isTemporary":false})
                    )
                },
                LcuEvent::PhaseChanged {
                    phase: GameflowPhase::ChampSelect
                },
                LcuEvent::RunePageChanged {
                    page: RunePage::parse(
                        serde_json::json!({"primaryStyleId":8000,"subStyleId":8200,"selectedPerkIds":[],"isValid":false,"isTemporary":false})
                    )
                },
                LcuEvent::DraftChanged {
                    draft: DraftSession::parse(
                        serde_json::from_str(include_str!(
                            "../tests/fixtures/champ-select-public.json"
                        ))
                        .unwrap()
                    )
                },
                LcuEvent::DraftChanged {
                    draft: DraftSession::parse_for_mode(
                        serde_json::from_str(include_str!(
                            "../tests/fixtures/champ-select-public.json"
                        ))
                        .unwrap(),
                        DraftMode::CustomRift(Some(3100))
                    )
                },
                LcuEvent::PhaseChanged {
                    phase: GameflowPhase::Lobby
                },
                LcuEvent::DraftChanged {
                    draft: DraftSession::parse_for_mode(
                        serde_json::from_str(include_str!(
                            "../tests/fixtures/champ-select-public.json"
                        ))
                        .unwrap(),
                        DraftMode::Unsupported
                    )
                },
                LcuEvent::DraftChanged { draft: None },
                LcuEvent::RunePageChanged {
                    page: RunePage::parse(
                        serde_json::json!({"primaryStyleId":8100,"subStyleId":8200,"selectedPerkIds":[8112,8126,8140,8106,8210,8236,5008,5008,5011],"isValid":true,"isTemporary":true})
                    )
                },
                LcuEvent::RunePageChanged { page: None },
                LcuEvent::Disconnected,
            ]
        );
    }

    async fn serve_json(listener: &TcpListener, path: &str, body: serde_json::Value) {
        let (mut http, _) = listener.accept().await.unwrap();
        let mut buf = [0; 4096];
        let n = http.read(&mut buf).await.unwrap();
        assert!(String::from_utf8_lossy(&buf[..n]).starts_with(&format!("GET {path} ")));
        let body = body.to_string();
        http.write_all(format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
    }

    #[tokio::test]
    async fn suit_le_compte_initial_les_changements_et_la_suppression_via_le_websocket() {
        use serde_json::json;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let creds =
            Credentials::from_lockfile(&format!("LeagueClient:1:{port}:fixture:http")).unwrap();
        let (observed_tx, observed_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            ws.next().await.unwrap().unwrap();
            serve_json(&listener, "/lol-gameflow/v1/gameflow-phase", json!("Lobby")).await;
            for name in ["Alpha", "Beta"] {
                if name == "Beta" {
                    // Les données de l'événement ne sont pas considérées comme une lecture atomique.
                    ws.send(Message::text(json!([8,"OnJsonApiEvent",{"uri":ACCOUNT_ENDPOINT,"eventType":"Update","data":{}}]).to_string())).await.unwrap();
                }
                let summoner = json!({"gameName":name,"tagLine":"TAG","puuid":"synthetic-private"});
                serve_json(&listener, ACCOUNT_ENDPOINT, summoner.clone()).await;
                serve_json(&listener, REGION_ENDPOINT, json!({"region":"EUW"})).await;
                serve_json(&listener, ACCOUNT_ENDPOINT, summoner).await;
                if name == "Alpha" {
                    serve_json(&listener, RUNES_ENDPOINT, json!({})).await;
                }
            }
            observed_rx.await.unwrap();
            ws.send(Message::text(json!([8,"OnJsonApiEvent",{"uri":ACCOUNT_ENDPOINT,"eventType":"Delete","data":null}]).to_string())).await.unwrap();
            ws.close(None).await.unwrap();
        });
        let (tx, mut rx) = mpsc::channel(32);
        let watcher = tokio::spawn(async move { run_session(&creds, &tx).await });
        let mut observed_tx = Some(observed_tx);
        let accounts = tokio::time::timeout(Duration::from_secs(8), async {
            let mut accounts = Vec::new();
            while let Some(event) = rx.recv().await {
                if let LcuEvent::AccountChanged { account } = event {
                    if account
                        .as_ref()
                        .is_some_and(|account| account.game_name == "Beta")
                    {
                        observed_tx.take().unwrap().send(()).unwrap();
                    }
                    accounts.push(account);
                }
            }
            accounts
        })
        .await
        .unwrap();
        watcher.await.unwrap();
        server.await.unwrap();
        assert_eq!(
            accounts,
            vec![
                Some(crate::LcuAccount {
                    platform: "EUW1".into(),
                    game_name: "Alpha".into(),
                    tag_line: "TAG".into(),
                    profile_icon_id: None
                }),
                Some(crate::LcuAccount {
                    platform: "EUW1".into(),
                    game_name: "Beta".into(),
                    tag_line: "TAG".into(),
                    profile_icon_id: None
                }),
                None
            ]
        );
    }

    #[tokio::test]
    async fn le_rattrapage_du_compte_ne_bloque_pas_les_evenements_de_phase() {
        use serde_json::json;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let creds =
            Credentials::from_lockfile(&format!("LeagueClient:1:{port}:fixture:http")).unwrap();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let (received_tx, received_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            ws.next().await.unwrap().unwrap();
            serve_json(&listener, "/lol-gameflow/v1/gameflow-phase", json!("Lobby")).await;
            serve_json(&listener, ACCOUNT_ENDPOINT, json!({})).await;
            // read_account demande la région avant de valider l'identité.
            serve_json(&listener, REGION_ENDPOINT, json!({"region":"EUW"})).await;
            serve_json(&listener, RUNES_ENDPOINT, json!({})).await;
            ready_tx.send(()).unwrap();
            let (mut http, _) = listener.accept().await.unwrap();
            let mut buf = [0; 4096];
            let n = http.read(&mut buf).await.unwrap();
            assert!(
                String::from_utf8_lossy(&buf[..n]).starts_with(&format!("GET {ACCOUNT_ENDPOINT} "))
            );
            ws.send(Message::text(json!([8,"OnJsonApiEvent",{"uri":"/lol-gameflow/v1/gameflow-phase","eventType":"Update","data":"InProgress"}]).to_string())).await.unwrap();
            received_tx.send(()).unwrap();
            // HTTP reste suspendu jusqu'à la réception de la phase par le consommateur.
            release_rx.await.unwrap();
            let account = json!({"gameName":"Recovered","tagLine":"TEST"});
            let body = account.to_string();
            http.write_all(format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            serve_json(&listener, REGION_ENDPOINT, json!({"region":"EUW"})).await;
            serve_json(&listener, ACCOUNT_ENDPOINT, account).await;
            // Le consommateur ferme le canal après avoir reçu l'identité récupérée.
            let _ = ws.next().await;
        });
        let (tx, mut rx) = mpsc::channel(32);
        let watcher = tokio::spawn(async move { run_session(&creds, &tx).await });
        ready_rx.await.unwrap();
        // Avancer uniquement la période de rattrapage, sans attendre 30 secondes réelles.
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(30)).await;
        tokio::time::resume();
        tokio::time::timeout(Duration::from_secs(2), received_rx)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), async {
            while let Some(event) = rx.recv().await {
                if event
                    == (LcuEvent::PhaseChanged {
                        phase: GameflowPhase::InProgress,
                    })
                {
                    return;
                }
            }
            panic!("phase absente");
        })
        .await
        .expect("une lecture HTTP du compte ne doit pas bloquer la phase");
        release_tx.send(()).unwrap();
        let recovered = tokio::time::timeout(Duration::from_secs(2), async {
            while let Some(event) = rx.recv().await {
                if let LcuEvent::AccountChanged {
                    account: Some(account),
                } = event
                {
                    return account;
                }
            }
            panic!("compte non récupéré");
        })
        .await
        .unwrap();
        assert_eq!(recovered.game_name, "Recovered");
        assert_eq!(recovered.platform, "EUW1");
        drop(rx);
        tokio::time::timeout(Duration::from_secs(1), watcher)
            .await
            .unwrap()
            .unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn suppression_et_deconnexion_annulent_une_lecture_de_compte_en_vol() {
        use serde_json::json;
        for ending in ["delete", "disconnect", "consumer"] {
            let disconnect = ending == "disconnect";
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let creds =
                Credentials::from_lockfile(&format!("LeagueClient:1:{port}:fixture:http")).unwrap();
            let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = tokio::sync::oneshot::channel();
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                ws.next().await.unwrap().unwrap();
                serve_json(&listener, "/lol-gameflow/v1/gameflow-phase", json!("Lobby")).await;
                let account = json!({"gameName":"Old","tagLine":"TEST"});
                serve_json(&listener, ACCOUNT_ENDPOINT, account.clone()).await;
                serve_json(&listener, REGION_ENDPOINT, json!({"region":"EUW"})).await;
                serve_json(&listener, ACCOUNT_ENDPOINT, account.clone()).await;
                serve_json(&listener, RUNES_ENDPOINT, json!({})).await;
                ready_tx.send(()).unwrap();
                let (mut http, _) = listener.accept().await.unwrap();
                let mut buf = [0; 4096];
                let n = http.read(&mut buf).await.unwrap();
                assert!(String::from_utf8_lossy(&buf[..n])
                    .starts_with(&format!("GET {ACCOUNT_ENDPOINT} ")));
                started_tx.send(()).unwrap();
                if disconnect {
                    ws.close(None).await.unwrap();
                } else if ending == "delete" {
                    ws.send(Message::text(json!([8,"OnJsonApiEvent",{"uri":ACCOUNT_ENDPOINT,"eventType":"Delete","data":null}]).to_string())).await.unwrap();
                }
                release_rx.await.unwrap();
                // La requête annulée doit fermer sa connexion avant toute réponse,
                // sans dépendre de la fermeture WebSocket qui vient ensuite.
                let ended = tokio::time::timeout(Duration::from_secs(1), http.read(&mut buf))
                    .await
                    .expect("la lecture annulée doit libérer la connexion HTTP")
                    .unwrap();
                assert_eq!(ended, 0);
                let body = account.to_string();
                // Une réponse peut arriver après l'annulation ; elle ne doit plus être publiée.
                let _ = http.write_all(format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
                if !disconnect {
                    let _ = ws.close(None).await;
                }
            });
            let (tx, mut rx) = mpsc::channel(32);
            let watcher = tokio::spawn(async move { run_session(&creds, &tx).await });
            ready_rx.await.unwrap();
            tokio::time::pause();
            tokio::time::advance(Duration::from_secs(30)).await;
            tokio::time::resume();
            tokio::time::timeout(Duration::from_secs(1), started_rx)
                .await
                .unwrap()
                .unwrap();
            if ending == "consumer" {
                drop(rx);
                release_tx.send(()).unwrap();
                tokio::time::timeout(Duration::from_secs(2), watcher)
                    .await
                    .unwrap()
                    .unwrap();
                server.await.unwrap();
                continue;
            }
            let expected = if disconnect {
                LcuEvent::Disconnected
            } else {
                LcuEvent::AccountChanged { account: None }
            };
            tokio::time::timeout(Duration::from_secs(1), async {
                while let Some(event) = rx.recv().await {
                    if event == expected {
                        return;
                    }
                }
                panic!("invalidation absente");
            })
            .await
            .expect("l'invalidation doit passer avant la réponse HTTP");
            release_tx.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(2), watcher)
                .await
                .unwrap()
                .unwrap();
            while let Some(event) = rx.recv().await {
                assert!(!matches!(
                    event,
                    LcuEvent::AccountChanged { account: Some(_) }
                ));
            }
            server.await.unwrap();
        }
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
        let (tx, mut rx) = mpsc::channel(32);
        run_session(&creds, &tx).await;
        drop(tx);
        assert_eq!(rx.recv().await, None);
    }
}
