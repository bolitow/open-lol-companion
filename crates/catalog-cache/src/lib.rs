//! Contrat d’instantané et stockage vérifié, partagé entre distributeur et desktop.
mod catalog;
pub mod cosmetics;
mod storage;
pub use catalog::validate_catalog;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
pub use storage::{Cache, Lease};
pub const MAX_MANIFEST: usize = 2 * 1024 * 1024;
pub const MAX_JSON: u64 = 16 * 1024 * 1024;
pub const MAX_IMAGE: u64 = 8 * 1024 * 1024;
pub const MAX_TOTAL: u64 = 256 * 1024 * 1024;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("catalog_invalid")]
    Invalid,
    #[error("catalog_storage")]
    Storage(#[from] std::io::Error),
    #[error("catalog_json")]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    pub bytes: u64,
    pub sha256: String,
    pub media_type: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub version: String,
    pub normalizer_version: u32,
    pub snapshot_id: String,
    #[serde(deserialize_with = "unique_files")]
    pub files: BTreeMap<String, FileEntry>,
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn valid_id(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn version_parts(version: &str) -> Option<Vec<u32>> {
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|p| {
            p.is_empty()
                || p.len() > 4
                || !p.bytes().all(|b| b.is_ascii_digit())
                || p.len() > 1 && p.starts_with('0')
        })
    {
        return None;
    }
    parts.iter().map(|p| p.parse().ok()).collect()
}
pub fn select_release(patch: &str, versions: &[String]) -> Option<String> {
    let prefix = format!("{patch}.");
    versions
        .iter()
        .filter(|v| v.starts_with(&prefix))
        .filter_map(|v| version_parts(v).map(|n| (n, v)))
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, v)| v.clone())
}
pub fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 256
        && path.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        })
        && [".json", ".png", ".jpg"]
            .iter()
            .any(|ext| path.ends_with(ext))
}
pub fn snapshot_id(m: &Manifest) -> Result<String> {
    // BTreeMap et structure fixe : sérialisation canonique propre au schéma 1.
    Ok(digest(&serde_json::to_vec(&(
        m.schema_version,
        &m.version,
        m.normalizer_version,
        &m.files,
    ))?))
}
pub fn validate_manifest(m: &Manifest) -> Result<()> {
    if m.schema_version != 1
        || m.normalizer_version == 0
        || version_parts(&m.version).is_none()
        || !valid_id(&m.snapshot_id)
        || m.snapshot_id != snapshot_id(m)?
        || m.files.is_empty()
        || m.files.len() > 10000
        || serde_json::to_vec(m)?.len() > MAX_MANIFEST
    {
        return Err(Error::Invalid);
    }
    let mut total = 0u64;
    for (path, file) in &m.files {
        let limit = match file.media_type.as_str() {
            "application/json" if path.ends_with(".json") => MAX_JSON,
            "image/png" if path.ends_with(".png") => MAX_IMAGE,
            "image/jpeg" if path.ends_with(".jpg") => MAX_IMAGE,
            _ => return Err(Error::Invalid),
        };
        if !valid_path(path) || !valid_id(&file.sha256) || file.bytes == 0 || file.bytes > limit {
            return Err(Error::Invalid);
        }
        total = total.checked_add(file.bytes).ok_or(Error::Invalid)?;
    }
    if total > MAX_TOTAL {
        return Err(Error::Invalid);
    }
    Ok(())
}
#[cfg(test)]
mod tests;

fn unique_files<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, FileEntry>, D::Error> {
    struct Files;
    impl<'de> serde::de::Visitor<'de> for Files {
        type Value = BTreeMap<String, FileEntry>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("des chemins uniques")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut files = BTreeMap::new();
            while let Some((path, entry)) = map.next_entry::<String, FileEntry>()? {
                if files.insert(path, entry).is_some() {
                    return Err(serde::de::Error::custom("chemin dupliqué"));
                }
            }
            Ok(files)
        }
    }
    deserializer.deserialize_map(Files)
}
