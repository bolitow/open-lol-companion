use std::future::Future;
use std::time::Duration;

use serde_json::Value;

use super::StaticError;
use crate::riot_client::public_tls_config;

const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Réponse d'un GET public ; aucun en-tête d'authentification.
#[derive(Clone)]
pub struct StaticResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Frontière réseau remplaçable par des réponses synthétiques en test.
pub trait StaticTransport: Clone + Send + Sync + 'static {
    fn get(&self, url: &str) -> impl Future<Output = Result<StaticResponse, StaticError>> + Send;
}

#[derive(Clone)]
pub(super) struct PublicHttp {
    client: reqwest::Client,
}

impl PublicHttp {
    pub fn new() -> Result<Self, StaticError> {
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(public_tls_config().map_err(|_| StaticError::Network)?)
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| StaticError::Network)?;
        Ok(Self { client })
    }
}

impl StaticTransport for PublicHttp {
    async fn get(&self, url: &str) -> Result<StaticResponse, StaticError> {
        let url = reqwest::Url::parse(url).map_err(|_| StaticError::Network)?;
        if url.scheme() != "https"
            || !matches!(
                url.host_str(),
                Some("ddragon.leagueoflegends.com" | "static.developer.riotgames.com")
            )
        {
            return Err(StaticError::Network);
        }
        let mut response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_| StaticError::Network)?;
        let status = response.status().as_u16();
        // Les corps d'erreur n'ont aucune utilité et ne doivent pas être conservés.
        if status != 200 {
            return Ok(StaticResponse {
                status,
                body: Vec::new(),
            });
        }
        if response
            .content_length()
            .is_some_and(|len| len > MAX_BODY_BYTES as u64)
        {
            return Err(StaticError::InvalidDocument);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| StaticError::Network)? {
            if body.len() + chunk.len() > MAX_BODY_BYTES {
                return Err(StaticError::InvalidDocument);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(StaticResponse { status, body })
    }
}

pub(super) async fn fetch_json<T: StaticTransport>(
    transport: T,
    url: String,
) -> Result<Value, StaticError> {
    for attempt in 0..3 {
        let result = tokio::time::timeout(Duration::from_secs(35), transport.get(&url)).await;
        match result {
            Ok(Ok(response)) if response.status == 200 => {
                if response.body.len() > MAX_BODY_BYTES {
                    return Err(StaticError::InvalidDocument);
                }
                return serde_json::from_slice(&response.body)
                    .map_err(|_| StaticError::InvalidDocument);
            }
            Ok(Ok(response))
                if response.status != 429 && !(500..=599).contains(&response.status) =>
            {
                return Err(StaticError::Network)
            }
            Ok(Err(StaticError::InvalidDocument)) => return Err(StaticError::InvalidDocument),
            _ if attempt < 2 => {
                tokio::time::sleep(Duration::from_millis(250 * (1 << attempt))).await
            }
            _ => return Err(StaticError::Network),
        }
    }
    Err(StaticError::Network)
}
