//! Métadonnées cosmétiques publiques du patch ; les images restent téléchargées à la demande.
use super::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;
fn numeric(value: &Value) -> Result<u32> {
    if let Some(n) = value.as_u64() {
        return Ok(n.try_into()?);
    }
    let text = value.as_str().ok_or("identité cosmétique invalide")?;
    let n = text.parse::<u32>()?;
    if n.to_string() != text {
        return Err("identité cosmétique non canonique".into());
    }
    Ok(n)
}
fn image_path(value: &Value) -> Result<Option<String>> {
    if value.is_null() {
        return Ok(None);
    }
    let path = value
        .as_str()
        .and_then(|s| s.strip_prefix("/lol-game-data/assets/"))
        .ok_or("chemin cosmétique invalide")?
        .to_ascii_lowercase();
    if !path.starts_with("assets/")
        || path.len() > 512
        || ![".png", ".jpg", ".jpeg", ".webp"]
            .iter()
            .any(|ext| path.ends_with(ext))
        || !path.split('/').all(|s| {
            !s.is_empty()
                && s != "."
                && s != ".."
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
    {
        return Err("chemin cosmétique invalide".into());
    }
    Ok(Some(path))
}
fn skin_lines(value: &Value) -> Result<BTreeMap<String, String>> {
    let records = value
        .as_array()
        .filter(|r| !r.is_empty())
        .ok_or("séries absentes")?;
    let mut lines = BTreeMap::new();
    for record in records {
        let id = numeric(&record["id"])?;
        let name = record["name"].as_str().ok_or("nom de série absent")?;
        if name.len() > 1024
            || (id != 0 && name.is_empty())
            || lines.insert(id.to_string(), name.to_owned()).is_some()
        {
            return Err("série cosmétique invalide".into());
        }
    }
    Ok(lines)
}
fn normalize(version: &str, icons: &Value, skins: &Value, fr: &Value, en: &Value) -> Result<Value> {
    if olc_catalog_cache::version_parts(version).is_none()
        || icons["version"] != version
        || icons["type"] != "profileicon"
    {
        return Err("version cosmétique incohérente".into());
    }
    let mut profile_icons = Vec::new();
    for (key, record) in icons["data"]
        .as_object()
        .filter(|v| !v.is_empty())
        .ok_or("icônes absentes")?
    {
        let id = numeric(&record["id"])?;
        if id.to_string() != *key {
            return Err("identité icône incohérente".into());
        }
        profile_icons.push(id);
    }
    profile_icons.sort_unstable();
    let mut skin_images = BTreeMap::new();
    for (key, record) in skins
        .as_object()
        .filter(|v| !v.is_empty())
        .ok_or("skins absents")?
    {
        let id = numeric(&record["id"])?;
        if id == 0 || id.to_string() != *key {
            return Err("identité skin incohérente".into());
        }
        skin_images.insert(key.clone(),json!({"tile":image_path(&record["tilePath"])?,"splash":image_path(&record["splashPath"])?}));
    }
    let fr = skin_lines(fr)?;
    let en = skin_lines(en)?;
    if !fr.keys().eq(en.keys()) {
        return Err("langues des séries incohérentes".into());
    }
    Ok(
        json!({"schema_version":1,"version":version,"profile_icons":profile_icons,"skin_lines":{"fr":fr,"en":en},"skins":skin_images}),
    )
}
fn append_bounded(bytes: &mut Vec<u8>, chunk: &[u8]) -> Result<()> {
    if chunk.len() as u64 > olc_catalog_cache::MAX_JSON.saturating_sub(bytes.len() as u64) {
        return Err("métadonnées cosmétiques trop grandes".into());
    }
    bytes.extend_from_slice(chunk);
    Ok(())
}
async fn fetch(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let mut response = client.get(url).send().await?.error_for_status()?;
    if response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        != Some("application/json")
        || response
            .content_length()
            .is_some_and(|n| n > olc_catalog_cache::MAX_JSON)
    {
        return Err("réponse cosmétique invalide".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        append_bounded(&mut bytes, &chunk)?;
    }
    Ok(bytes)
}
pub async fn download(version: &str) -> Result<BTreeMap<String, Vec<u8>>> {
    let parts = olc_catalog_cache::version_parts(version).ok_or("version cosmétique invalide")?;
    let base = format!(
        "https://raw.communitydragon.org/{}.{}/plugins/rcp-be-lol-game-data/global",
        parts[0], parts[1]
    );
    let urls = [
        format!("https://ddragon.leagueoflegends.com/cdn/{version}/data/en_US/profileicon.json"),
        format!("{base}/default/v1/skins.json"),
        format!("{base}/fr_fr/v1/skinlines.json"),
        format!("{base}/default/v1/skinlines.json"),
    ];
    let client = reqwest::Client::builder()
        .use_preconfigured_tls(olc_collector::riot_client::public_tls_config()?)
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("OpenLoLCompanion/0.1 (desktop-cosmetics-export)")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let (icons, skins, fr, en) = tokio::try_join!(
        fetch(&client, &urls[0]),
        fetch(&client, &urls[1]),
        fetch(&client, &urls[2]),
        fetch(&client, &urls[3])
    )?;
    let payloads = [icons, skins, fr, en];
    let documents: Vec<Value> = payloads
        .iter()
        .map(|b| serde_json::from_slice(b))
        .collect::<std::result::Result<_, _>>()?;
    let cosmetics = normalize(
        version,
        &documents[0],
        &documents[1],
        &documents[2],
        &documents[3],
    )?;
    let sources:Vec<_>=urls.iter().zip(&payloads).map(|(url,bytes)|json!({"url":url,"bytes":bytes.len(),"sha256":olc_catalog_cache::digest(bytes)})).collect();
    Ok(BTreeMap::from([
        ("cosmetics.json".into(), serde_json::to_vec(&cosmetics)?),
        (
            "cosmetics-sources.json".into(),
            serde_json::to_vec(&json!({"schema_version":1,"version":version,"sources":sources}))?,
        ),
    ]))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixtures() -> [Value; 4] {
        [
            json!({"type":"profileicon","version":"16.19.1","data":{"0":{"id":0},"50":{"id":"50"}}}),
            json!({"103000":{"id":103000,"tilePath":"/lol-game-data/assets/ASSETS/Characters/Ahri/Skins/Base/Images/Ahri_Tile.JPG","splashPath":null}}),
            json!([{"id":0,"name":""},{"id":1,"name":"Champions du monde"}]),
            json!([{"id":0,"name":""},{"id":1,"name":"World Champions"}]),
        ]
    }
    #[test]
    fn normalise_identites_langues_et_chemins_sans_url() {
        let [icons, skins, fr, en] = fixtures();
        let result = normalize("16.19.1", &icons, &skins, &fr, &en).unwrap();
        assert_eq!(
            result,
            json!({"schema_version":1,"version":"16.19.1","profile_icons":[0,50],"skin_lines":{"fr":{"0":"","1":"Champions du monde"},"en":{"0":"","1":"World Champions"}},"skins":{"103000":{"tile":"assets/characters/ahri/skins/base/images/ahri_tile.jpg","splash":null}}})
        );
    }
    #[test]
    fn refuse_version_identites_et_paths_incoherents() {
        let [mut icons, skins, fr, en] = fixtures();
        icons["version"] = json!("16.18.1");
        assert!(normalize("16.19.1", &icons, &skins, &fr, &en).is_err());
        let [icons, mut skins, fr, en] = fixtures();
        skins["103000"]["id"] = json!(86000);
        assert!(normalize("16.19.1", &icons, &skins, &fr, &en).is_err());
        for path in [
            "https://evil.example/a.jpg",
            "/lol-game-data/assets/../a.jpg",
            "/lol-game-data/assets/ASSETS/a.svg",
            "/lol-game-data/assets/ASSETS/a.jpg?token=x",
        ] {
            let [icons, mut skins, fr, en] = fixtures();
            skins["103000"]["tilePath"] = json!(path);
            assert!(normalize("16.19.1", &icons, &skins, &fr, &en).is_err());
        }
    }
    #[test]
    fn refuse_langue_incomplete_et_doublons_de_series() {
        let [icons, skins, fr, mut en] = fixtures();
        en.as_array_mut().unwrap().pop();
        assert!(normalize("16.19.1", &icons, &skins, &fr, &en).is_err());
        let [icons, skins, mut fr, en] = fixtures();
        fr.as_array_mut()
            .unwrap()
            .push(json!({"id":1,"name":"Doublon"}));
        assert!(normalize("16.19.1", &icons, &skins, &fr, &en).is_err());
    }
    #[test]
    fn extensions_bornees_et_octets_reels() {
        for extension in ["png", "jpg", "jpeg", "webp"] {
            assert_eq!(
                image_path(&json!(format!(
                    "/lol-game-data/assets/ASSETS/skin.{extension}"
                )))
                .unwrap(),
                Some(format!("assets/skin.{extension}"))
            );
        }
        let mut bytes = vec![0; olc_catalog_cache::MAX_JSON as usize];
        assert!(append_bounded(&mut bytes, b"x").is_err());
        assert_eq!(bytes.len(), olc_catalog_cache::MAX_JSON as usize);
    }
    #[tokio::test]
    #[ignore = "Recette réseau explicite des quatre sources versionnées, hors CI"]
    async fn sources_reelles_versionnees() {
        let files = download("16.19.1").await.unwrap();
        let catalog: Value = serde_json::from_slice(&files["cosmetics.json"]).unwrap();
        assert!(catalog["profile_icons"].as_array().unwrap().len() > 1000);
        assert!(catalog["skins"].as_object().unwrap().len() > 1000);
        assert!(catalog["skin_lines"]["fr"].as_object().unwrap().len() > 100);
        let sources: Value = serde_json::from_slice(&files["cosmetics-sources.json"]).unwrap();
        assert_eq!(sources["sources"].as_array().unwrap().len(), 4);
        for source in sources["sources"].as_array().unwrap() {
            assert_eq!(source["sha256"].as_str().unwrap().len(), 64);
            assert!(source["bytes"].as_u64().unwrap() > 0);
        }
    }
}
