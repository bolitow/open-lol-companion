use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{ring, verify_tls12_signature, verify_tls13_signature, CryptoProvider};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::{Credentials, GameflowPhase};

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("requête vers le client LoL impossible : {0}")]
    Http(#[from] reqwest::Error),
    #[error("WebSocket du client LoL : {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
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
        let mut auth = reqwest::header::HeaderValue::from_str(&creds.authorization_header())
            .expect("en-tête base64 toujours valide");
        auth.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, auth);

        let http = reqwest::Client::builder()
            .use_preconfigured_tls((*tls_config()).clone())
            .default_headers(headers)
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
        Ok(res.error_for_status()?.json().await?)
    }

    /// Phase de jeu courante.
    pub async fn gameflow_phase(&self) -> Result<GameflowPhase, ClientError> {
        self.get_json(GameflowPhase::ENDPOINT).await
    }
}

/// Configuration TLS commune au client HTTPS et au WebSocket.
///
/// Le client LoL présente un certificat auto-signé par Riot. On accepte donc le
/// certificat sans le vérifier ; c'est sans risque ici car on ne se connecte
/// qu'à `127.0.0.1` (voir `Credentials::base_url`) : rien ne transite hors de la machine.
pub(crate) fn tls_config() -> Arc<ClientConfig> {
    let provider = Arc::new(ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .expect("versions TLS par défaut supportées par ring")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(LocalClientVerifier(provider)))
        .with_no_client_auth();
    Arc::new(config)
}

/// Accepte le certificat du client local, mais vérifie quand même les signatures de la poignée de main.
#[derive(Debug)]
struct LocalClientVerifier(Arc<CryptoProvider>);

impl ServerCertVerifier for LocalClientVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
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

    #[test]
    fn construit_le_client_https() {
        let creds = Credentials::from_lockfile("LeagueClient:1:2:secret:https").unwrap();
        let client = LcuClient::new(&creds).unwrap();
        assert_eq!(client.base_url, "https://127.0.0.1:2");
    }
}
