use std::{sync::Arc, time::Duration};

use reqwest::header::HeaderValue;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{ring, verify_tls12_signature, verify_tls13_signature, CryptoProvider};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, ClientConfig, DigitallySignedStruct, SignatureScheme};
use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;

use crate::{Credentials, GameflowPhase};

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("réponse inattendue du client LoL : {0}")]
    UnexpectedStatus(reqwest::StatusCode),
    #[error("requête vers le client LoL impossible : {0}")]
    Http(#[from] reqwest::Error),
    #[error("WebSocket du client LoL : {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("configuration TLS invalide : {0}")]
    Tls(#[from] rustls::Error),
    #[error("en-tête d'authentification invalide")]
    Header(#[from] reqwest::header::InvalidHeaderValue),
}

/// Client HTTPS de la League Client API, lié à une session du client
/// (le port et le mot de passe changent à chaque lancement).
#[derive(Clone)]
pub struct LcuClient {
    http: reqwest::Client,
    base_url: String,
}

impl LcuClient {
    pub fn new(creds: &Credentials) -> Result<Self, ClientError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::AUTHORIZATION, auth_header(creds)?);

        let http = reqwest::Client::builder()
            .use_preconfigured_tls(tls_config()?)
            .default_headers(headers)
            // Chaque lecture et écriture doit rester sur le port du client local.
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self {
            http,
            base_url: creds.base_url(),
        })
    }

    /// `GET` d'un endpoint JSON, ex. `/lol-gameflow/v1/gameflow-phase`.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let res = self
            .http
            .get(format!("{}{path}", self.base_url))
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(ClientError::UnexpectedStatus(res.status()));
        }
        Ok(res.json().await?)
    }

    /// Phase de jeu courante.
    pub async fn gameflow_phase(&self) -> Result<GameflowPhase, ClientError> {
        self.get_json(GameflowPhase::ENDPOINT).await
    }

    /// Écriture JSON locale ; une réponse vide (204) constitue aussi un succès.
    pub(crate) async fn write_json<T: Serialize + ?Sized>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: &T,
    ) -> Result<(), ClientError> {
        let response = self
            .http
            .request(method, format!("{}{path}", self.base_url))
            .json(body)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(ClientError::UnexpectedStatus(response.status()));
        }
        Ok(())
    }
}

/// En-tête `Authorization` commun au HTTP et au WebSocket, masqué dans les logs des librairies.
pub(crate) fn auth_header(creds: &Credentials) -> Result<HeaderValue, ClientError> {
    let mut value = HeaderValue::from_str(&creds.authorization_header())?;
    value.set_sensitive(true);
    Ok(value)
}

/// Seul hôte pour lequel le certificat du client LoL est accepté (voir `Credentials::base_url`).
const LOCALHOST: &str = "127.0.0.1";

/// Configuration TLS commune au client HTTPS et au WebSocket.
///
/// Le client LoL présente un certificat auto-signé par Riot : on l'accepte sans
/// vérifier sa chaîne, mais uniquement pour `127.0.0.1`. Toute autre destination est refusée.
pub(crate) fn tls_config() -> Result<ClientConfig, ClientError> {
    let provider = Arc::new(ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(LocalClientVerifier(provider)))
        .with_no_client_auth();
    Ok(config)
}

/// Accepte le certificat du client local (et lui seul), en vérifiant quand même les signatures de la poignée de main.
#[derive(Debug)]
struct LocalClientVerifier(Arc<CryptoProvider>);

impl ServerCertVerifier for LocalClientVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if server_name.to_str() == LOCALHOST {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(
                CertificateError::NotValidForName,
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imports::ImportError;
    use crate::test_support::{mock_client, ExpectedRequest};
    use serde_json::json;

    #[tokio::test]
    async fn refuse_les_redirections_des_imports_et_des_lectures() {
        use std::time::Duration;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        for write in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let credentials =
                Credentials::from_lockfile(&format!("LeagueClient:1:{port}:test:http")).unwrap();
            let client = LcuClient::new(&credentials).unwrap();
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = [0; 4096];
                assert!(stream.read(&mut request).await.unwrap() > 0);
                let response = format!("HTTP/1.1 307 Temporary Redirect\r\nlocation: http://127.0.0.1:{port}/forbidden\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
                stream.write_all(response.as_bytes()).await.unwrap();
                drop(stream);
                if let Ok(Ok((mut stream, _))) =
                    tokio::time::timeout(Duration::from_millis(200), listener.accept()).await
                {
                    assert!(stream.read(&mut request).await.unwrap() > 0);
                    stream.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 4\r\nconnection: close\r\n\r\nnull").await.unwrap();
                    true
                } else {
                    false
                }
            });
            let result = if write {
                client
                    .write_json(reqwest::Method::PATCH, "/redirect", &json!({}))
                    .await
            } else {
                client
                    .get_json::<serde_json::Value>("/redirect")
                    .await
                    .map(|_| ())
            };
            assert!(!server.await.unwrap(), "une requête a suivi la redirection");
            assert!(
                matches!(result, Err(ClientError::UnexpectedStatus(status)) if status.as_u16() == 307)
            );
        }
    }

    #[tokio::test]
    async fn ecrit_le_json_et_accepte_une_reponse_vide() {
        let (client, server) = mock_client(vec![ExpectedRequest {
            method: "PATCH",
            path: "/selection".into(),
            body: Some(json!({"spell1Id":4,"spell2Id":14})),
            status: 204,
            response: serde_json::Value::Null,
        }])
        .await;
        assert!(client
            .write_json(
                reqwest::Method::PATCH,
                "/selection",
                &json!({"spell1Id":4,"spell2Id":14})
            )
            .await
            .is_ok());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn refuse_les_echecs_http_sans_exposer_le_corps() {
        let (client, server) = mock_client(vec![ExpectedRequest {
            method: "PUT",
            path: "/pages/1".into(),
            body: Some(json!({})),
            status: 400,
            response: json!({"message":"réponse interne à ne pas transmettre"}),
        }])
        .await;
        let error = client
            .write_json(reqwest::Method::PUT, "/pages/1", &json!({}))
            .await
            .unwrap_err();
        assert_eq!(ImportError::from(error), ImportError::ClientRejected);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn distingue_une_reponse_lcu_malformee() {
        let (client, server) = mock_client(vec![ExpectedRequest {
            method: "GET",
            path: "/number".into(),
            body: None,
            status: 200,
            response: json!("nombre attendu"),
        }])
        .await;
        let error = client.get_json::<u32>("/number").await.unwrap_err();
        assert_eq!(ImportError::from(error), ImportError::InvalidClientData);
        server.await.unwrap();
    }

    #[test]
    fn construit_le_client_https() {
        let creds = Credentials::from_lockfile("LeagueClient:1:2:secret:https").unwrap();
        let client = LcuClient::new(&creds).unwrap();
        assert_eq!(client.base_url, "https://127.0.0.1:2");
    }

    #[test]
    fn n_accepte_le_certificat_que_pour_la_machine_locale() {
        let verifier = LocalClientVerifier(Arc::new(ring::default_provider()));
        let verify = |host: &'static str| {
            let name = ServerName::try_from(host).unwrap();
            let cert = CertificateDer::from(Vec::new());
            verifier.verify_server_cert(&cert, &[], &name, &[], UnixTime::now())
        };
        assert!(verify("127.0.0.1").is_ok());
        assert!(verify("10.0.0.1").is_err());
        assert!(verify("localhost").is_err());
        assert!(verify("example.com").is_err());
    }
}
