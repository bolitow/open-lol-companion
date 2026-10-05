//! Transport des instantanés desktop sur l’origine configurée exclusivement.
use crate::{BuildClient, BuildError};
use olc_catalog_cache::{
    digest, valid_id, valid_path, validate_manifest, version_parts, FileEntry, Manifest,
    MAX_MANIFEST,
};
use std::time::Duration;
impl BuildClient {
    /// Reprise par fichier vérifié, au plus quatre transports simultanés ; aucune activation ici.
    pub async fn download_catalog(
        &self,
        cache: &mut olc_catalog_cache::Cache,
        m: &Manifest,
    ) -> Result<(), BuildError> {
        use futures_util::{stream, StreamExt};
        validate_manifest(m).map_err(|_| BuildError::InvalidResponse)?;
        cache.prepare(m).map_err(|_| BuildError::Unavailable)?;
        // Un nettoyage impossible ne doit pas empêcher la reprise du catalogue sain.
        let _ = cache.cleanup();
        let mut seen = std::collections::HashSet::new();
        let pending: Vec<_> = m
            .files
            .iter()
            .filter(|(_, entry)| !cache.has(entry) && seen.insert(entry.sha256.clone()))
            .collect();
        let mut downloads = stream::iter(pending.into_iter().map(|(path, entry)| async move {
            self.catalog_file(&m.snapshot_id, path, entry)
                .await
                .map(|bytes| (entry, bytes))
        }))
        .buffer_unordered(4);
        while let Some(result) = downloads.next().await {
            let (entry, bytes) = result?;
            cache
                .put(entry, &bytes)
                .map_err(|_| BuildError::Unavailable)?;
        }
        cache.save(m).map_err(|_| BuildError::InvalidResponse)
    }
    async fn catalog_bytes(
        &self,
        path: &str,
        limit: u64,
        etag: Option<&str>,
    ) -> Result<Option<Vec<u8>>, BuildError> {
        let url = self
            .base
            .join(path)
            .map_err(|_| BuildError::InvalidRequest)?;
        let mut request = self.http.get(url).timeout(Duration::from_secs(30));
        if let Some(tag) = etag {
            request = request.header(reqwest::header::IF_NONE_MATCH, tag);
        }
        let mut response = request.send().await.map_err(|_| BuildError::Unavailable)?;
        match response.status().as_u16() {
            200 => {}
            304 if etag.is_some() => return Ok(None),
            401 | 403 => return Err(BuildError::Unauthorized),
            429 => return Err(BuildError::RateLimited),
            _ => return Err(BuildError::Unavailable),
        }
        if response.content_length().is_some_and(|n| n > limit) {
            return Err(BuildError::InvalidResponse);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| BuildError::Unavailable)?
        {
            if bytes.len() as u64 + chunk.len() as u64 > limit {
                return Err(BuildError::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(Some(bytes))
    }
    /// ETag de contrat : identifiant SHA-256 de l’instantané entre guillemets.
    pub async fn catalog_manifest(
        &self,
        version: &str,
        current: Option<&str>,
    ) -> Result<Option<Manifest>, BuildError> {
        if version_parts(version).is_none() || current.is_some_and(|v| !valid_id(v)) {
            return Err(BuildError::InvalidRequest);
        }
        let tag = current.map(|v| format!("\"{v}\""));
        let Some(bytes) = self
            .catalog_bytes(
                &format!("v1/desktop-catalog/{version}/manifest"),
                MAX_MANIFEST as u64,
                tag.as_deref(),
            )
            .await?
        else {
            return Ok(None);
        };
        let manifest: Manifest =
            serde_json::from_slice(&bytes).map_err(|_| BuildError::InvalidResponse)?;
        validate_manifest(&manifest).map_err(|_| BuildError::InvalidResponse)?;
        if manifest.version != version {
            return Err(BuildError::InvalidResponse);
        }
        Ok(Some(manifest))
    }
    pub async fn catalog_file(
        &self,
        id: &str,
        path: &str,
        entry: &FileEntry,
    ) -> Result<Vec<u8>, BuildError> {
        if !valid_id(id) || !valid_path(path) || entry.bytes > olc_catalog_cache::MAX_JSON {
            return Err(BuildError::InvalidRequest);
        }
        let bytes = self
            .catalog_bytes(
                &format!("v1/desktop-catalog/snapshots/{id}/{path}"),
                entry.bytes,
                None,
            )
            .await?
            .ok_or(BuildError::InvalidResponse)?;
        if bytes.len() as u64 != entry.bytes || digest(&bytes) != entry.sha256 {
            return Err(BuildError::InvalidResponse);
        }
        Ok(bytes)
    }
}
