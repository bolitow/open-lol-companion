use crate::*;
use serde_json::Value;
use std::collections::BTreeSet;
fn json(c: &Cache, m: &Manifest, path: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&c.read(&m.snapshot_id, path)?)?)
}
fn records<'a>(v: &'a Value, m: &Manifest) -> Result<&'a Vec<Value>> {
    if v["version"] != m.version {
        return Err(Error::Invalid);
    }
    v["records"]
        .as_array()
        .filter(|r| !r.is_empty())
        .ok_or(Error::Invalid)
}
fn record(r: &Value, locale: &str, m: &Manifest) -> Result<()> {
    if r["namespace"] != "standard"
        || r["locale"] != locale
        || r["name"].as_str().is_none()
        || r["id"].as_str().is_none()
        || !r
            .get("description")
            .is_some_and(|v| v.is_null() || v.is_string())
        || !r.get("icon").is_some_and(|v| v.is_null() || v.is_string())
        || !r["fields"].is_object()
        || !r["stats"].is_object()
        || !r["effects"].is_array()
        || !r["coverage"].is_object()
    {
        return Err(Error::Invalid);
    }
    if !r["icon"].is_null() {
        let path = r["icon"]
            .as_str()
            .and_then(|s| s.strip_prefix("/game-data/"))
            .ok_or(Error::Invalid)?;
        if !path.starts_with("catalog/icons/") || !m.files.contains_key(path) {
            return Err(Error::Invalid);
        }
    }
    Ok(())
}
/// Vérifie le contrat métier avant l’activation ; les hashes seuls ne prouvent pas la cohérence.
pub fn validate_catalog(c: &Cache, m: &Manifest) -> Result<()> {
    c.verify(m)?;
    let directory = json(c, m, "champion-directory.json")?;
    if directory["version"] != m.version {
        return Err(Error::Invalid);
    }
    let entries = directory["champions"]
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or(Error::Invalid)?;
    let names = json(c, m, "champions.json")?;
    let names = names.as_object().ok_or(Error::Invalid)?;
    let mut ids = BTreeSet::new();
    for entry in entries {
        let id = entry["id"]
            .as_u64()
            .filter(|n| *n > 0 && *n <= u32::MAX as u64)
            .ok_or(Error::Invalid)?
            .to_string();
        if !ids.insert(id.clone())
            || entry["key"]
                .as_str()
                .filter(|k| {
                    !k.is_empty()
                        && k.len() <= 64
                        && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                })
                .is_none()
            || !entry["categories"].as_array().is_some_and(|items| {
                items.iter().all(|item| {
                    item.as_str().is_some_and(|s| {
                        ["Assassin", "Fighter", "Mage", "Marksman", "Support", "Tank"].contains(&s)
                    })
                })
            })
        {
            return Err(Error::Invalid);
        }
        let image = entry["image"].as_str().ok_or(Error::Invalid)?;
        if !image.starts_with("champions/")
            || !m
                .files
                .get(image)
                .is_some_and(|f| f.media_type.starts_with("image/"))
        {
            return Err(Error::Invalid);
        }
        for (short, locale) in [("fr", "fr_FR"), ("en", "en_US")] {
            let name = entry["names"][short]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or(Error::Invalid)?;
            let n = names.get(&id).ok_or(Error::Invalid)?;
            if n[short] != name
                || n["key"] != entry["key"]
                || entry["titles"][short].as_str().is_none()
            {
                return Err(Error::Invalid);
            }
            let detail = json(c, m, &format!("catalog/champions/{id}/{locale}.json"))?;
            let rows = records(&detail, m)?;
            let mut found = BTreeSet::new();
            for r in rows {
                record(r, locale, m)?;
                let rid = r["id"].as_str().ok_or(Error::Invalid)?;
                if !found.insert(rid.to_string())
                    || (rid == id && (r["kind"] != "champion" || r["name"] != name))
                    || (rid != id && r["kind"] != "ability")
                {
                    return Err(Error::Invalid);
                }
            }
            let expected: BTreeSet<_> = std::iter::once(id.clone())
                .chain(["Q", "W", "E", "R", "passive"].map(|slot| format!("{id}:{slot}")))
                .collect();
            if found != expected {
                return Err(Error::Invalid);
            }
        }
    }
    if ids != names.keys().cloned().collect() {
        return Err(Error::Invalid);
    }
    let mut language_ids = None;
    for locale in ["fr_FR", "en_US"] {
        let root = json(c, m, &format!("catalog/{locale}.json"))?;
        let mut found = BTreeSet::new();
        for r in records(&root, m)? {
            record(r, locale, m)?;
            let kind = r["kind"].as_str().ok_or(Error::Invalid)?;
            let id = r["id"].as_str().ok_or(Error::Invalid)?;
            if !["rune", "rune_shard", "item", "summoner_spell", "augment"].contains(&kind)
                || !found.insert((kind.to_owned(), id.to_owned()))
            {
                return Err(Error::Invalid);
            }
        }
        for required in ["rune", "rune_shard", "item", "summoner_spell"] {
            if !found.iter().any(|(kind, _)| kind == required) {
                return Err(Error::Invalid);
            }
        }
        if language_ids
            .as_ref()
            .is_some_and(|expected| expected != &found)
        {
            return Err(Error::Invalid);
        }
        language_ids = Some(found);
    }
    Ok(())
}
