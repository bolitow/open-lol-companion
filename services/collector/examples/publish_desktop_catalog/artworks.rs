use super::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub struct Artwork {
    pub bytes: Vec<u8>,
    pub provenance: Value,
}
fn patch(version: &str) -> Result<String> {
    let parts = olc_catalog_cache::version_parts(version).ok_or("version artwork invalide")?;
    Ok(format!("{}.{}", parts[0], parts[1]))
}
pub fn artwork_url(version: &str, id: u32, key: &str, metadata: &Value) -> Result<String> {
    let patch = patch(version)?;
    if metadata["id"].as_u64() != Some(id as u64)
        || !metadata["alias"]
            .as_str()
            .is_some_and(|alias| alias.eq_ignore_ascii_case(key))
    {
        return Err("identité artwork incohérente".into());
    }
    if metadata
        .get("version")
        .is_some_and(|v| v != version && v != &patch)
    {
        return Err("version artwork incohérente".into());
    }
    let skins: Vec<_> = metadata["skins"]
        .as_array()
        .ok_or("skins absents")?
        .iter()
        .filter(|s| s["isBase"] == true)
        .collect();
    if skins.len() != 1 || skins[0]["id"].as_u64() != Some(id as u64 * 1000) {
        return Err("skin de base absent ou ambigu".into());
    }
    let relative = skins[0]["loadScreenPath"]
        .as_str()
        .and_then(|s| s.strip_prefix("/lol-game-data/assets/"))
        .ok_or("chemin artwork invalide")?
        .to_ascii_lowercase();
    if !olc_catalog_cache::valid_path(&relative) || !relative.ends_with(".jpg") {
        return Err("chemin artwork invalide".into());
    }
    Ok(format!("https://raw.communitydragon.org/{patch}/plugins/rcp-be-lol-game-data/global/default/{relative}"))
}
fn append_bounded(bytes: &mut Vec<u8>, chunk: &[u8], limit: u64) -> Result<()> {
    if chunk.len() as u64 > limit.saturating_sub(bytes.len() as u64) {
        return Err("ressource trop grande".into());
    }
    bytes.extend_from_slice(chunk);
    Ok(())
}
async fn fetch(client: &reqwest::Client, url: &str, media: &str, limit: u64) -> Result<Vec<u8>> {
    let mut response = client.get(url).send().await?.error_for_status()?;
    if response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        != Some(media)
        || response.content_length().is_some_and(|n| n > limit)
    {
        return Err("type ou taille artwork invalide".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        append_bounded(&mut bytes, &chunk, limit)?;
    }
    Ok(bytes)
}
pub async fn download(version: &str, names: &Value) -> Result<BTreeMap<String, Artwork>> {
    let patch = patch(version)?;
    let client = reqwest::Client::builder()
        .use_preconfigured_tls(olc_collector::riot_client::public_tls_config()?)
        .https_only(true)
        .user_agent("OpenLoLCompanion/0.1 (desktop-artwork-export)")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let names = names.as_object().ok_or("index invalide")?;
    let mut pending = names.iter();
    let mut jobs = tokio::task::JoinSet::new();
    let mut results = BTreeMap::new();
    let mut total = 0u64;
    loop {
        while jobs.len() < 4 {
            let Some((id, entry)) = pending.next() else {
                break;
            };
            let numeric = id.parse::<u32>()?;
            let id = id.clone();
            let key = entry["key"].as_str().ok_or("clé absente")?.to_owned();
            let version = version.to_owned();
            let client = client.clone();
            let metadata_url=format!("https://raw.communitydragon.org/{patch}/plugins/rcp-be-lol-game-data/global/default/v1/champions/{numeric}.json");
            jobs.spawn(async move {
 let metadata_bytes=fetch(&client,&metadata_url,"application/json",olc_catalog_cache::MAX_JSON).await?;
 let metadata:Value=serde_json::from_slice(&metadata_bytes)?;let url=artwork_url(&version,numeric,&key,&metadata)?;
 let bytes=fetch(&client,&url,"image/jpeg",olc_catalog_cache::MAX_IMAGE).await?;
 if !bytes.starts_with(&[0xff,0xd8,0xff]) || !bytes.ends_with(&[0xff,0xd9]) {return Err("JPEG invalide".into());}
 let provenance=json!({"champion_id":numeric,"version":version,"metadata_url":metadata_url,"metadata_sha256":olc_catalog_cache::digest(&metadata_bytes),"base_skin":metadata["skins"].as_array().and_then(|skins|skins.iter().find(|s|s["isBase"]==true)),"image_url":url,"image_sha256":olc_catalog_cache::digest(&bytes)});
 Ok::<_,Box<dyn std::error::Error+Send+Sync>>((id,Artwork{bytes,provenance}))
 });
        }
        let Some(result) = jobs.join_next().await else {
            break;
        };
        let (id, artwork) = result??;
        total += artwork.bytes.len() as u64 + serde_json::to_vec(&artwork.provenance)?.len() as u64;
        if total > olc_catalog_cache::MAX_TOTAL {
            return Err("artworks trop grands".into());
        }
        results.insert(id, artwork);
    }
    Ok(results)
}
pub fn apply(
    version: &str,
    files: &mut BTreeMap<String, Vec<u8>>,
    artworks: BTreeMap<String, Artwork>,
) -> Result<()> {
    let mut directory: Value = serde_json::from_slice(
        files
            .get("champion-directory.json")
            .ok_or("répertoire absent")?,
    )?;
    let champions = directory["champions"]
        .as_array_mut()
        .ok_or("champions absents")?;
    if champions.len() != artworks.len() {
        return Err("artworks incomplets".into());
    }
    let mut sources = Vec::new();
    for champion in champions {
        let id = champion["id"]
            .as_u64()
            .ok_or("identité absente")?
            .to_string();
        let artwork = artworks.get(&id).ok_or("artwork absent")?;
        if artwork.bytes.len() as u64 > olc_catalog_cache::MAX_IMAGE
            || !artwork.bytes.starts_with(&[0xff, 0xd8, 0xff])
        {
            return Err("JPEG invalide".into());
        }
        let image = format!("champions/{id}.jpg");
        files.remove(&format!("champions/{id}.png"));
        files.insert(image.clone(), artwork.bytes.clone());
        champion["image"] = json!(image);
        sources.push(artwork.provenance.clone());
    }
    files.insert(
        "champion-directory.json".into(),
        serde_json::to_vec(&directory)?,
    );
    files.insert(
        "artwork-sources.json".into(),
        serde_json::to_vec(
            &json!({"version":version,"provider":"community_dragon","sources":sources}),
        )?,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapping_skin_base_verifie_identite_et_patch() {
        let metadata = json!({"id":103,"alias":"Ahri","skins":[{"id":103000,"isBase":true,"loadScreenPath":"/lol-game-data/assets/ASSETS/Characters/Ahri/Skins/Base/AhriLoadscreen_0.jpg"}]});
        assert_eq!(artwork_url("16.19.1",103,"Ahri",&metadata).unwrap(),"https://raw.communitydragon.org/16.19/plugins/rcp-be-lol-game-data/global/default/assets/characters/ahri/skins/base/ahriloadscreen_0.jpg");
        assert!(artwork_url("latest", 103, "Ahri", &metadata).is_err());
        assert!(artwork_url("16.19.1", 86, "Garen", &metadata).is_err());
        let mut casing = metadata.clone();
        casing["alias"] = json!("aHRI");
        assert!(artwork_url("16.19.1", 103, "Ahri", &casing).is_ok());
        let mut wrong_version = metadata.clone();
        wrong_version["version"] = json!("16.18");
        assert!(artwork_url("16.19.1", 103, "Ahri", &wrong_version).is_err());
        let mut duplicate = metadata.clone();
        duplicate["skins"]
            .as_array_mut()
            .unwrap()
            .push(metadata["skins"][0].clone());
        assert!(artwork_url("16.19.1", 103, "Ahri", &duplicate).is_err());
        let mut invalid = metadata.clone();
        invalid["skins"][0]["loadScreenPath"] = json!("/lol-game-data/assets/../secret.jpg");
        assert!(artwork_url("16.19.1", 103, "Ahri", &invalid).is_err());
    }
    #[test]
    fn flux_borne_sans_content_length() {
        let mut bytes = Vec::new();
        append_bounded(&mut bytes, b"123", 4).unwrap();
        assert!(append_bounded(&mut bytes, b"45", 4).is_err());
    }
    #[test]
    fn artwork_manquant_interdit_publication() {
        let mut files = BTreeMap::from([(
            "champion-directory.json".into(),
            serde_json::to_vec(&json!({"champions":[{"id":103}]})).unwrap(),
        )]);
        assert!(apply("16.19.1", &mut files, BTreeMap::new()).is_err());
    }
    #[tokio::test]
    #[ignore = "Recette réseau explicite : trois artworks CDragon versionnés, hors CI"]
    async fn trois_artworks_reels() {
        let files = download(
            "16.19.1",
            &json!({"103":{"key":"Ahri"},"86":{"key":"Garen"},"266":{"key":"Aatrox"}}),
        )
        .await
        .unwrap();
        assert_eq!(files.len(), 3);
        for artwork in files.values() {
            assert!(artwork.bytes.len() > 10000);
            assert!(artwork.provenance["image_url"]
                .as_str()
                .unwrap()
                .contains("/16.19/"));
        }
    }
}
