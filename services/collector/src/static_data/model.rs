use std::collections::BTreeSet;

use serde_json::Value;

use super::StaticError;

type Version = (u32, u32, u32);

fn version_parts(value: &str) -> Option<Version> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

pub(super) fn patch(version: &str) -> Result<String, StaticError> {
    let (major, minor, _) = version_parts(version).ok_or(StaticError::InvalidManifest)?;
    Ok(format!("{major}.{minor}"))
}

pub(super) fn select_versions(
    versions: &Value,
    realm: &Value,
    count: usize,
) -> Result<Vec<String>, StaticError> {
    if !(1..=10).contains(&count) {
        return Err(StaticError::InvalidCount);
    }
    let live = realm
        .get("v")
        .and_then(Value::as_str)
        .and_then(version_parts)
        .ok_or(StaticError::InvalidManifest)?;
    let mut candidates = BTreeSet::new();
    for entry in versions.as_array().ok_or(StaticError::InvalidManifest)? {
        let value = entry.as_str().ok_or(StaticError::InvalidManifest)?;
        // versions.json conserve d'anciennes archives non sélectionnables (lolpatch_7.20).
        if let Some(legacy) = value.strip_prefix("lolpatch_") {
            let parts: Vec<_> = legacy.split('.').collect();
            if parts.len() == 2
                && parts
                    .iter()
                    .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            {
                continue;
            }
        }
        let version = version_parts(value).ok_or(StaticError::InvalidManifest)?;
        if (version.0, version.1) <= (live.0, live.1) {
            candidates.insert(version);
        }
    }
    let mut patches = BTreeSet::new();
    let selected: Vec<_> = candidates
        .into_iter()
        .rev()
        .filter(|(major, minor, _)| patches.insert((*major, *minor)))
        .take(count)
        .map(|(major, minor, build)| format!("{major}.{minor}.{build}"))
        .collect();
    if selected.len() != count
        || selected
            .first()
            .and_then(|v| version_parts(v))
            .map(|v| (v.0, v.1))
            != Some((live.0, live.1))
    {
        return Err(StaticError::InvalidManifest);
    }
    Ok(selected)
}

#[derive(Debug, Clone)]
pub(super) enum Kind {
    ChampionList,
    Champion { id: String, key: String },
    Item,
    Summoner,
    Runes,
    Map,
    ProfileIcon,
    Queues,
    Maps,
    Modes,
    GameTypes,
}

fn require(condition: bool) -> Result<(), StaticError> {
    if condition {
        Ok(())
    } else {
        Err(StaticError::InvalidDocument)
    }
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, StaticError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or(StaticError::InvalidDocument)
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn positive_key(value: &str) -> bool {
    value.parse::<u32>().is_ok_and(|n| n > 0)
}

fn image(value: &Value) -> Result<(), StaticError> {
    let image = value.get("image").ok_or(StaticError::InvalidDocument)?;
    let file = text(image, "full")?;
    require(!file.contains('/') && !file.contains('\\') && !file.contains(".."))
}

fn validate_runes(value: &Value) -> Result<(), StaticError> {
    let styles = value
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or(StaticError::InvalidDocument)?;
    let mut ids = BTreeSet::new();
    for style in styles {
        let id = style
            .get("id")
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
            .ok_or(StaticError::InvalidDocument)?;
        require(ids.insert(id))?;
        for field in ["key", "name", "icon"] {
            text(style, field)?;
        }
        let slots = style
            .get("slots")
            .and_then(Value::as_array)
            .filter(|a| !a.is_empty())
            .ok_or(StaticError::InvalidDocument)?;
        for slot in slots {
            let runes = slot
                .get("runes")
                .and_then(Value::as_array)
                .filter(|a| !a.is_empty())
                .ok_or(StaticError::InvalidDocument)?;
            for rune in runes {
                let id = rune
                    .get("id")
                    .and_then(Value::as_u64)
                    .filter(|n| *n > 0)
                    .ok_or(StaticError::InvalidDocument)?;
                require(ids.insert(id))?;
                for field in ["key", "name", "icon", "shortDesc", "longDesc"] {
                    text(rune, field)?;
                }
            }
        }
    }
    Ok(())
}

// Formats vérifiés le 01/10/2026 : https://developer.riotgames.com/docs/lol#data-dragon
// et https://developer.riotgames.com/docs/lol#game-constants.
pub(super) fn validate_document(
    kind: &Kind,
    version: &str,
    value: &Value,
) -> Result<(), StaticError> {
    if matches!(kind, Kind::Runes) {
        return validate_runes(value);
    }
    let catalog_fields = match kind {
        Kind::Queues => Some(("queueId", "map", true)),
        Kind::Maps => Some(("mapId", "mapName", true)),
        Kind::Modes => Some(("gameMode", "description", false)),
        Kind::GameTypes => Some(("gametype", "description", false)),
        _ => None,
    };
    if let Some((id_field, name_field, numeric)) = catalog_fields {
        let entries = value
            .as_array()
            .filter(|a| !a.is_empty())
            .ok_or(StaticError::InvalidDocument)?;
        let mut ids = BTreeSet::new();
        for entry in entries {
            let id = if numeric {
                entry
                    .get(id_field)
                    .and_then(Value::as_u64)
                    .ok_or(StaticError::InvalidDocument)?
                    .to_string()
            } else {
                text(entry, id_field)?.to_owned()
            };
            require(ids.insert(id))?;
            text(entry, name_field)?;
        }
        return Ok(());
    }
    let expected_type = match kind {
        Kind::ChampionList | Kind::Champion { .. } => "champion",
        Kind::Item => "item",
        Kind::Summoner => "summoner",
        Kind::Map => "map",
        Kind::ProfileIcon => "profileicon",
        _ => return Err(StaticError::InvalidDocument),
    };
    require(text(value, "type")? == expected_type && text(value, "version")? == version)?;
    let data = value
        .get("data")
        .and_then(Value::as_object)
        .filter(|o| !o.is_empty())
        .ok_or(StaticError::InvalidDocument)?;
    if matches!(kind, Kind::Champion { .. }) {
        require(data.len() == 1)?;
    }
    let mut ids = BTreeSet::new();
    for (key, entry) in data {
        image(entry)?;
        match kind {
            Kind::ChampionList | Kind::Champion { .. } | Kind::Summoner => {
                require(identifier(key) && text(entry, "id")? == key)?;
                let numeric = text(entry, "key")?;
                require(positive_key(numeric) && ids.insert(numeric.to_owned()))?;
                text(entry, "name")?;
                if let Some(declared) = entry.get("version") {
                    require(declared.as_str() == Some(version))?;
                }
                if let Kind::Champion {
                    id,
                    key: expected_key,
                } = kind
                {
                    require(id == key && expected_key == numeric)?;
                    let spells = entry
                        .get("spells")
                        .and_then(Value::as_array)
                        .filter(|s| s.len() == 4)
                        .ok_or(StaticError::InvalidDocument)?;
                    let mut spells_ids = BTreeSet::new();
                    for spell in spells {
                        require(spells_ids.insert(text(spell, "id")?))?;
                        text(spell, "name")?;
                        text(spell, "description")?;
                        image(spell)?;
                    }
                    let passive = entry.get("passive").ok_or(StaticError::InvalidDocument)?;
                    text(passive, "name")?;
                    text(passive, "description")?;
                    image(passive)?;
                }
            }
            // Certains identifiants réservés officiels ont un libellé vide : garder
            // la valeur source, sans en fabriquer un ni supprimer l'identifiant.
            Kind::Item => {
                require(positive_key(key) && entry.get("name").is_some_and(Value::is_string))?;
            }
            Kind::Map => {
                require(
                    key.parse::<u32>().is_ok()
                        && text(entry, "MapId")? == key
                        && entry.get("MapName").is_some_and(Value::is_string),
                )?;
            }
            Kind::ProfileIcon => {
                let expected = key
                    .parse::<u64>()
                    .map_err(|_| StaticError::InvalidDocument)?;
                let declared = entry.get("id").and_then(|id| {
                    id.as_u64()
                        .or_else(|| id.as_str().and_then(|text| text.parse::<u64>().ok()))
                });
                require(declared == Some(expected))?;
            }
            _ => return Err(StaticError::InvalidDocument),
        }
    }
    Ok(())
}
