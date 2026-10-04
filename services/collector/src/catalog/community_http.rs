//! Transport séparé : la limite et les hôtes Data Dragon restent inchangés.
use std::time::Duration;

use serde_json::Value;

use super::super::{make_source, valid_version, CatalogError, CatalogSource};
use crate::collector::now_ms;
use crate::riot_client::public_tls_config;
use crate::static_data::{StaticError, StaticResponse, StaticTransport};

pub(super) const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;
pub(super) const RESOURCES: [(&str, Option<&str>, &str); 8] = [
    ("content-metadata.json", None, "content-metadata.json"),
    ("items.bin", None, "game/items.cdtb.bin.json"),
    (
        "fr_FR/items.json",
        Some("fr_FR"),
        "plugins/rcp-be-lol-game-data/global/fr_fr/v1/items.json",
    ),
    (
        "en_US/items.json",
        Some("en_US"),
        "plugins/rcp-be-lol-game-data/global/default/v1/items.json",
    ),
    (
        "fr_FR/perks.json",
        Some("fr_FR"),
        "plugins/rcp-be-lol-game-data/global/fr_fr/v1/perks.json",
    ),
    (
        "fr_FR/perkstyles.json",
        Some("fr_FR"),
        "plugins/rcp-be-lol-game-data/global/fr_fr/v1/perkstyles.json",
    ),
    (
        "en_US/perks.json",
        Some("en_US"),
        "plugins/rcp-be-lol-game-data/global/default/v1/perks.json",
    ),
    (
        "en_US/perkstyles.json",
        Some("en_US"),
        "plugins/rcp-be-lol-game-data/global/default/v1/perkstyles.json",
    ),
];

/// Augments Arena et Mayhem (#118) : noms FR/EN, icône, rareté et listes par mode viennent de
/// `cherry-augments.json` (554 augments sur 16.19) ; `augment-lists.json` est identique dans toutes
/// les langues (vérifié FR/EN sur 16.19), il n'est donc téléchargé qu'une fois. Les descriptions
/// ne figurent pas dans ces fichiers : l'export généré `cdragon/arena/{fr_fr,en_us}.json` les publie
/// (champ `desc`) pour 225 augments (HTTP 200 et correspondance vérifiés le 4 octobre 2026 sur 16.19),
/// les autres, Mayhem, n'en ont pas. Cet export est produit par CommunityDragon et sort du schéma
/// `plugins/rcp-be-lol-game-data` des autres ressources ; il est épinglé au patch, au build et à
/// l'URL exacte comme elles. Ces cinq ressources restent facultatives à la relecture : une archive
/// antérieure à #118 n'en a pas, un jeu partiel est refusé.
pub(super) const AUGMENT_RESOURCES: [(&str, Option<&str>, &str); 5] = [
    (
        "fr_FR/cherry-augments.json",
        Some("fr_FR"),
        "plugins/rcp-be-lol-game-data/global/fr_fr/v1/cherry-augments.json",
    ),
    (
        "en_US/cherry-augments.json",
        Some("en_US"),
        "plugins/rcp-be-lol-game-data/global/default/v1/cherry-augments.json",
    ),
    (
        "augment-lists.json",
        None,
        "plugins/rcp-be-lol-game-data/global/default/v1/augment-lists.json",
    ),
    (
        "fr_FR/arena-augments.json",
        Some("fr_FR"),
        "cdragon/arena/fr_fr.json",
    ),
    (
        "en_US/arena-augments.json",
        Some("en_US"),
        "cdragon/arena/en_us.json",
    ),
];

pub(super) fn patch(version: &str) -> Result<&str, CatalogError> {
    if !valid_version(version) {
        return Err(CatalogError::InvalidRequest);
    }
    version
        .rsplit_once('.')
        .map(|(p, _)| p)
        .ok_or(CatalogError::InvalidRequest)
}

pub(super) fn build<'a>(patch: &str, metadata: &'a Value) -> Result<&'a str, CatalogError> {
    let version = metadata
        .get("version")
        .and_then(Value::as_str)
        .ok_or(CatalogError::InvalidSource)?;
    let number = version
        .split('+')
        .next()
        .ok_or(CatalogError::InvalidSource)?;
    let (found_patch, revision) = number.rsplit_once('.').ok_or(CatalogError::InvalidSource)?;
    if found_patch != patch
        || revision.is_empty()
        || !revision.bytes().all(|b| b.is_ascii_digit())
        || version.len() > 160
    {
        return Err(CatalogError::InvalidSource);
    }
    Ok(version)
}

pub(super) fn url(patch: &str, path: &str) -> String {
    format!("https://raw.communitydragon.org/{patch}/{path}")
}

#[derive(Clone)]
struct CommunityHttp {
    client: reqwest::Client,
}

impl CommunityHttp {
    fn new() -> Result<Self, CatalogError> {
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(public_tls_config().map_err(|_| CatalogError::Network)?)
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("OpenLoLCompanion/0.1 (public-static-catalog)")
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| CatalogError::Network)?;
        Ok(Self { client })
    }
}

impl StaticTransport for CommunityHttp {
    async fn get(&self, url: &str) -> Result<StaticResponse, StaticError> {
        let url = reqwest::Url::parse(url).map_err(|_| StaticError::Network)?;
        if url.scheme() != "https"
            || url.host_str() != Some("raw.communitydragon.org")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port_or_known_default() != Some(443)
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
        // Aucun retry sur 429 : le rafraîchissement échoue sans devancer Retry-After.
        if status != 200 {
            return Ok(StaticResponse {
                status,
                body: vec![],
            });
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_BODY_BYTES as u64)
        {
            return Err(StaticError::InvalidDocument);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| StaticError::Network)? {
            if body.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
                return Err(StaticError::InvalidDocument);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(StaticResponse { status, body })
    }
}

async fn fetch_json<T: StaticTransport>(transport: &T, url: &str) -> Result<Value, CatalogError> {
    let response = tokio::time::timeout(Duration::from_secs(35), transport.get(url))
        .await
        .map_err(|_| CatalogError::Network)?
        .map_err(|e| {
            if e == StaticError::InvalidDocument {
                CatalogError::InvalidSource
            } else {
                CatalogError::Network
            }
        })?;
    if response.status != 200 {
        return Err(CatalogError::Network);
    }
    if response.body.len() > MAX_BODY_BYTES {
        return Err(CatalogError::InvalidSource);
    }
    serde_json::from_slice(&response.body).map_err(|_| CatalogError::InvalidSource)
}

/// Télécharge le complément public du patch avec contrôle de version et de taille.
pub async fn fetch_sources(version: &str) -> Result<Vec<CatalogSource>, CatalogError> {
    patch(version)?;
    fetch_sources_with(version, CommunityHttp::new()?).await
}

/// Injection de réponses publiques pour tester panne, taille et concordance de patch.
pub async fn fetch_sources_with<T: StaticTransport>(
    version: &str,
    transport: T,
) -> Result<Vec<CatalogSource>, CatalogError> {
    let patch = patch(version)?;
    let metadata_url = url(patch, RESOURCES[0].2);
    let metadata = fetch_json(&transport, &metadata_url).await?;
    let build = build(patch, &metadata)?.to_owned();
    let observed = now_ms().to_string();
    let mut sources = vec![make_source(
        "cdragon",
        RESOURCES[0].0,
        &build,
        None,
        &metadata_url,
        &observed,
        metadata.clone(),
    )];
    // Une requête à la fois borne la mémoire du grand export BIN et la charge publique.
    for (key, locale, path) in RESOURCES.iter().skip(1).chain(AUGMENT_RESOURCES.iter()) {
        let url = url(patch, path);
        let data = fetch_json(&transport, &url).await?;
        sources.push(make_source(
            "cdragon",
            key,
            &build,
            *locale,
            &url,
            &now_ms().to_string(),
            data,
        ));
    }
    if metadata != fetch_json(&transport, &metadata_url).await? {
        return Err(CatalogError::InvalidSource);
    }
    super::validate_sources(version, &sources)?;
    Ok(sources)
}
