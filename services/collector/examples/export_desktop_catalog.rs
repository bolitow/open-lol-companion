//! Export développeur hors ligne pour le desktop (#13), sans base ni clé Riot.
//!
//! `cargo run -p olc-collector --example export_desktop_catalog --release -- \
//! --version 16.19.1 --output apps/desktop/public/game-data/catalog`
//! La sortie doit être absente ou vide : aucune publication locale n'est écrasée.

use clap::Parser;
use olc_collector::catalog::{
    community, make_source, project_sources, valid_version, CatalogRecord, CatalogSource,
    SourceMeta, ValueStatus, NORMALIZER_VERSION,
};
use reqwest::{Client, Url};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::task::JoinSet;

type ExportError = Box<dyn std::error::Error + Send + Sync>;
const PUBLIC_PREFIX: &str = "/game-data/catalog/";
const LOCALES: [&str; 2] = ["fr_FR", "en_US"];
const ICON_CONCURRENCY: usize = 4;
const DOCUMENT_CONCURRENCY: usize = 4;
const MAX_ICON_BYTES: usize = 2 * 1024 * 1024;
const MAX_JSON_BYTES: usize = 16 * 1024 * 1024;

/// Cartes dont les objets sont exportés (#116) : Faille (11), ARAM (12) et Arena (30).
/// La disponibilité par carte reste dans `fields.maps` de chaque objet ; elle ne dit pas
/// qu'un objet est achetable dans chaque file, ce que le front vérifie avec `in_store`.
const ITEM_MAPS: [&str; 3] = ["11", "12", "30"];

#[derive(Debug, Serialize)]
struct ItemSelection {
    #[serde(skip)]
    ids: BTreeSet<String>,
    mode: &'static str,
    reason: Option<&'static str>,
    maps: [&'static str; 3],
    /// Objets conservés disponibles sur chaque carte ; un objet commun à deux cartes compte deux fois.
    by_map: BTreeMap<String, usize>,
    /// Repli `all_items` seulement : objets dont la disponibilité sur la carte n'a pas pu être
    /// lue (clé absente, valeur non booléenne ou champ `maps` incertain), par carte. Vide quand
    /// le filtre par carte a pu s'appliquer.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    unreadable_by_map: BTreeMap<String, usize>,
}

fn on_map(record: &CatalogRecord, map: &str) -> bool {
    record
        .fields
        .get("maps")
        .and_then(|maps| maps.value.get(map))
        .and_then(Value::as_bool)
        == Some(true)
}

fn public_url(url: &str) -> Result<Url, ExportError> {
    let parsed = Url::parse(url)?;
    if parsed.scheme() != "https"
        || !matches!(
            parsed.host_str(),
            Some("ddragon.leagueoflegends.com" | "raw.communitydragon.org")
        )
        || parsed.port_or_known_default() != Some(443)
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err("URL de ressource publique non autorisée".into());
    }
    Ok(parsed)
}

fn icon_relative_path(url: &str) -> Result<String, ExportError> {
    let parsed = public_url(url)?;
    if !parsed.path().to_ascii_lowercase().ends_with(".png") {
        return Err("l'icône du catalogue doit être un PNG".into());
    }
    Ok(format!("icons/{:x}.png", Sha256::digest(url.as_bytes())))
}

fn select_items(records: &[CatalogRecord]) -> ItemSelection {
    let items: BTreeMap<_, _> = records
        .iter()
        .filter(|record| record.kind == "item")
        .map(|record| (record.id.clone(), record))
        .collect();
    let selection = |ids: BTreeSet<String>, mode, reason| ItemSelection {
        by_map: ITEM_MAPS
            .iter()
            .map(|map| {
                let count = ids
                    .iter()
                    .filter(|id| items.get(*id).is_some_and(|item| on_map(item, map)))
                    .count();
                (map.to_string(), count)
            })
            .collect(),
        ids,
        mode,
        reason,
        maps: ITEM_MAPS,
        unreadable_by_map: BTreeMap::new(),
    };
    let all = |reason| selection(items.keys().cloned().collect(), "all_items", Some(reason));
    let mut selected = BTreeSet::new();
    // Premier objet illisible : sa raison est conservée. Tous les objets sont parcourus pour
    // dénombrer, par carte, ce qui a déclenché le repli (un seul objet suffit à le déclencher).
    let mut fallback: Option<&'static str> = None;
    let mut unreadable_by_map: BTreeMap<String, usize> = BTreeMap::new();
    for (id, record) in &items {
        let readable = record
            .fields
            .get("maps")
            .filter(|maps| matches!(maps.status, ValueStatus::Verified | ValueStatus::Derived));
        let Some(maps) = readable else {
            fallback.get_or_insert(if record.fields.contains_key("maps") {
                "uncertain_map"
            } else {
                "unknown_map"
            });
            for map in ITEM_MAPS {
                *unreadable_by_map.entry(map.to_string()).or_default() += 1;
            }
            continue;
        };
        // Une carte exportée absente ou non booléenne rend la disponibilité incertaine :
        // mieux vaut tout garder que perdre un objet propre à l'ARAM ou à l'Arena.
        let mut available = false;
        for map in ITEM_MAPS {
            match maps.value.get(map).and_then(Value::as_bool) {
                Some(on_map) => available |= on_map,
                None => {
                    fallback.get_or_insert("unknown_map");
                    *unreadable_by_map.entry(map.to_string()).or_default() += 1;
                }
            }
        }
        if available {
            selected.insert(id.clone());
        }
    }
    if let Some(reason) = fallback {
        let mut result = all(reason);
        result.unreadable_by_map = unreadable_by_map;
        return result;
    }
    // Une recette peut inclure un composant réservé : garder sa fiche même si sa
    // carte est différente. Le parcours par ensemble termine aussi sur un cycle.
    let mut pending: Vec<_> = selected.iter().cloned().collect();
    while let Some(id) = pending.pop() {
        let Some(record) = items.get(&id) else {
            return all("unknown_component");
        };
        let Some(recipe) = record.fields.get("builds_from") else {
            continue;
        };
        if recipe.status == ValueStatus::Missing && recipe.value.is_null() {
            continue;
        }
        if !matches!(recipe.status, ValueStatus::Verified | ValueStatus::Derived) {
            return all("uncertain_recipe");
        }
        let Some(components) = recipe.value.as_array() else {
            return all("uncertain_recipe");
        };
        for component in components {
            let Some(component) = component.as_str() else {
                return all("uncertain_recipe");
            };
            if selected.insert(component.into()) {
                pending.push(component.into());
            }
        }
    }
    selection(selected, "maps_11_12_30_with_components", None)
}

fn champion_resources(index: &Value) -> Result<Vec<String>, ExportError> {
    let champions = index["data"]
        .as_object()
        .filter(|data| !data.is_empty())
        .ok_or("index des champions absent ou vide")?;
    let mut ids = BTreeSet::new();
    let mut resources = Vec::new();
    for (key, champion) in champions {
        let id = champion["key"]
            .as_str()
            .and_then(|id| id.parse::<u32>().ok())
            .filter(|id| *id > 0)
            .ok_or("identifiant numérique de champion invalide")?;
        if key.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || champion["id"].as_str() != Some(key)
            || !ids.insert(id)
        {
            return Err("clé ou identité de champion incohérente".into());
        }
        resources.push(format!("champion/{key}.json"));
    }
    resources.sort();
    Ok(resources)
}

fn select_desktop_records(
    records: &[CatalogRecord],
    locale: &str,
) -> (Vec<CatalogRecord>, ItemSelection) {
    let mut records: Vec<_> = records
        .iter()
        .filter(|record| {
            record.locale == locale
                && record.namespace == "standard"
                && matches!(
                    record.kind.as_str(),
                    "item"
                        | "rune"
                        | "rune_shard"
                        | "champion"
                        | "ability"
                        | "summoner_spell"
                        | "augment"
                )
        })
        .cloned()
        .collect();
    let item_filter = select_items(&records);
    records.retain(|record| record.kind != "item" || item_filter.ids.contains(&record.id));
    records.sort_by(|a, b| (&a.kind, &a.id).cmp(&(&b.kind, &b.id)));
    (records, item_filter)
}

struct CatalogPartition {
    root: Vec<CatalogRecord>,
    champions: BTreeMap<String, Vec<CatalogRecord>>,
}

fn partition_champions(records: Vec<CatalogRecord>) -> Result<CatalogPartition, ExportError> {
    let mut result = CatalogPartition {
        root: Vec::new(),
        champions: BTreeMap::new(),
    };
    for record in records {
        let champion_id = match record.kind.as_str() {
            "champion" => record.id.as_str(),
            "ability" => {
                let champion_id = record
                    .fields
                    .get("champion_id")
                    .and_then(|field| field.value.as_str())
                    .ok_or("champion de compétence absent")?;
                let slot = record
                    .fields
                    .get("slot")
                    .and_then(|field| field.value.as_str())
                    .filter(|slot| ["passive", "Q", "W", "E", "R"].contains(slot))
                    .ok_or("emplacement de compétence invalide")?;
                if record.id != format!("{champion_id}:{slot}") {
                    return Err("identité de compétence incohérente".into());
                }
                champion_id
            }
            _ => {
                result.root.push(record);
                continue;
            }
        };
        let id = champion_id
            .parse::<u32>()
            .ok()
            .filter(|id| *id > 0)
            .ok_or("identifiant numérique de champion invalide")?;
        if id.to_string() != champion_id {
            return Err("identifiant numérique de champion non canonique".into());
        }
        result
            .champions
            .entry(champion_id.into())
            .or_default()
            .push(record);
    }
    for (champion_id, records) in &result.champions {
        let identities: BTreeSet<_> = records
            .iter()
            .map(|record| (record.kind.as_str(), record.id.clone()))
            .collect();
        if records.len() != 6
            || !identities.contains(&("champion", champion_id.clone()))
            || ["passive", "Q", "W", "E", "R"]
                .iter()
                .any(|slot| !identities.contains(&("ability", format!("{champion_id}:{slot}"))))
        {
            return Err("fiche champion ou compétences incomplètes".into());
        }
    }
    Ok(result)
}

#[derive(Parser)]
#[command(about = "Exporte le catalogue desktop FR/EN sans PostgreSQL ni clé Riot")]
struct Args {
    #[arg(long)]
    version: String,
    /// Répertoire absent ou vide, afin de préserver les publications existantes.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Serialize)]
struct LocaleCatalog {
    version: String,
    records: Vec<CatalogRecord>,
}

#[derive(Serialize)]
struct IconMeta {
    local_path: String,
    source_url: String,
    sha256: String,
    bytes: usize,
}

#[derive(Serialize)]
struct ExportSources {
    version: String,
    normalizer_version: u32,
    sources: Vec<SourceMeta>,
    icons: Vec<IconMeta>,
    riot_notice: &'static str,
}

#[derive(Serialize)]
struct FileMeta {
    path: String,
    sha256: String,
    bytes: usize,
}

#[derive(Serialize)]
struct LocaleMeta {
    #[serde(flatten)]
    file: FileMeta,
    records: usize,
    total_records: usize,
    by_kind: BTreeMap<String, usize>,
    item_filter: ItemSelection,
}

#[derive(Serialize)]
struct ChampionMeta {
    #[serde(flatten)]
    file: FileMeta,
    records: usize,
}

#[derive(Serialize)]
struct Manifest {
    schema_version: u32,
    version: String,
    normalizer_version: u32,
    locales: BTreeMap<String, LocaleMeta>,
    champions: BTreeMap<String, BTreeMap<String, ChampionMeta>>,
    sources: FileMeta,
    icons: usize,
}

fn assert_empty_output(output: &Path) -> Result<(), ExportError> {
    match fs::symlink_metadata(output) {
        Ok(meta) => {
            if !meta.is_dir() || fs::read_dir(output)?.next().transpose()?.is_some() {
                return Err("la sortie doit être un répertoire absent ou vide".into());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

async fn download(client: &Client, url: &str, limit: usize) -> Result<Vec<u8>, ExportError> {
    let mut response = client
        .get(public_url(url)?)
        .send()
        .await?
        .error_for_status()?;
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err(format!("ressource trop volumineuse : {url}").into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(format!("ressource trop volumineuse : {url}").into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn download_source(
    client: &Client,
    version: &str,
    locale: &str,
    resource: &str,
) -> Result<CatalogSource, ExportError> {
    let url = format!("https://ddragon.leagueoflegends.com/cdn/{version}/data/{locale}/{resource}");
    let bytes = download(client, &url, MAX_JSON_BYTES).await?;
    Ok(make_source(
        "ddragon",
        &format!("{locale}/{resource}"),
        version,
        Some(locale),
        &url,
        &SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis()
            .to_string(),
        serde_json::from_slice(&bytes)?,
    ))
}

async fn icons(
    client: &Client,
    urls: BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<u8>>, ExportError> {
    let urls: Vec<_> = urls.into_iter().collect();
    let mut result = BTreeMap::new();
    for batch in urls.chunks(ICON_CONCURRENCY) {
        let mut tasks = JoinSet::new();
        for url in batch {
            let client = client.clone();
            let url = url.clone();
            tasks.spawn(async move {
                let bytes = download(&client, &url, MAX_ICON_BYTES).await?;
                if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                    return Err::<_, ExportError>(format!("icône PNG invalide : {url}").into());
                }
                Ok((url, bytes))
            });
        }
        while let Some(task) = tasks.join_next().await {
            let (url, bytes) = task??;
            result.insert(url, bytes);
        }
        if result.len() % 100 < ICON_CONCURRENCY || result.len() == urls.len() {
            eprintln!("Icônes vérifiées : {}/{}", result.len(), urls.len());
        }
    }
    Ok(result)
}

fn file_meta(name: &str, bytes: &[u8]) -> FileMeta {
    FileMeta {
        path: format!("{PUBLIC_PREFIX}{name}"),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        bytes: bytes.len(),
    }
}

fn write_new(output: &Path, name: &str, bytes: &[u8]) -> Result<(), ExportError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join(name))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ExportError> {
    let args = Args::parse();
    if !valid_version(&args.version) {
        return Err("version attendue : majeure.mineure.révision".into());
    }
    assert_empty_output(&args.output)?;
    let client = Client::builder()
        .use_preconfigured_tls(olc_collector::riot_client::public_tls_config()?)
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("OpenLoLCompanion/0.1 (desktop-catalog-export)")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()?;
    let mut sources = Vec::new();
    for locale in LOCALES {
        for resource in ["item.json", "runesReforged.json", "summoner.json"] {
            sources.push(download_source(&client, &args.version, locale, resource).await?);
        }
        // L'index officiel donne les clés techniques ; les fiches détaillées
        // apportent le passif et les quatre compétences dans l'ordre Q/W/E/R.
        // Schéma : https://developer.riotgames.com/docs/lol#data-dragon_champions
        let index = download_source(&client, &args.version, locale, "champion.json").await?;
        let resources = champion_resources(&index.data)?;
        sources.push(index);
        for batch in resources.chunks(DOCUMENT_CONCURRENCY) {
            let mut tasks = JoinSet::new();
            for resource in batch {
                let client = client.clone();
                let version = args.version.clone();
                let resource = resource.clone();
                tasks.spawn(
                    async move { download_source(&client, &version, locale, &resource).await },
                );
            }
            while let Some(task) = tasks.join_next().await {
                sources.push(task??);
            }
        }
        eprintln!(
            "{locale} : {} fiches champion téléchargées",
            resources.len()
        );
    }
    eprintln!(
        "{} sources Data Dragon téléchargées ; complément CommunityDragon…",
        sources.len()
    );
    sources.extend(community::fetch_sources(&args.version).await?);
    let projection = project_sources(&args.version, sources, false, vec![])?;
    let mut catalog_files = BTreeMap::new();
    let mut locale_meta = BTreeMap::new();
    let mut champion_meta = BTreeMap::<String, BTreeMap<String, ChampionMeta>>::new();
    let mut icon_urls = BTreeSet::new();
    for locale in LOCALES {
        let (mut records, item_filter) = select_desktop_records(&projection.records, locale);
        for record in &mut records {
            if let Some(url) = &record.icon {
                let local_path = format!("{PUBLIC_PREFIX}{}", icon_relative_path(url)?);
                icon_urls.insert(url.clone());
                record.icon = Some(local_path);
            }
        }
        let total_records = records.len();
        let partition = partition_champions(records)?;
        for (champion_id, records) in partition.champions {
            let count = records.len();
            let bytes = serde_json::to_vec(&LocaleCatalog {
                version: args.version.clone(),
                records,
            })?;
            let name = format!("champions/{champion_id}/{locale}.json");
            champion_meta.entry(champion_id).or_default().insert(
                locale.into(),
                ChampionMeta {
                    file: file_meta(&name, &bytes),
                    records: count,
                },
            );
            catalog_files.insert(name, bytes);
        }
        let mut by_kind = BTreeMap::new();
        for record in &partition.root {
            *by_kind.entry(record.kind.clone()).or_insert(0) += 1;
        }
        let count = partition.root.len();
        let catalog = LocaleCatalog {
            version: args.version.clone(),
            records: partition.root,
        };
        let bytes = serde_json::to_vec(&catalog)?;
        let name = format!("{locale}.json");
        eprintln!(
            "{locale} : {count} fiches racine, {total_records} au total, filtre {}",
            item_filter.mode
        );
        if let Some(reason) = item_filter.reason {
            eprintln!("{locale} : repli sur tous les objets ({reason})");
            for (map, count) in &item_filter.unreadable_by_map {
                eprintln!("{locale} : carte {map}, {count} objet(s) à disponibilité illisible");
            }
        }
        locale_meta.insert(
            locale.into(),
            LocaleMeta {
                file: file_meta(&name, &bytes),
                records: count,
                total_records,
                by_kind,
                item_filter,
            },
        );
        catalog_files.insert(name, bytes);
    }
    let icon_files = icons(&client, icon_urls).await?;
    let mut icon_meta = Vec::new();
    for (url, bytes) in &icon_files {
        icon_meta.push(IconMeta {
            local_path: format!("{PUBLIC_PREFIX}{}", icon_relative_path(url)?),
            source_url: url.clone(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
            bytes: bytes.len(),
        });
    }
    let mut sources: Vec<_> = projection
        .sources
        .iter()
        .map(|source| SourceMeta {
            id: source.id.clone(),
            provider: source.provider.clone(),
            key: source.key.clone(),
            version: source.version.clone(),
            locale: source.locale.clone(),
            url: source.url.clone(),
            observed_at: source.observed_at.clone(),
        })
        .collect();
    sources.sort_by(|a, b| a.id.cmp(&b.id));
    let source_bytes = serde_json::to_vec(&ExportSources {
        version: args.version.clone(),
        normalizer_version: NORMALIZER_VERSION,
        sources,
        icons: icon_meta,
        riot_notice: "League of Legends et les ressources associées appartiennent à Riot Games. Open LoL Companion n'est ni approuvé ni sponsorisé par Riot Games.",
    })?;
    let manifest = Manifest {
        schema_version: 2,
        version: args.version,
        normalizer_version: NORMALIZER_VERSION,
        locales: locale_meta,
        champions: champion_meta,
        sources: file_meta("sources.json", &source_bytes),
        icons: icon_files.len(),
    };
    // Toute acquisition et sérialisation réussit avant de toucher la sortie.
    // create_new interdit aussi un écrasement concurrent entre ces deux contrôles.
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    assert_empty_output(&args.output)?;
    fs::create_dir_all(args.output.join("icons"))?;
    for (url, bytes) in &icon_files {
        write_new(&args.output, &icon_relative_path(url)?, bytes)?;
    }
    for (name, bytes) in catalog_files {
        if let Some(parent) = Path::new(&name).parent() {
            fs::create_dir_all(args.output.join(parent))?;
        }
        write_new(&args.output, &name, &bytes)?;
    }
    write_new(&args.output, "sources.json", &source_bytes)?;
    write_new(&args.output, "manifest.json", &manifest_bytes)?;
    println!("{}", serde_json::to_string(&manifest)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use olc_collector::catalog::{CatalogValue, RecordCoverage, ValueSource};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn item(id: &str, map: Option<bool>, components: &[&str]) -> CatalogRecord {
        // Les trois cartes exportées sont toujours présentes dans Data Dragon ; un test
        // qui ne s'intéresse qu'à la Faille laisse donc l'ARAM et l'Arena à faux.
        item_on(
            id,
            map.map(|map| json!({"11": map, "12": false, "30": false})),
            components,
        )
    }

    fn item_on(id: &str, maps: Option<Value>, components: &[&str]) -> CatalogRecord {
        let mut fields = BTreeMap::new();
        if let Some(maps) = maps {
            fields.insert(
                "maps".into(),
                CatalogValue {
                    value: maps,
                    unit: None,
                    status: ValueStatus::Verified,
                    sources: vec![],
                },
            );
        }
        fields.insert(
            "builds_from".into(),
            CatalogValue {
                value: json!(components),
                unit: None,
                status: ValueStatus::Verified,
                sources: vec![],
            },
        );
        CatalogRecord {
            kind: "item".into(),
            id: id.into(),
            namespace: "standard".into(),
            locale: "fr_FR".into(),
            name: id.into(),
            description: None,
            icon: None,
            fields,
            stats: BTreeMap::new(),
            effects: vec![],
            coverage: RecordCoverage::default(),
        }
    }

    #[test]
    fn les_icones_de_meme_nom_ne_secrasent_pas_et_leur_chemin_est_stable() {
        let first = "https://ddragon.leagueoflegends.com/cdn/16.19.1/img/item/1001.png";
        let other = "https://ddragon.leagueoflegends.com/cdn/16.18.1/img/item/1001.png";
        let path = icon_relative_path(first).unwrap();
        assert!(path.starts_with("icons/"));
        assert!(path.ends_with(".png"));
        assert_eq!(path, icon_relative_path(first).unwrap());
        assert_ne!(path, icon_relative_path(other).unwrap());
    }

    #[test]
    fn refuse_une_icone_sur_un_hote_non_autorise_ou_non_png() {
        for url in [
            "http://ddragon.leagueoflegends.com/icon.png",
            "https://example.com/icon.png",
            "https://ddragon.leagueoflegends.com/icon.svg",
            "https://user:password@ddragon.leagueoflegends.com/icon.png",
            "https://ddragon.leagueoflegends.com/icon.png?token=synthetic",
        ] {
            assert!(icon_relative_path(url).is_err());
        }
    }

    #[test]
    fn garde_les_composants_transitifs_meme_absents_de_la_carte() {
        let records = vec![
            item("100", Some(true), &["200"]),
            item("200", Some(false), &["300"]),
            item("300", Some(false), &[]),
            item("400", Some(false), &[]),
        ];
        let result = select_items(&records);
        assert_eq!(result.mode, "maps_11_12_30_with_components");
        assert_eq!(result.ids, ["100", "200", "300"].map(String::from).into());
    }

    #[test]
    fn garde_les_objets_propres_a_laram_et_a_larena_sans_les_autres_cartes() {
        let maps = |rift, aram, arena, other| {
            Some(json!({"11": rift, "12": aram, "30": arena, "453": other}))
        };
        let records = vec![
            item_on("100", maps(true, false, false, false), &[]),
            item_on("200", maps(false, true, false, false), &[]),
            item_on("300", maps(false, false, true, false), &[]),
            item_on("400", maps(false, false, false, true), &[]),
            item_on("500", maps(false, false, false, false), &[]),
        ];
        let result = select_items(&records);
        assert_eq!(result.mode, "maps_11_12_30_with_components");
        assert_eq!(result.ids, ["100", "200", "300"].map(String::from).into());
        assert_eq!(result.maps, ["11", "12", "30"]);
        assert_eq!(
            result.by_map,
            [("11", 1), ("12", 1), ("30", 1)]
                .map(|(map, n)| (map.to_string(), n))
                .into()
        );
    }

    #[test]
    fn le_decompte_par_carte_inclut_les_objets_presents_sur_plusieurs_cartes() {
        let records = vec![
            item_on(
                "100",
                Some(json!({"11": true, "12": true, "30": false})),
                &["200"],
            ),
            item_on(
                "200",
                Some(json!({"11": false, "12": false, "30": false})),
                &[],
            ),
            item_on(
                "300",
                Some(json!({"11": false, "12": true, "30": true})),
                &[],
            ),
        ];
        let result = select_items(&records);
        assert_eq!(result.ids, ["100", "200", "300"].map(String::from).into());
        // Le composant 200 est conservé pour la recette mais n'est disponible sur aucune carte.
        assert_eq!(
            result.by_map,
            [("11", 1), ("12", 2), ("30", 1)]
                .map(|(map, n)| (map.to_string(), n))
                .into()
        );
    }

    #[test]
    fn une_carte_exportee_absente_ou_non_booleenne_conserve_tous_les_objets() {
        for maps in [
            json!({"11": true, "12": false}),
            json!({"11": false, "30": true}),
            json!({"12": true, "30": false}),
            json!({"11": true, "12": "true", "30": false}),
        ] {
            let records = vec![
                item_on(
                    "100",
                    Some(json!({"11": true, "12": false, "30": false})),
                    &[],
                ),
                item_on("200", Some(maps), &[]),
            ];
            let result = select_items(&records);
            assert_eq!(result.mode, "all_items");
            assert_eq!(result.reason, Some("unknown_map"));
            assert_eq!(result.ids, ["100", "200"].map(String::from).into());
        }
    }

    #[test]
    fn le_repli_all_items_denombre_par_carte_les_objets_a_disponibilite_illisible() {
        let records = vec![
            item_on(
                "100",
                Some(json!({"11": true, "12": false, "30": false})),
                &[],
            ),
            item_on("200", Some(json!({"11": true, "30": false})), &[]),
            item_on(
                "300",
                Some(json!({"11": true, "12": "oui", "30": null})),
                &[],
            ),
            item_on("400", None, &[]),
        ];
        let result = select_items(&records);
        assert_eq!(result.mode, "all_items");
        // 200 ne dit rien de la carte 12, 300 est illisible sur 12 et 30, 400 sur les trois.
        assert_eq!(
            result.unreadable_by_map,
            [("11", 1), ("12", 3), ("30", 2)]
                .map(|(map, n)| (map.to_string(), n))
                .into()
        );
        assert_eq!(result.ids.len(), 4);
    }

    #[test]
    fn le_filtre_normal_ne_signale_aucune_disponibilite_illisible() {
        let records = vec![item("100", Some(true), &[]), item("200", Some(false), &[])];
        let result = select_items(&records);
        assert_eq!(result.mode, "maps_11_12_30_with_components");
        assert!(result.unreadable_by_map.is_empty());
    }

    #[test]
    fn une_carte_ou_recette_ambigue_conserve_tous_les_objets() {
        let unknown_map = vec![item("100", Some(true), &[]), item("200", None, &[])];
        let mut conflict = item("100", Some(true), &["200"]);
        conflict.fields.get_mut("builds_from").unwrap().status = ValueStatus::Conflict;
        let unknown_recipe = vec![conflict, item("200", Some(false), &[])];
        for records in [unknown_map, unknown_recipe] {
            let result = select_items(&records);
            assert_eq!(result.mode, "all_items");
            assert_eq!(result.ids, ["100", "200"].map(String::from).into());
        }
    }

    #[test]
    fn les_fiches_champions_sont_adressees_par_cle_technique_sans_doublon() {
        let index = json!({"data": {
            "Ahri": {"id": "Ahri", "key": "103"},
            "Aatrox": {"id": "Aatrox", "key": "266"}
        }});
        assert_eq!(
            champion_resources(&index).unwrap(),
            ["champion/Aatrox.json", "champion/Ahri.json"]
        );
    }

    #[test]
    fn un_index_champion_incomplet_ou_un_chemin_ambigu_est_refuse() {
        for index in [
            json!({}),
            json!({"data": {}}),
            json!({"data": {"Ahri": {"id": "Garen", "key": "103"}}}),
            json!({"data": {"../Ahri": {"id": "../Ahri", "key": "103"}}}),
            json!({"data": {"Ahri": {"id": "Ahri", "key": "0"}}}),
            json!({"data": {"Ahri": {"id": "Ahri", "key": "103"}, "Garen": {"id": "Garen", "key": "103"}}}),
        ] {
            assert!(champion_resources(&index).is_err());
        }
    }

    #[test]
    fn le_catalogue_desktop_conserve_les_competences_sorts_et_leur_provenance() {
        let mut records = Vec::new();
        for (kind, id) in [
            ("champion", "103"),
            ("ability", "103:Q"),
            ("ability", "103:passive"),
            ("summoner_spell", "4"),
            ("rune", "8000"),
            ("rune_shard", "5008"),
            ("map", "11"),
        ] {
            let mut record = item(id, Some(true), &[]);
            record.kind = kind.into();
            records.push(record);
        }
        records[1].fields.insert(
            "slot".into(),
            CatalogValue {
                value: json!("Q"),
                unit: None,
                status: ValueStatus::Derived,
                sources: vec![ValueSource {
                    source_id: "synthetic-champion-source".into(),
                    pointer: "/data/Ahri/spells/0".into(),
                }],
            },
        );
        let mut classic = records[0].clone();
        classic.namespace = "classic".into();
        records.push(classic);
        let mut english = records[0].clone();
        english.locale = "en_US".into();
        records.push(english);
        records.push(item("1001", Some(true), &["1002"]));
        records.push(item("1002", Some(false), &[]));
        records.push(item("1003", Some(false), &[]));
        let (selected, filter) = select_desktop_records(&records, "fr_FR");
        assert_eq!(filter.mode, "maps_11_12_30_with_components");
        assert_eq!(
            selected
                .iter()
                .map(|r| (r.kind.as_str(), r.id.as_str()))
                .collect::<Vec<_>>(),
            [
                ("ability", "103:Q"),
                ("ability", "103:passive"),
                ("champion", "103"),
                ("item", "1001"),
                ("item", "1002"),
                ("rune", "8000"),
                ("rune_shard", "5008"),
                ("summoner_spell", "4")
            ]
        );
        assert_eq!(selected[0], records[1]);
        assert_eq!(selected[2], records[0]);
        assert_eq!(selected[7], records[3]);
    }

    #[test]
    fn le_catalogue_desktop_garde_les_augments_a_la_racine_sans_les_autres_familles() {
        // #118 : augments Arena et Mayhem (données statiques), exportés avec leur langue.
        let mut records = Vec::new();
        for (kind, id, locale) in [
            ("augment", "1205", "fr_FR"),
            ("augment", "1205", "en_US"),
            ("augment", "341", "fr_FR"),
            ("queue", "1700", "fr_FR"),
        ] {
            let mut record = item(id, Some(true), &[]);
            record.kind = kind.into();
            record.locale = locale.into();
            records.push(record);
        }
        let (selected, _) = select_desktop_records(&records, "fr_FR");
        assert_eq!(
            selected
                .iter()
                .map(|r| (r.kind.as_str(), r.id.as_str()))
                .collect::<Vec<_>>(),
            [("augment", "1205"), ("augment", "341")]
        );
        let partition = partition_champions(selected).unwrap();
        assert_eq!(partition.root.len(), 2);
        assert!(partition.champions.is_empty());
    }

    fn ability(champion_id: &str, slot: &str) -> CatalogRecord {
        let mut record = item(&format!("{champion_id}:{slot}"), Some(true), &[]);
        record.kind = "ability".into();
        for (key, value) in [("champion_id", champion_id), ("slot", slot)] {
            record.fields.insert(
                key.into(),
                CatalogValue {
                    value: json!(value),
                    unit: None,
                    status: ValueStatus::Derived,
                    sources: vec![ValueSource {
                        source_id: "synthetic-champion-source".into(),
                        pointer: "/data/Ahri/spells/0".into(),
                    }],
                },
            );
        }
        record
    }

    fn champion_records(champion_id: &str) -> Vec<CatalogRecord> {
        let mut champion = item(champion_id, Some(true), &[]);
        champion.kind = "champion".into();
        let mut records = vec![champion];
        records.extend(
            ["Q", "W", "E", "R", "passive"]
                .iter()
                .map(|slot| ability(champion_id, slot)),
        );
        records
    }

    #[test]
    fn les_fiches_sont_separees_par_champion_sans_perdre_la_provenance() {
        let object = item("1001", Some(true), &[]);
        let ahri = champion_records("103");
        let garen = champion_records("86");
        let records = [vec![object.clone()], ahri.clone(), garen.clone()].concat();
        let partition = partition_champions(records).unwrap();
        assert_eq!(partition.root, vec![object]);
        assert_eq!(partition.champions.len(), 2);
        assert_eq!(partition.champions["103"], ahri);
        assert_eq!(partition.champions["86"], garen);
    }

    #[test]
    fn une_competence_sans_identite_coherente_ne_cree_pas_de_chemin_de_sortie() {
        let mut mismatched = ability("103", "Q");
        mismatched.id = "86:Q".into();
        for record in [
            ability("../103", "Q"),
            ability("0", "Q"),
            ability("103", "S"),
            mismatched,
        ] {
            let mut records = champion_records("103");
            records[1] = record;
            assert!(partition_champions(records).is_err());
        }
    }

    #[test]
    fn une_fiche_champion_incomplete_ou_dupliquee_est_refusee() {
        let mut missing = champion_records("103");
        missing.pop();
        let mut duplicate = champion_records("103");
        duplicate[1] = duplicate[2].clone();
        for records in [missing, duplicate] {
            assert!(partition_champions(records).is_err());
        }
    }
}
