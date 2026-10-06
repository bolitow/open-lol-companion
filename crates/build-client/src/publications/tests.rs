use super::*;
use futures_util::{SinkExt, StreamExt};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::{net::TcpListener, sync::mpsc};
use tokio_tungstenite::tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame},
    Message,
};

#[path = "tls_tests.rs"]
mod tls_tests;

const TOKEN: &str = "test-token";

fn fast() -> ReconnectPolicy {
    ReconnectPolicy {
        initial: Duration::from_millis(10),
        max: Duration::from_millis(40),
        idle: Duration::from_millis(300),
    }
}
fn publication(stats: &str) -> Publication {
    Publication {
        kind: "data.updated".into(),
        stats_version: Some(stats.into()),
        static_version: Some("2026-10-01 12:00:00+00".into()),
        available: true,
    }
}
fn message(stats: &str) -> Message {
    Message::Text(serde_json::to_string(&publication(stats)).unwrap().into())
}
fn client(address: std::net::SocketAddr) -> Arc<BuildClient> {
    Arc::new(BuildClient::new(Some(format!("http://{address}")), Some(TOKEN.into())).unwrap())
}
/// Lance l'écoute et renvoie les événements dans l'ordre ; la tâche s'arrête avec le test.
fn watch(
    client: Arc<BuildClient>,
) -> (
    mpsc::UnboundedReceiver<PublicationEvent>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        client
            .watch_publications(fast(), move |event| {
                let _ = tx.send(event);
            })
            .await
    });
    (rx, task)
}
async fn next(rx: &mut mpsc::UnboundedReceiver<PublicationEvent>) -> PublicationEvent {
    tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .expect("événement attendu")
        .expect("canal fermé")
}
/// Accepte une connexion, vérifie l'authentification et renvoie le flux.
async fn accept(
    listener: &TcpListener,
) -> tokio_tungstenite::WebSocketStream<tokio::net::TcpStream> {
    let (stream, _) = listener.accept().await.unwrap();
    let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
    let first = ws.next().await.unwrap().unwrap();
    let first: serde_json::Value = serde_json::from_str(first.to_text().unwrap()).unwrap();
    assert_eq!(
        first,
        serde_json::json!({"type":"authenticate","token":TOKEN})
    );
    ws
}

#[test]
fn le_delai_double_puis_reste_borne() {
    let policy = ReconnectPolicy {
        initial: Duration::from_secs(1),
        max: Duration::from_secs(60),
        idle: Duration::from_secs(90),
    };
    let delays: Vec<_> = (0..9).map(|n| policy.delay(n).as_secs()).collect();
    assert_eq!(delays, [1, 2, 4, 8, 16, 32, 60, 60, 60]);
    assert_eq!(policy.delay(u32::MAX), Duration::from_secs(60));
    let default = ReconnectPolicy::default();
    assert_eq!(default.initial, Duration::from_secs(1));
    assert_eq!(default.max, Duration::from_secs(60));
    assert_eq!(default.idle, Duration::from_secs(90));
}

#[test]
fn le_repli_double_a_chaque_session_courte_puis_reste_borne() {
    let mut backoff = Backoff::new(ReconnectPolicy {
        initial: Duration::from_secs(1),
        max: Duration::from_secs(60),
        idle: Duration::from_secs(90),
    });
    let delays: Vec<_> = (0..9)
        .map(|_| backoff.next_delay(Duration::from_millis(5)).as_secs())
        .collect();
    assert_eq!(delays, [1, 2, 4, 8, 16, 32, 60, 60, 60]);
}

#[test]
fn une_session_d_au_moins_le_plafond_remet_le_repli_a_zero() {
    let policy = ReconnectPolicy {
        initial: Duration::from_secs(1),
        max: Duration::from_secs(60),
        idle: Duration::from_secs(90),
    };
    let mut backoff = Backoff::new(policy);
    for _ in 0..4 {
        backoff.next_delay(Duration::ZERO);
    }
    assert_eq!(backoff.next_delay(policy.max), policy.initial);
    assert_eq!(backoff.next_delay(Duration::ZERO), policy.initial * 2);
}

#[test]
fn une_session_plus_courte_que_le_plafond_ne_remet_pas_le_repli_a_zero() {
    let policy = ReconnectPolicy {
        initial: Duration::from_secs(1),
        max: Duration::from_secs(60),
        idle: Duration::from_secs(90),
    };
    let mut backoff = Backoff::new(policy);
    for _ in 0..3 {
        backoff.next_delay(Duration::ZERO);
    }
    let almost = policy.max - Duration::from_millis(1);
    assert_eq!(backoff.next_delay(almost), Duration::from_secs(8));
    assert_eq!(backoff.next_delay(almost), Duration::from_secs(16));
}

#[test]
fn la_revision_ne_change_qu_avec_une_nouvelle_publication() {
    let mut state = PublicationState::default();
    assert_eq!(state.status, PublicationStatus::NotConfigured);
    assert_eq!(state.revision, 0);
    assert!(state.apply(PublicationEvent::Status(PublicationStatus::Connecting)));
    assert!(!state.apply(PublicationEvent::Status(PublicationStatus::Connecting)));
    assert_eq!(state.revision, 0);
    assert!(state.apply(PublicationEvent::Received(publication("a"))));
    assert_eq!(
        (state.revision, state.status),
        (1, PublicationStatus::Connected)
    );
    // Reconnexion sans nouvelle publication : rien à relire.
    assert!(state.apply(PublicationEvent::Status(PublicationStatus::Reconnecting)));
    assert!(state.apply(PublicationEvent::Received(publication("a"))));
    assert_eq!(
        (state.revision, state.status),
        (1, PublicationStatus::Connected)
    );
    assert!(!state.apply(PublicationEvent::Received(publication("a"))));
    assert!(state.apply(PublicationEvent::Received(publication("b"))));
    assert_eq!(state.revision, 2);
    assert_eq!(state.publication, Some(publication("b")));
}

#[test]
fn l_etat_est_serialise_comme_le_miroir_typescript() {
    let mut state = PublicationState::default();
    state.apply(PublicationEvent::Received(publication("a")));
    assert_eq!(
        serde_json::to_value(&state).unwrap(),
        serde_json::json!({
            "revision": 1,
            "status": "connected",
            "publication": {
                "type": "data.updated",
                "stats_version": "a",
                "static_version": "2026-10-01 12:00:00+00",
                "available": true
            }
        })
    );
}

#[tokio::test]
async fn s_authentifie_dans_le_premier_message_et_relaie_chaque_publication() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener).await;
        ws.send(message("a")).await.unwrap();
        ws.send(message("b")).await.unwrap();
        ws.next().await; // reste ouvert jusqu'à la fin du test
    });
    let (mut rx, task) = watch(client(address));
    assert_eq!(
        next(&mut rx).await,
        PublicationEvent::Status(PublicationStatus::Connecting)
    );
    assert_eq!(
        next(&mut rx).await,
        PublicationEvent::Received(publication("a"))
    );
    assert_eq!(
        next(&mut rx).await,
        PublicationEvent::Received(publication("b"))
    );
    task.abort();
    server.abort();
}

// Le type d'erreur du rappel est imposé par tungstenite.
#[allow(clippy::result_large_err)]
#[tokio::test]
async fn le_jeton_ne_passe_jamais_dans_l_url() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut path = String::new();
        let mut ws = tokio_tungstenite::accept_hdr_async(
            stream,
            |request: &tokio_tungstenite::tungstenite::handshake::server::Request, response| {
                path = request.uri().to_string();
                assert!(!request.headers().contains_key("authorization"));
                assert!(!request.headers().contains_key("origin"));
                Ok(response)
            },
        )
        .await
        .unwrap();
        assert_eq!(path, "/v1/ws");
        ws.close(None).await.ok();
    });
    let (_rx, task) = watch(client(address));
    server.await.unwrap();
    task.abort();
}

#[tokio::test]
async fn se_reconnecte_apres_une_fermeture_sans_rejouer_une_publication_connue() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let count = accepted.clone();
    let server = tokio::spawn(async move {
        loop {
            let mut ws = accept(&listener).await;
            let n = count.fetch_add(1, Ordering::SeqCst);
            ws.send(message(if n == 0 { "a" } else { "b" }))
                .await
                .unwrap();
            ws.send(Message::Close(Some(CloseFrame {
                code: CloseCode::Away,
                reason: "shutdown".into(),
            })))
            .await
            .unwrap();
            ws.next().await;
        }
    });
    let (mut rx, task) = watch(client(address));
    let mut events = vec![];
    while events.len() < 6 {
        events.push(next(&mut rx).await);
    }
    assert_eq!(
        events,
        [
            PublicationEvent::Status(PublicationStatus::Connecting),
            PublicationEvent::Received(publication("a")),
            PublicationEvent::Status(PublicationStatus::Reconnecting),
            PublicationEvent::Received(publication("b")),
            PublicationEvent::Status(PublicationStatus::Reconnecting),
            PublicationEvent::Received(publication("b")),
        ]
    );
    assert!(accepted.load(Ordering::SeqCst) >= 3);
    task.abort();
    server.abort();
}

#[tokio::test]
async fn reessaie_avec_un_repli_quand_le_serveur_est_injoignable() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let started = std::time::Instant::now();
    let (mut rx, task) = watch(client(address));
    assert_eq!(
        next(&mut rx).await,
        PublicationEvent::Status(PublicationStatus::Connecting)
    );
    assert_eq!(
        next(&mut rx).await,
        PublicationEvent::Status(PublicationStatus::Reconnecting)
    );
    // Le serveur revient : la connexion reprend sans intervention.
    let listener = TcpListener::bind(address).await.unwrap();
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener).await;
        ws.send(message("a")).await.unwrap();
        ws.next().await;
    });
    assert_eq!(
        next(&mut rx).await,
        PublicationEvent::Received(publication("a"))
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    task.abort();
    server.abort();
}

#[tokio::test]
async fn s_arrete_quand_le_serveur_refuse_le_jeton() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let count = accepted.clone();
    let server = tokio::spawn(async move {
        loop {
            let mut ws = accept(&listener).await;
            count.fetch_add(1, Ordering::SeqCst);
            ws.send(Message::Close(Some(CloseFrame {
                code: CloseCode::Policy,
                reason: "unauthorized".into(),
            })))
            .await
            .unwrap();
            ws.next().await;
        }
    });
    let (mut rx, task) = watch(client(address));
    assert_eq!(
        next(&mut rx).await,
        PublicationEvent::Status(PublicationStatus::Connecting)
    );
    assert_eq!(
        next(&mut rx).await,
        PublicationEvent::Status(PublicationStatus::Unauthorized)
    );
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
    server.abort();
}

#[test]
fn l_url_websocket_est_chiffree_en_production_et_claire_en_loopback() {
    let url = |base: &str| {
        BuildClient::new(Some(base.into()), Some(TOKEN.into()))
            .unwrap()
            .websocket_url()
    };
    // Chemin de production : le jeton part dans le premier message, jamais en clair.
    assert_eq!(
        url("https://example.com/").as_deref(),
        Some("wss://example.com/v1/ws")
    );
    assert_eq!(
        url("http://127.0.0.1:8080/").as_deref(),
        Some("ws://127.0.0.1:8080/v1/ws")
    );
    assert_eq!(
        url("http://[::1]:8080/").as_deref(),
        Some("ws://[::1]:8080/v1/ws")
    );
}

/// Serveur qui répond `status` au handshake HTTP (comme le ferait un proxy d'authentification).
async fn refuse_handshake(listener: TcpListener, status: &'static str, attempts: Arc<AtomicUsize>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        attempts.fetch_add(1, Ordering::SeqCst);
        let mut request = Vec::new();
        let mut chunk = [0u8; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            match stream.read(&mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(n) => request.extend_from_slice(&chunk[..n]),
            }
        }
        let response =
            format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;
    }
}

#[tokio::test]
async fn s_arrete_quand_le_handshake_est_refuse_en_401_ou_403() {
    for status in ["401 Unauthorized", "403 Forbidden"] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let attempts = Arc::new(AtomicUsize::new(0));
        let server = tokio::spawn(refuse_handshake(listener, status, attempts.clone()));
        let (mut rx, task) = watch(client(address));
        assert_eq!(
            next(&mut rx).await,
            PublicationEvent::Status(PublicationStatus::Connecting)
        );
        assert_eq!(
            next(&mut rx).await,
            PublicationEvent::Status(PublicationStatus::Unauthorized),
            "{status}"
        );
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        // Plus de nouvelle tentative : l'attente de repli (10 ms ici) est largement dépassée.
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(attempts.load(Ordering::SeqCst), 1, "{status}");
        server.abort();
    }
}

#[tokio::test]
async fn un_refus_http_autre_que_401_403_est_retente_avec_repli() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(refuse_handshake(
        listener,
        "429 Too Many Requests",
        attempts.clone(),
    ));
    let (mut rx, task) = watch(client(address));
    assert_eq!(
        next(&mut rx).await,
        PublicationEvent::Status(PublicationStatus::Connecting)
    );
    assert_eq!(
        next(&mut rx).await,
        PublicationEvent::Status(PublicationStatus::Reconnecting)
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(attempts.load(Ordering::SeqCst) >= 2);
    assert!(!task.is_finished());
    task.abort();
    server.abort();
}

#[tokio::test]
async fn ignore_un_message_invalide_et_se_reconnecte() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for bad in [
            Message::Text(r#"{"type":"autre","stats_version":null,"static_version":null,"available":true}"#.into()),
            Message::Text("pas du json".into()),
            Message::Binary(vec![1, 2].into()),
            Message::Text(
                serde_json::json!({"type":"data.updated","stats_version":"x".repeat(65),"static_version":null,"available":true})
                    .to_string()
                    .into(),
            ),
        ] {
            let mut ws = accept(&listener).await;
            ws.send(bad).await.unwrap();
            ws.next().await;
        }
        let mut ws = accept(&listener).await;
        ws.send(message("ok")).await.unwrap();
        ws.next().await;
    });
    let (mut rx, task) = watch(client(address));
    let mut received = vec![];
    while received.is_empty() {
        if let PublicationEvent::Received(p) = next(&mut rx).await {
            received.push(p);
        }
    }
    assert_eq!(received, [publication("ok")]);
    task.abort();
    server.abort();
}

#[tokio::test]
async fn considere_la_connexion_perdue_apres_un_trop_long_silence() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let count = accepted.clone();
    let server = tokio::spawn(async move {
        let mut sockets = vec![];
        loop {
            let mut ws = accept(&listener).await;
            count.fetch_add(1, Ordering::SeqCst);
            ws.send(message("a")).await.unwrap();
            sockets.push(ws); // reste muet, sans fermer
        }
    });
    let (mut rx, task) = watch(client(address));
    let mut reconnecting = false;
    for _ in 0..4 {
        reconnecting |=
            next(&mut rx).await == PublicationEvent::Status(PublicationStatus::Reconnecting);
    }
    assert!(reconnecting);
    assert!(accepted.load(Ordering::SeqCst) >= 2);
    task.abort();
    server.abort();
}
