use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task::JoinSet;

use super::model::{patch, validate_document, Kind};
use super::transport::fetch_json;
use super::{StaticError, StaticTransport};

const LOCALES: [&str; 2] = ["fr_FR", "en_US"];
const NAMESPACES: [&str; 2] = ["", "mode/classic/"];
const CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct CachedDocument {
    pub url: String,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ReleaseBundle {
    pub schema_version: u32,
    pub version: String,
    pub patch: String,
    /// Base versionnée pour les images champion/item/spell/passive/map/profileicon.
    /// Les chemins perk-images des runes restent ceux du catalogue Riot, non versionnés.
    pub asset_base: String,
    pub classic_asset_base: String,
    pub rune_asset_base: String,
    pub champion_count: usize,
    pub classic_champion_count: usize,
    pub documents: BTreeMap<String, CachedDocument>,
}

struct Request {
    key: String,
    url: String,
    kind: Kind,
}

fn data_request(version: &str, locale: &str, resource: &str, kind: Kind) -> Request {
    Request {
        key: format!("{locale}/{resource}"),
        url: format!("https://ddragon.leagueoflegends.com/cdn/{version}/data/{locale}/{resource}"),
        kind,
    }
}

fn base_requests(version: &str) -> Vec<Request> {
    let mut requests = Vec::new();
    for locale in LOCALES {
        for (resource, kind) in [
            ("champion.json", Kind::ChampionList),
            ("mode/classic/champion.json", Kind::ChampionList),
            ("item.json", Kind::Item),
            ("summoner.json", Kind::Summoner),
            ("runesReforged.json", Kind::Runes),
            ("map.json", Kind::Map),
            ("profileicon.json", Kind::ProfileIcon),
        ] {
            requests.push(data_request(version, locale, resource, kind));
        }
    }
    requests
}

fn details_requests(
    version: &str,
    documents: &BTreeMap<String, CachedDocument>,
) -> Result<(Vec<Request>, [usize; 2]), StaticError> {
    let mut requests = Vec::new();
    let mut counts = [0; 2];
    for (index, namespace) in NAMESPACES.iter().enumerate() {
        let mut previous = None;
        for locale in LOCALES {
            let roster = documents
                .get(&format!("{locale}/{namespace}champion.json"))
                .and_then(|d| d.data.get("data"))
                .and_then(Value::as_object)
                .ok_or(StaticError::InvalidDocument)?;
            let mut identities = BTreeMap::new();
            for (id, champion) in roster {
                let key = champion
                    .get("key")
                    .and_then(Value::as_str)
                    .ok_or(StaticError::InvalidDocument)?;
                identities.insert(id.clone(), key.to_owned());
                requests.push(data_request(
                    version,
                    locale,
                    &format!("{namespace}champion/{id}.json"),
                    Kind::Champion {
                        id: id.clone(),
                        key: key.into(),
                    },
                ));
            }
            if previous.as_ref().is_some_and(|p| p != &identities) {
                return Err(StaticError::InvalidDocument);
            }
            counts[index] = identities.len();
            previous = Some(identities);
        }
    }
    Ok((requests, counts))
}

async fn fetch_batch<T: StaticTransport>(
    transport: T,
    version: &str,
    requests: Vec<Request>,
) -> Result<BTreeMap<String, CachedDocument>, StaticError> {
    let mut queue = requests.into_iter();
    let mut jobs = JoinSet::new();
    let mut documents = BTreeMap::new();
    loop {
        while jobs.len() < CONCURRENCY {
            let Some(request) = queue.next() else {
                break;
            };
            let transport = transport.clone();
            let version = version.to_owned();
            jobs.spawn(async move {
                let data = fetch_json(transport, request.url.clone()).await?;
                validate_document(&request.kind, &version, &data).inspect_err(|_| {
                    // Clé de ressource publique uniquement, jamais le corps téléchargé.
                    tracing::warn!(resource = %request.key, version = %version, "document statique Riot invalide");
                })?;
                Ok::<_, StaticError>((
                    request.key,
                    CachedDocument {
                        url: request.url,
                        data,
                    },
                ))
            });
        }
        let Some(completed) = jobs.join_next().await else {
            break;
        };
        let (key, document) = completed.map_err(|_| StaticError::Network)??;
        documents.insert(key, document);
    }
    Ok(documents)
}

pub(super) async fn download_release<T: StaticTransport>(
    transport: T,
    version: &str,
) -> Result<ReleaseBundle, StaticError> {
    let patch = patch(version)?;
    let mut documents = fetch_batch(transport.clone(), version, base_requests(version)).await?;
    let (requests, counts) = details_requests(version, &documents)?;
    documents.extend(fetch_batch(transport, version, requests).await?);
    Ok(ReleaseBundle {
        schema_version: 1,
        version: version.into(),
        patch,
        asset_base: format!("https://ddragon.leagueoflegends.com/cdn/{version}/img/"),
        classic_asset_base: format!(
            "https://ddragon.leagueoflegends.com/cdn/{version}/img/mode/classic/"
        ),
        rune_asset_base: "https://ddragon.leagueoflegends.com/cdn/img/".into(),
        champion_count: counts[0],
        classic_champion_count: counts[1],
        documents,
    })
}

pub(super) fn validate_bundle(bundle: &ReleaseBundle, version: &str) -> Result<(), StaticError> {
    if bundle.schema_version != 1
        || bundle.version != version
        || bundle.patch != patch(version)?
        || bundle.asset_base != format!("https://ddragon.leagueoflegends.com/cdn/{version}/img/")
        || bundle.classic_asset_base
            != format!("https://ddragon.leagueoflegends.com/cdn/{version}/img/mode/classic/")
        || bundle.rune_asset_base != "https://ddragon.leagueoflegends.com/cdn/img/"
    {
        return Err(StaticError::InvalidDocument);
    }
    let mut requests = base_requests(version);
    let (details, counts) = details_requests(version, &bundle.documents)?;
    requests.extend(details);
    if bundle.documents.len() != requests.len()
        || bundle.champion_count != counts[0]
        || bundle.classic_champion_count != counts[1]
    {
        return Err(StaticError::InvalidDocument);
    }
    for request in requests {
        let document = bundle
            .documents
            .get(&request.key)
            .ok_or(StaticError::InvalidDocument)?;
        if document.url != request.url {
            return Err(StaticError::InvalidDocument);
        }
        validate_document(&request.kind, version, &document.data)?;
    }
    Ok(())
}

pub(super) async fn download_catalogs<T: StaticTransport>(
    transport: T,
) -> Result<BTreeMap<String, CachedDocument>, StaticError> {
    let requests = [
        ("queues", Kind::Queues),
        ("maps", Kind::Maps),
        ("gameModes", Kind::Modes),
        ("gameTypes", Kind::GameTypes),
    ]
    .into_iter()
    .map(|(name, kind)| Request {
        key: name.into(),
        url: format!("https://static.developer.riotgames.com/docs/lol/{name}.json"),
        kind,
    })
    .collect();
    fetch_batch(transport, "", requests).await
}
