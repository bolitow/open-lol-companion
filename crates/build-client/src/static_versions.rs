//! Projection bornée du manifeste public ; les versions statiques ne garantissent pas des builds.
use crate::{BuildClient, BuildError};
use serde::{Deserialize, Serialize};

/// Miroir de `StaticVersions` dans @olc/shared.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticVersions {
    pub live_version: String,
    pub versions: Vec<String>,
}
fn valid(version: &str) -> bool {
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 4 && p.bytes().all(|b| b.is_ascii_digit()))
}
impl BuildClient {
    /// Lecture sur l'origine API configurée seulement, sans suivre de redirection.
    pub async fn static_versions(&self) -> Result<StaticVersions, BuildError> {
        const LIMIT: usize = 256 * 1024;
        let url = self
            .base
            .join("v1/static/manifest")
            .map_err(|_| BuildError::InvalidConfiguration)?;
        let mut response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|_| BuildError::Unavailable)?;
        match response.status().as_u16() {
            200 => {}
            401 | 403 => return Err(BuildError::Unauthorized),
            429 => return Err(BuildError::RateLimited),
            _ => return Err(BuildError::Unavailable),
        }
        if response.content_length().is_some_and(|n| n > LIMIT as u64) {
            return Err(BuildError::InvalidResponse);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| BuildError::Unavailable)?
        {
            if bytes.len() + chunk.len() > LIMIT {
                return Err(BuildError::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        let data: StaticVersions =
            serde_json::from_slice(&bytes).map_err(|_| BuildError::InvalidResponse)?;
        if !valid(&data.live_version)
            || data.versions.len() > 4096
            || data.versions.iter().any(|v| !valid(v))
        {
            return Err(BuildError::InvalidResponse);
        }
        Ok(data)
    }
}
