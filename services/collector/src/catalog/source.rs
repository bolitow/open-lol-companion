//! Empreintes reproductibles des sources publiques.
use super::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[allow(clippy::too_many_arguments)]
/// Identifie une source par contenu et contexte, indépendamment de sa date de relecture.
pub fn make_source(
    provider: &str,
    key: &str,
    version: &str,
    locale: Option<&str>,
    url: &str,
    observed_at: &str,
    data: Value,
) -> CatalogSource {
    let identity = json!([provider, key, version, locale, url, data]);
    let id = format!("{:x}", Sha256::digest(identity.to_string().as_bytes()));
    CatalogSource {
        id,
        provider: provider.into(),
        key: key.into(),
        version: version.into(),
        locale: locale.map(str::to_owned),
        url: url.into(),
        observed_at: observed_at.into(),
        data,
    }
}

pub(crate) fn source_meta(source: &CatalogSource) -> SourceMeta {
    SourceMeta {
        id: source.id.clone(),
        provider: source.provider.clone(),
        key: source.key.clone(),
        version: source.version.clone(),
        locale: source.locale.clone(),
        url: source.url.clone(),
        observed_at: source.observed_at.clone(),
    }
}

/// Accepte uniquement une version numérique Data Dragon majeure.mineure.révision.
pub fn valid_version(version: &str) -> bool {
    let pieces: Vec<_> = version.split('.').collect();
    pieces.len() == 3
        && pieces
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 4 && p.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn une_correction_change_la_revision_mais_pas_une_simple_reverification() {
        let a = make_source(
            "ddragon",
            "fr_FR/item.json",
            "16.19.1",
            Some("fr_FR"),
            "https://example.test/item.json",
            "date1",
            json!({"gold":0}),
        );
        let b = make_source(
            "ddragon",
            "fr_FR/item.json",
            "16.19.1",
            Some("fr_FR"),
            "https://example.test/item.json",
            "date2",
            json!({"gold":0}),
        );
        let c = make_source(
            "ddragon",
            "fr_FR/item.json",
            "16.19.1",
            Some("fr_FR"),
            "https://example.test/item.json",
            "date2",
            json!({"gold":1}),
        );
        assert_eq!(a.id, b.id);
        assert_ne!(a.id, c.id);
        assert!(!valid_version("../16.19.1"));
    }
}
