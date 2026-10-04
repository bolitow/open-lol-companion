//! Publie un export desktop vérifié dans le stockage immuable distribué par l’API.
//! cargo run -p olc-collector --example publish_desktop_catalog -- --input apps/desktop/public/game-data/catalog --output /chemin/artefacts
//! Configurer OLC_DESKTOP_CATALOG_DIR sur ce même répertoire dans l’API.
//! Les portraits JPEG proviennent des skins de base CommunityDragon du même patch.
//! Un artwork manquant interrompt la publication, sans remplacement par une icône.
#[path = "publish_desktop_catalog/artworks.rs"]
mod artworks;
use clap::Parser;
use olc_catalog_cache::{digest, snapshot_id, Cache, FileEntry, Manifest};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
}
fn read_file(root: &Path, relative: &str, limit: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    if !olc_catalog_cache::valid_path(relative) {
        return Err("chemin invalide".into());
    }
    let mut path = root.to_path_buf();
    if std::fs::symlink_metadata(&path)?.file_type().is_symlink() {
        return Err("lien symbolique interdit".into());
    }
    for part in relative.split('/') {
        path.push(part);
        if std::fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err("lien symbolique interdit".into());
        }
    }
    if !std::fs::symlink_metadata(&path)?.is_file() {
        return Err("fichier régulier requis".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("fichier trop grand".into());
    }
    Ok(bytes)
}
fn insert_metadata(
    input: &Path,
    metadata: &Value,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    let path = metadata["path"]
        .as_str()
        .or_else(|| metadata["local_path"].as_str())
        .ok_or("chemin absent")?;
    let relative = path
        .strip_prefix("/game-data/catalog/")
        .ok_or("préfixe invalide")?;
    let announced = metadata["bytes"].as_u64().ok_or("taille absente")?;
    let used: u64 = files.values().map(|b| b.len() as u64).sum();
    if files.len() >= 10000 || announced > olc_catalog_cache::MAX_TOTAL.saturating_sub(used) {
        return Err("export trop grand".into());
    }
    let bytes = read_file(
        input,
        relative,
        if relative.ends_with(".json") {
            olc_catalog_cache::MAX_JSON
        } else {
            olc_catalog_cache::MAX_IMAGE
        },
    )?;
    if metadata["bytes"].as_u64() != Some(bytes.len() as u64)
        || metadata["sha256"].as_str() != Some(digest(&bytes).as_str())
    {
        return Err("empreinte export invalide".into());
    }
    files.insert(format!("catalog/{relative}"), bytes);
    Ok(())
}
fn assemble(input: &Path) -> Result<(Manifest, BTreeMap<String, Vec<u8>>)> {
    let bytes = read_file(
        input,
        "manifest.json",
        olc_catalog_cache::MAX_MANIFEST as u64,
    )?;
    let export: Value = serde_json::from_slice(&bytes)?;
    if export["schema_version"] != 2 {
        return Err("schéma export invalide".into());
    }
    let version = export["version"]
        .as_str()
        .ok_or("version absente")?
        .to_owned();
    let mut files = BTreeMap::from([("catalog/manifest.json".into(), bytes)]);
    insert_metadata(input, &export["sources"], &mut files)?;
    let sources: Value = serde_json::from_slice(
        files
            .get("catalog/sources.json")
            .ok_or("sources absentes")?,
    )?;
    if sources["version"] != version
        || sources["normalizer_version"] != export["normalizer_version"]
    {
        return Err("provenance export incohérente".into());
    }
    for icon in sources["icons"].as_array().ok_or("icônes absentes")? {
        insert_metadata(input, icon, &mut files)?;
    }
    for locale in ["fr_FR", "en_US"] {
        insert_metadata(input, &export["locales"][locale], &mut files)?;
    }
    let mut index = BTreeMap::new();
    let mut directory = Vec::new();
    for (id, locales) in export["champions"].as_object().ok_or("champions absents")? {
        let numeric = id.parse::<u32>()?;
        if numeric == 0 || numeric.to_string() != *id {
            return Err("identité champion invalide".into());
        }
        let mut champions = BTreeMap::new();
        for locale in ["fr_FR", "en_US"] {
            insert_metadata(input, &locales[locale], &mut files)?;
            let path = locales[locale]["path"]
                .as_str()
                .ok_or("fiche absente")?
                .strip_prefix("/game-data/")
                .ok_or("préfixe invalide")?;
            let document: Value = serde_json::from_slice(&files[path])?;
            if document["version"] != version {
                return Err("version fiche incohérente".into());
            }
            let records = document["records"].as_array().ok_or("fiches absentes")?;
            let selected: Vec<_> = records.iter().filter(|r| r["kind"] == "champion").collect();
            if selected.len() != 1 || selected[0]["id"] != *id || selected[0]["locale"] != locale {
                return Err("champion incohérent".into());
            }
            champions.insert(locale, selected[0].clone());
        }
        let fr = &champions["fr_FR"];
        let en = &champions["en_US"];
        let key = en["fields"]["technical_id"]["value"]
            .as_str()
            .ok_or("clé champion absente")?;
        if fr["fields"]["technical_id"]["value"] != key || fr["icon"] != en["icon"] {
            return Err("langues incohérentes".into());
        }
        let icon = en["icon"]
            .as_str()
            .and_then(|p| p.strip_prefix("/game-data/"))
            .ok_or("icône champion absente")?;
        if !icon.ends_with(".png") {
            return Err("icône PNG requise".into());
        }
        let image = format!("champions/{id}.png");
        files.insert(
            image.clone(),
            files.get(icon).ok_or("icône non inventoriée")?.clone(),
        );
        index.insert(
            id.clone(),
            json!({"key":key,"fr":fr["name"],"en":en["name"]}),
        );
        directory.push(json!({"id":numeric,"key":key,"names":{"fr":fr["name"],"en":en["name"]},"titles":{"fr":fr["fields"]["title"]["value"],"en":en["fields"]["title"]["value"]},"categories":en["fields"]["categories"]["value"],"image":image}));
    }
    directory.sort_by_key(|c| c["id"].as_u64());
    files.insert("champions.json".into(), serde_json::to_vec(&index)?);
    files.insert(
        "champion-directory.json".into(),
        serde_json::to_vec(&json!({"version":version,"champions":directory}))?,
    );
    let mut manifest = Manifest {
        schema_version: 1,
        version,
        normalizer_version: export["normalizer_version"]
            .as_u64()
            .and_then(|n| n.try_into().ok())
            .ok_or("normaliseur absent")?,
        snapshot_id: String::new(),
        files: file_entries(&files),
    };
    manifest.snapshot_id = snapshot_id(&manifest)?;
    olc_catalog_cache::validate_manifest(&manifest)?;
    Ok((manifest, files))
}
fn file_entries(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, FileEntry> {
    files
        .iter()
        .map(|(path, bytes)| {
            (
                path.clone(),
                FileEntry {
                    bytes: bytes.len() as u64,
                    sha256: digest(bytes),
                    media_type: if path.ends_with(".json") {
                        "application/json"
                    } else if path.ends_with(".jpg") {
                        "image/jpeg"
                    } else {
                        "image/png"
                    }
                    .into(),
                },
            )
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let (mut manifest, mut files) = assemble(&args.input)?;
    let names: Value = serde_json::from_slice(&files["champions.json"])?;
    let downloaded = artworks::download(&manifest.version, &names).await?;
    artworks::apply(&manifest.version, &mut files, downloaded)?;
    manifest.files = file_entries(&files);
    manifest.snapshot_id = snapshot_id(&manifest)?;
    olc_catalog_cache::validate_manifest(&manifest)?;
    let mut cache = Cache::open(&args.output)?;
    for (path, bytes) in files {
        cache.put(&manifest.files[&path], &bytes)?;
    }
    cache.save(&manifest)?;
    olc_catalog_cache::validate_catalog(&cache, &manifest)?;
    cache.publish(&manifest)?;
    println!("{}", serde_json::to_string(&manifest)?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_reel_genere_index_et_images_coherents() {
        let input = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/desktop/public/game-data/catalog");
        let (mut manifest, mut files) = assemble(&input).unwrap();
        let names: Value = serde_json::from_slice(&files["champions.json"]).unwrap();
        let fixtures = names
            .as_object()
            .unwrap()
            .keys()
            .map(|id| {
                (
                    id.clone(),
                    artworks::Artwork {
                        bytes: vec![0xff, 0xd8, 0xff, 0xd9],
                        provenance: json!({"fixture":true,"champion_id":id}),
                    },
                )
            })
            .collect();
        artworks::apply(&manifest.version, &mut files, fixtures).unwrap();
        manifest.files = file_entries(&files);
        manifest.snapshot_id = snapshot_id(&manifest).unwrap();
        let directory: Value = serde_json::from_slice(&files["champion-directory.json"]).unwrap();
        let champions: Value = serde_json::from_slice(&files["champions.json"]).unwrap();
        assert_eq!(
            directory["champions"].as_array().unwrap().len(),
            champions.as_object().unwrap().len()
        );
        assert!(champions.get("103").is_some());
        for c in directory["champions"].as_array().unwrap() {
            assert!(files.contains_key(
                c["image"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("/game-data/")
                    .unwrap_or(c["image"].as_str().unwrap())
            ));
        }
        olc_catalog_cache::validate_manifest(&manifest).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let mut cache = Cache::open(directory.path()).unwrap();
        for (path, bytes) in &files {
            cache.put(&manifest.files[path], bytes).unwrap();
        }
        cache.save(&manifest).unwrap();
        olc_catalog_cache::validate_catalog(&cache, &manifest).unwrap();
        cache.publish(&manifest).unwrap();
        assert_eq!(
            cache.release(&manifest.version).unwrap().snapshot_id,
            manifest.snapshot_id
        );
    }
    #[test]
    fn refuse_corruption_et_traversee_dans_metadonnees() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sample.json"), b"{}").unwrap();
        let mut files = BTreeMap::new();
        let metadata =
            json!({"path":"/game-data/catalog/sample.json","bytes":2,"sha256":digest(b"{}")});
        insert_metadata(dir.path(), &metadata, &mut files).unwrap();
        std::fs::write(dir.path().join("sample.json"), b"[]").unwrap();
        assert!(insert_metadata(dir.path(), &metadata, &mut files).is_err());
        let metadata =
            json!({"path":"/game-data/catalog/../sample.json","bytes":2,"sha256":digest(b"[]")});
        assert!(insert_metadata(dir.path(), &metadata, &mut files).is_err());
    }
}
