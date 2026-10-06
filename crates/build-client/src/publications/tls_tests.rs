//! Recette WSS réelle sur loopback ; aucune confiance ajoutée au système.
use super::*;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::TlsAcceptor;

const AUTHORITY: &[u8] = include_bytes!("fixtures/authority.der");
const CERTIFICATE: &[u8] = include_bytes!("fixtures/server.der");
// Clé privée de recette, publiquement distribuée et inutilisable en production.
const TEST_PRIVATE_KEY: &[u8] = include_bytes!("fixtures/server-key.der");

fn acceptor() -> TlsAcceptor {
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![CertificateDer::from(CERTIFICATE.to_vec())],
        PrivateKeyDer::try_from(TEST_PRIVATE_KEY.to_vec()).unwrap(),
    )
    .unwrap();
    TlsAcceptor::from(Arc::new(config))
}

fn tls_client(address: std::net::SocketAddr, trust_fixture: bool) -> Arc<BuildClient> {
    let mut client = BuildClient::new(
        Some(format!("https://localhost:{}", address.port())),
        Some(TOKEN.into()),
    )
    .unwrap();
    if trust_fixture {
        let mut roots = rustls::RootCertStore::empty();
        roots.add(CertificateDer::from(AUTHORITY.to_vec())).unwrap();
        // Seul le client de test connaît cette autorité ; aucun vérificateur permissif.
        client.tls = Arc::new(
            rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth(),
        );
    }
    Arc::new(client)
}

#[tokio::test]
async fn wss_authentifie_et_recoit_les_publications_sur_un_flux_tls_verifie() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let tls = acceptor().accept(stream).await.unwrap();
        let mut ws = tokio_tungstenite::accept_hdr_async(
            tls,
            |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
             response: tokio_tungstenite::tungstenite::handshake::server::Response| {
                assert_eq!(request.uri().path(), "/v1/ws");
                assert!(request.uri().query().is_none());
                assert!(!request.headers().contains_key("authorization"));
                Ok(response)
            },
        )
        .await
        .unwrap();
        let first = ws.next().await.unwrap().unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(first.to_text().unwrap()).unwrap(),
            serde_json::json!({"type":"authenticate","token":TOKEN})
        );
        ws.send(message("tls-a")).await.unwrap();
        ws.send(message("tls-b")).await.unwrap();
        ws.next().await;
    });
    let (mut events, task) = watch(tls_client(address, true));
    assert_eq!(
        next(&mut events).await,
        PublicationEvent::Status(PublicationStatus::Connecting)
    );
    assert_eq!(
        next(&mut events).await,
        PublicationEvent::Received(publication("tls-a"))
    );
    assert_eq!(
        next(&mut events).await,
        PublicationEvent::Received(publication("tls-b"))
    );
    task.abort();
    let _ = task.await;
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn wss_refuse_un_certificat_non_approuve_avant_toute_authentification() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        // Le handshake doit échouer avant de pouvoir recevoir une frame contenant le jeton.
        assert!(acceptor().accept(stream).await.is_err());
    });
    let (mut events, task) = watch(tls_client(address, false));
    assert_eq!(
        next(&mut events).await,
        PublicationEvent::Status(PublicationStatus::Connecting)
    );
    assert_eq!(
        next(&mut events).await,
        PublicationEvent::Status(PublicationStatus::Reconnecting)
    );
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    let _ = task.await;
}
