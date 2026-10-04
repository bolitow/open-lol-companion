//! Métadonnées publiques des cosmétiques ; aucune URL arbitraire dans le protocole natif.
use crate::{Error, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CosmeticSkin {
    pub tile: Option<String>,
    pub splash: Option<String>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cosmetics {
    pub schema_version: u32,
    pub version: String,
    pub profile_icons: Vec<u32>,
    pub skin_lines: BTreeMap<String, BTreeMap<String, String>>,
    pub skins: BTreeMap<String, CosmeticSkin>,
}
impl Cosmetics {
    pub fn parse(bytes: &[u8], version: &str) -> Result<Self> {
        if bytes.len() as u64 > crate::MAX_JSON {
            return Err(Error::Invalid);
        }
        let value: serde_json::Value = serde_json::from_slice(bytes)?;
        if value
            .get("skins")
            .and_then(|v| v.as_object())
            .map_or(true, |skins| {
                skins
                    .values()
                    .any(|s| s.get("tile").is_none() || s.get("splash").is_none())
            })
        {
            return Err(Error::Invalid);
        }
        let c: Self = serde_json::from_value(value)?;
        let numeric = |s: &str| s.parse::<u32>().is_ok_and(|n| n.to_string() == s);
        if c.schema_version != 1
            || c.version != version
            || crate::version_parts(version).is_none()
            || c.profile_icons.is_empty()
            || c.profile_icons.len() > 50_000
            || c.profile_icons.iter().collect::<BTreeSet<_>>().len() != c.profile_icons.len()
            || c.skins.is_empty()
            || c.skins.len() > 20_000
            || c.skin_lines.len() != 2
        {
            return Err(Error::Invalid);
        }
        for locale in ["fr", "en"] {
            let lines = c.skin_lines.get(locale).ok_or(Error::Invalid)?;
            if lines.is_empty()
                || lines.len() > 10_000
                || lines.iter().any(|(id, n)| {
                    !numeric(id)
                        || n.len() > 1024
                        || n.chars().any(char::is_control)
                        || (id != "0" && n.is_empty())
                })
            {
                return Err(Error::Invalid);
            }
        }
        if c.skin_lines["fr"].keys().ne(c.skin_lines["en"].keys()) {
            return Err(Error::Invalid);
        }
        for (id, skin) in &c.skins {
            if !numeric(id)
                || id == "0"
                || [&skin.tile, &skin.splash]
                    .iter()
                    .any(|p| p.as_ref().is_some_and(|p| !image_path(p)))
            {
                return Err(Error::Invalid);
            }
        }
        Ok(c)
    }
    pub fn image_url(&self, kind: &str, id: u32) -> Option<String> {
        if kind == "profile" {
            return self.profile_icons.contains(&id).then(|| {
                format!(
                    "https://ddragon.leagueoflegends.com/cdn/{}/img/profileicon/{id}.png",
                    self.version
                )
            });
        }
        let s = self.skins.get(&id.to_string())?;
        let path = match kind {
            "tile" => s.tile.as_ref(),
            "splash" => s.splash.as_ref(),
            _ => None,
        }?;
        let parts = crate::version_parts(&self.version)?;
        Some(format!("https://raw.communitydragon.org/{}.{}/plugins/rcp-be-lol-game-data/global/default/{path}",parts[0],parts[1]))
    }
}
fn image_path(path: &str) -> bool {
    path.starts_with("assets/")
        && path.len() <= 512
        && path == path.to_ascii_lowercase()
        && [".png", ".jpg", ".jpeg", ".webp"]
            .iter()
            .any(|ext| path.ends_with(ext))
        && path.split('/').all(|s| {
            !s.is_empty()
                && s != "."
                && s != ".."
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn data() -> serde_json::Value {
        json!({"schema_version":1,"version":"16.19.1","profile_icons":[0,5],"skin_lines":{"fr":{"0":"","1":"Floraison"},"en":{"0":"","1":"Blossom"}},"skins":{"103001":{"tile":"assets/characters/ahri/tile.jpg","splash":null}}})
    }
    #[test]
    fn assets_connus_uniquement_et_patch_explicit() {
        let c = Cosmetics::parse(&serde_json::to_vec(&data()).unwrap(), "16.19.1").unwrap();
        assert_eq!(
            c.image_url("profile", 0).as_deref(),
            Some("https://ddragon.leagueoflegends.com/cdn/16.19.1/img/profileicon/0.png")
        );
        assert_eq!(c.image_url("tile",103001).as_deref(),Some("https://raw.communitydragon.org/16.19/plugins/rcp-be-lol-game-data/global/default/assets/characters/ahri/tile.jpg"));
        assert!(c.image_url("profile", 99).is_none());
        assert!(c.image_url("splash", 103001).is_none());
        assert!(c.image_url("unknown", 103001).is_none());
    }
    #[test]
    fn refuse_versions_chemins_et_identites_incoherentes() {
        for (p, v) in [
            ("/version", json!("latest")),
            ("/schema_version", json!(2)),
            ("/skins/103001/tile", json!("assets/../secret.png")),
            ("/skins/103001/tile", json!("https://evil/image.png")),
            ("/skins/103001/tile", json!("assets/image.svg")),
            ("/profile_icons", json!([1, 1])),
            ("/skin_lines/en", json!({"2":"Mismatch"})),
        ] {
            let mut x = data();
            *x.pointer_mut(p).unwrap() = v;
            assert!(
                Cosmetics::parse(&serde_json::to_vec(&x).unwrap(), "16.19.1").is_err(),
                "{p}"
            );
        }
        assert!(Cosmetics::parse(&serde_json::to_vec(&data()).unwrap(), "16.20.1").is_err());
        let mut missing = data();
        missing["skins"]["103001"]
            .as_object_mut()
            .unwrap()
            .remove("tile");
        assert!(Cosmetics::parse(&serde_json::to_vec(&missing).unwrap(), "16.19.1").is_err());
    }
}
