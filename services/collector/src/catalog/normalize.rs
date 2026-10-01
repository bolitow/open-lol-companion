//! Projection pure des catalogues publics, sans téléchargement ni publication.
use super::model::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[path = "project_value.rs"]
mod project_value;
use project_value::{escape, positive_id, Builder, Shape};
pub(super) fn plain_text(input: &str) -> String {
    project_value::plain_text(input)
}

/// Projette toutes les entrées reconnues ; les valeurs absentes ne sont jamais imputées.
pub fn normalize(
    version: &str,
    sources: &[CatalogSource],
) -> Result<Vec<CatalogRecord>, CatalogError> {
    if !super::valid_version(version) {
        return Err(CatalogError::InvalidRequest);
    }
    if sources.is_empty() {
        return Err(CatalogError::InvalidSource);
    }
    let mut records = BTreeMap::new();
    let mut seen = BTreeMap::new();
    let mut ordered: Vec<_> = sources.iter().collect();
    ordered.sort_by(|a, b| (&a.provider, &a.key, &a.id).cmp(&(&b.provider, &b.key, &b.id)));
    for source in ordered {
        if let Some(previous) = seen.insert(&source.id, source) {
            if previous != source {
                return Err(CatalogError::InvalidSource);
            }
            continue;
        }
        let projected = match source.provider.as_str() {
            "ddragon" => project_dragon(version, source)?,
            "riot_catalog" => project_catalog(source)?,
            // Les autres fournisseurs ont leur adaptateur et ne sont jamais assimilés à Riot.
            _ => continue,
        };
        for record in projected {
            merge_record(&mut records, record);
        }
    }
    if records.is_empty() {
        return Err(CatalogError::InvalidSource);
    }
    let mut records: Vec<_> = records.into_values().collect();
    finalize(&mut records);
    Ok(records)
}

fn project_dragon(
    version: &str,
    source: &CatalogSource,
) -> Result<Vec<CatalogRecord>, CatalogError> {
    let locale = source
        .locale
        .as_deref()
        .ok_or(CatalogError::InvalidSource)?;
    if source.version != version || !["fr_FR", "en_US"].contains(&locale) {
        return Err(CatalogError::InvalidSource);
    }
    let resource = source
        .key
        .strip_prefix(&format!("{locale}/"))
        .ok_or(CatalogError::InvalidSource)?;
    let namespace = if resource.starts_with("mode/classic/") {
        "classic"
    } else {
        "standard"
    };
    let resource = resource.strip_prefix("mode/classic/").unwrap_or(resource);
    if resource == "runesReforged.json" {
        return project_runes(version, source);
    }
    if source.data["version"].as_str() != Some(version) {
        return Err(CatalogError::InvalidSource);
    }
    let kind = match resource {
        "item.json" => "item",
        "champion.json" => "champion",
        "summoner.json" => "summoner_spell",
        "map.json" => "map",
        "profileicon.json" => "profile_icon",
        value if value.starts_with("champion/") && value.ends_with(".json") => "champion",
        _ => return Err(CatalogError::InvalidSource),
    };
    let expected = match kind {
        "summoner_spell" => "summoner",
        "profile_icon" => "profileicon",
        other => other,
    };
    if source.data["type"].as_str() != Some(expected) {
        return Err(CatalogError::InvalidSource);
    }
    let entries = source.data["data"]
        .as_object()
        .ok_or(CatalogError::InvalidSource)?;
    if let Some(champion) = resource
        .strip_prefix("champion/")
        .and_then(|s| s.strip_suffix(".json"))
    {
        if entries.len() != 1 || !entries.contains_key(champion) {
            return Err(CatalogError::InvalidSource);
        }
    }
    let mut result = Vec::new();
    let mut identities = BTreeSet::new();
    for (key, entry) in entries {
        if !entry.is_object() {
            return Err(CatalogError::InvalidSource);
        }
        let id = match kind {
            "champion" | "summoner_spell" => {
                if entry["id"].as_str() != Some(key) {
                    return Err(CatalogError::InvalidSource);
                }
                positive_id(&entry["key"]).ok_or(CatalogError::InvalidSource)?
            }
            "profile_icon" => key
                .parse::<u32>()
                .map_err(|_| CatalogError::InvalidSource)?
                .to_string(),
            _ => positive_id(&Value::String(key.clone())).ok_or(CatalogError::InvalidSource)?,
        };
        if !identities.insert(id.clone()) {
            return Err(CatalogError::InvalidSource);
        }
        let pointer = format!("/data/{}", escape(key));
        let mut builder = Builder::new(source, entry, pointer.clone(), kind, id.clone(), namespace);
        if kind != "profile_icon" {
            builder.field("technical_id", "/id", Shape::Text, None);
        }
        builder.field("numeric_id", "/key", Shape::Text, None);
        match kind {
            "item" => project_item(&mut builder, version),
            "champion" => {
                project_champion(&mut builder, version);
                if let Some(passive) = entry.get("passive") {
                    if !passive.is_object() {
                        return Err(CatalogError::InvalidSource);
                    }
                    result.push(project_ability(
                        version,
                        source,
                        passive,
                        format!("{pointer}/passive"),
                        &id,
                        "passive",
                        namespace,
                    ));
                    builder.delegate("/passive");
                }
                if let Some(spells) = entry.get("spells") {
                    let spells = spells
                        .as_array()
                        .filter(|s| s.len() == 4)
                        .ok_or(CatalogError::InvalidSource)?;
                    for (index, slot) in ["Q", "W", "E", "R"].into_iter().enumerate() {
                        if !spells[index].is_object() {
                            return Err(CatalogError::InvalidSource);
                        }
                        result.push(project_ability(
                            version,
                            source,
                            &spells[index],
                            format!("{pointer}/spells/{index}"),
                            &id,
                            slot,
                            namespace,
                        ));
                        builder.delegate(&format!("/spells/{index}"));
                    }
                }
            }
            "summoner_spell" => {
                common_spell(&mut builder, version, "spell");
                builder.field("modes", "/modes", Shape::Strings, None);
                builder.field(
                    "required_level",
                    "/summonerLevel",
                    Shape::Number,
                    Some("level"),
                );
            }
            "map" => {
                if entry["MapId"].as_str() != Some(key) {
                    return Err(CatalogError::InvalidSource);
                }
                builder.field("map_id", "/MapId", Shape::Text, None);
                builder.name("/MapName");
                builder.icon("/image/full", version, "map");
            }
            "profile_icon" => {
                let declared = entry["id"]
                    .as_u64()
                    .map(|n| n.to_string())
                    .or_else(|| entry["id"].as_str().map(str::to_owned));
                if declared.as_deref() != Some(id.as_str()) {
                    return Err(CatalogError::InvalidSource);
                }
                builder.consume("/id");
                builder.derived("numeric_id", json!(id), "/id", None);
                builder.record.fields.remove("technical_id");
                builder.icon("/image/full", version, "profileicon");
            }
            _ => return Err(CatalogError::InvalidSource),
        }
        result.push(builder.finish());
    }
    if let Some(record) = result.iter_mut().find(|r| r.kind == kind) {
        project_value::envelope_coverage(source, record);
    }
    Ok(result)
}

fn project_item(builder: &mut Builder<'_>, version: &str) {
    builder.name("/name");
    builder.description("/description");
    builder.icon("/image/full", version, "item");
    for (name, path) in [
        ("price_total", "/gold/total"),
        ("price_base", "/gold/base"),
        ("price_sell", "/gold/sell"),
    ] {
        builder.field(name, path, Shape::Number, Some("gold"));
    }
    for (name, path) in [
        ("purchasable", "/gold/purchasable"),
        ("in_store", "/inStore"),
        ("consumed", "/consumed"),
        ("consume_on_full", "/consumeOnFull"),
        ("hide_from_all", "/hideFromAll"),
    ] {
        builder.field(name, path, Shape::Bool, None);
    }
    builder.field("builds_from", "/from", Shape::Ids, None);
    builder.field("builds_into", "/into", Shape::Ids, None);
    builder.field("maps", "/maps", Shape::BoolMap, None);
    builder.field("categories", "/tags", Shape::Strings, None);
    builder.field("max_stacks", "/stacks", Shape::Number, Some("count"));
    builder.field("required_champion", "/requiredChampion", Shape::Text, None);
    builder.field("required_ally", "/requiredAlly", Shape::Text, None);
    builder.field("special_recipe", "/specialRecipe", Shape::Number, None);
    builder.field("depth", "/depth", Shape::Number, None);
    builder.text("summary", "/plaintext");
    builder.stats(&[
        ("FlatHPPoolMod", "health", "points"),
        ("FlatMPPoolMod", "mana", "points"),
        ("FlatPhysicalDamageMod", "attack_damage", "points"),
        ("FlatMagicDamageMod", "ability_power", "points"),
        ("FlatArmorMod", "armor", "points"),
        ("FlatSpellBlockMod", "magic_resistance", "points"),
        ("FlatMovementSpeedMod", "movement_speed", "points"),
        ("PercentMovementSpeedMod", "movement_speed_ratio", "ratio"),
        ("PercentAttackSpeedMod", "attack_speed", "ratio"),
        ("FlatCritChanceMod", "critical_strike_chance", "ratio"),
        ("PercentLifeStealMod", "life_steal", "ratio"),
        ("FlatHPRegenMod", "health_regeneration", "points_per_second"),
        // Champ absent des sources vérifiées : sa période reste indéterminée.
        ("FlatMPRegenMod", "mana_regeneration", ""),
        ("PercentHPPoolMod", "health_ratio", "ratio"),
        ("PercentMPPoolMod", "mana_ratio", "ratio"),
    ]);
    if builder.entry.get("effect").is_some() {
        builder.issue("uninterpreted_effect");
    }
}

fn project_champion(builder: &mut Builder<'_>, version: &str) {
    builder.name("/name");
    builder.description("/blurb");
    builder.icon("/image/full", version, "champion");
    builder.text("title", "/title");
    builder.text("lore", "/lore");
    builder.text("resource", "/partype");
    builder.field("categories", "/tags", Shape::Strings, None);
    builder.stats(&[
        ("hp", "health", "points"),
        ("hpperlevel", "health_per_level", "points_per_level"),
        ("mp", "mana", "points"),
        ("mpperlevel", "mana_per_level", "points_per_level"),
        ("attackdamage", "attack_damage", "points"),
        (
            "attackdamageperlevel",
            "attack_damage_per_level",
            "points_per_level",
        ),
        ("armor", "armor", "points"),
        ("armorperlevel", "armor_per_level", "points_per_level"),
        ("spellblock", "magic_resistance", "points"),
        (
            "spellblockperlevel",
            "magic_resistance_per_level",
            "points_per_level",
        ),
        ("movespeed", "movement_speed", "points"),
        ("attackrange", "attack_range", "game_units"),
        ("attackspeed", "attack_speed", "attacks_per_second"),
        (
            "attackspeedperlevel",
            "attack_speed_per_level",
            "percent_per_level",
        ),
        ("hpregen", "health_regeneration", "points_per_5_seconds"),
        (
            "hpregenperlevel",
            "health_regeneration_per_level",
            "points_per_5_seconds_per_level",
        ),
        ("mpregen", "mana_regeneration", "points_per_5_seconds"),
        (
            "mpregenperlevel",
            "mana_regeneration_per_level",
            "points_per_5_seconds_per_level",
        ),
        ("crit", "critical_strike_chance", "percent"),
        (
            "critperlevel",
            "critical_strike_chance_per_level",
            "percent_per_level",
        ),
    ]);
}

fn common_spell(builder: &mut Builder<'_>, version: &str, group: &str) {
    builder.name("/name");
    builder.description("/description");
    builder.icon("/image/full", version, group);
    builder.text("tooltip", "/tooltip");
    builder.field("cooldown", "/cooldown", Shape::Numbers, Some("seconds"));
    builder.field("range", "/range", Shape::Numbers, Some("game_units"));
    builder.field("cost", "/cost", Shape::Numbers, Some("resource_points"));
    builder.field("max_rank", "/maxrank", Shape::Number, Some("rank"));
    builder.text("resource", "/resource");
    builder.text("cost_type", "/costType");
    for key in ["effect", "vars", "leveltip"] {
        if builder.entry.get(key).is_some() {
            builder.issue(format!("uninterpreted:{key}"));
        }
    }
}

fn project_ability(
    version: &str,
    source: &CatalogSource,
    entry: &Value,
    pointer: String,
    champion: &str,
    slot: &str,
    namespace: &str,
) -> CatalogRecord {
    let mut builder = Builder::new(
        source,
        entry,
        pointer,
        "ability",
        format!("{champion}:{slot}"),
        namespace,
    );
    common_spell(
        &mut builder,
        version,
        if slot == "passive" {
            "passive"
        } else {
            "spell"
        },
    );
    builder.field("technical_id", "/id", Shape::Text, None);
    builder.derived("champion_id", json!(champion), "", None);
    builder.derived("slot", json!(slot), "", None);
    builder.finish()
}

fn project_runes(
    version: &str,
    source: &CatalogSource,
) -> Result<Vec<CatalogRecord>, CatalogError> {
    let styles = source.data.as_array().ok_or(CatalogError::InvalidSource)?;
    let mut result = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, style) in styles.iter().enumerate() {
        let id = positive_id(&style["id"]).ok_or(CatalogError::InvalidSource)?;
        if !ids.insert(id.clone()) {
            return Err(CatalogError::InvalidSource);
        }
        let mut builder = Builder::new(
            source,
            style,
            format!("/{index}"),
            "rune",
            id.clone(),
            "standard",
        );
        builder.name("/name");
        builder.icon("/icon", version, "rune");
        builder.consume("/id");
        builder.field("technical_id", "/key", Shape::Text, None);
        builder.derived("rune_kind", json!("style"), "", None);
        let slots = style["slots"]
            .as_array()
            .ok_or(CatalogError::InvalidSource)?;
        for (slot_index, slot) in slots.iter().enumerate() {
            for (rune_index, rune) in slot["runes"]
                .as_array()
                .ok_or(CatalogError::InvalidSource)?
                .iter()
                .enumerate()
            {
                let rune_id = positive_id(&rune["id"]).ok_or(CatalogError::InvalidSource)?;
                if !ids.insert(rune_id.clone()) {
                    return Err(CatalogError::InvalidSource);
                }
                let mut leaf = Builder::new(
                    source,
                    rune,
                    format!("/{index}/slots/{slot_index}/runes/{rune_index}"),
                    "rune",
                    rune_id,
                    "standard",
                );
                leaf.name("/name");
                leaf.description("/longDesc");
                leaf.text("summary", "/shortDesc");
                leaf.icon("/icon", version, "rune");
                leaf.consume("/id");
                leaf.field("technical_id", "/key", Shape::Text, None);
                leaf.derived("rune_kind", json!("rune"), "", None);
                leaf.derived("slot", json!(slot_index), "", None);
                leaf.record.fields.insert(
                    "style_id".into(),
                    CatalogValue {
                        value: json!(id),
                        unit: None,
                        status: ValueStatus::Derived,
                        sources: vec![ValueSource {
                            source_id: source.id.clone(),
                            pointer: format!("/{index}/id"),
                        }],
                    },
                );
                result.push(leaf.finish());
                builder.delegate(&format!("/slots/{slot_index}/runes/{rune_index}"));
            }
        }
        result.push(builder.finish());
    }
    Ok(result)
}

fn project_catalog(source: &CatalogSource) -> Result<Vec<CatalogRecord>, CatalogError> {
    if source.locale.is_some() {
        return Err(CatalogError::InvalidSource);
    }
    let (kind, id_field, name_field) = match source.key.as_str() {
        "queues" => ("queue", "queueId", "description"),
        "maps" => ("map", "mapId", "mapName"),
        "gameModes" => ("mode", "gameMode", "description"),
        "gameTypes" => ("game_type", "gametype", "description"),
        _ => return Err(CatalogError::InvalidSource),
    };
    let mut records = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, entry) in source
        .data
        .as_array()
        .ok_or(CatalogError::InvalidSource)?
        .iter()
        .enumerate()
    {
        let id = entry[id_field]
            .as_u64()
            .map(|n| n.to_string())
            .or_else(|| {
                entry[id_field]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
            })
            .ok_or(CatalogError::InvalidSource)?;
        if !ids.insert(id.clone()) {
            return Err(CatalogError::InvalidSource);
        }
        let mut builder = Builder::new(source, entry, format!("/{index}"), kind, id, "global");
        builder.consume(&format!("/{id_field}"));
        builder.name(&format!("/{name_field}"));
        builder.description("/description");
        builder.text("notes", "/notes");
        builder.text("map_name", "/map");
        builder.derived("version_scope", json!("unversioned"), "", None);
        records.push(builder.finish());
    }
    Ok(records)
}

type Identity = (String, String, String, String);

/// Revalide les relations sur le catalogue final, y compris les entités enrichies.
pub(super) fn finalize(records: &mut [CatalogRecord]) {
    for record in records.iter_mut() {
        record.coverage.issues.retain(|issue| {
            ![
                "unknown_recipe_reference:",
                "inconsistent_recipe:",
                "recipe_cycle",
                "locale_conflict:",
                "missing_locale:",
            ]
            .iter()
            .any(|prefix| issue.starts_with(prefix))
        });
    }
    check_locales(records);
    check_recipes(records);
    for record in records.iter_mut() {
        record.coverage.issues.sort();
        record.coverage.issues.dedup();
    }
    records.sort_by(|a, b| {
        (&a.kind, &a.namespace, &a.locale, &a.id).cmp(&(&b.kind, &b.namespace, &b.locale, &b.id))
    });
}

fn merge_record(records: &mut BTreeMap<Identity, CatalogRecord>, record: CatalogRecord) {
    let key = (
        record.kind.clone(),
        record.namespace.clone(),
        record.locale.clone(),
        record.id.clone(),
    );
    let Some(previous) = records.get_mut(&key) else {
        records.insert(key, record);
        return;
    };
    for (key, value) in record.fields {
        merge_value(
            &mut previous.fields,
            key,
            value,
            &mut previous.coverage.issues,
        );
    }
    for (key, value) in record.stats {
        merge_value(
            &mut previous.stats,
            key,
            value,
            &mut previous.coverage.issues,
        );
    }
    previous.coverage.source_fields += record.coverage.source_fields;
    previous.coverage.normalized_fields += record.coverage.normalized_fields;
    previous
        .coverage
        .unmapped_fields
        .extend(record.coverage.unmapped_fields);
    previous.coverage.issues.extend(record.coverage.issues);
    if previous.description.is_none() {
        previous.description = record.description;
    }
    if previous.icon.is_none() {
        previous.icon = record.icon;
    }
}

fn merge_value(
    fields: &mut BTreeMap<String, CatalogValue>,
    key: String,
    value: CatalogValue,
    issues: &mut Vec<String>,
) {
    let Some(previous) = fields.get_mut(&key) else {
        fields.insert(key, value);
        return;
    };
    if previous.value == value.value
        && previous.unit == value.unit
        && previous.status == value.status
    {
        for origin in value.sources {
            if !previous.sources.contains(&origin) {
                previous.sources.push(origin);
            }
        }
    } else {
        let old = previous.clone();
        let mut sources = old.sources.clone();
        sources.extend(value.sources.clone());
        *previous = CatalogValue {
            value: json!({"candidates":[old,value]}),
            unit: None,
            status: ValueStatus::Conflict,
            sources,
        };
        issues.push(format!("conflict:{key}"));
    }
}

fn check_locales(records: &mut [CatalogRecord]) {
    let mut available = BTreeMap::<(String, String), BTreeSet<String>>::new();
    for record in records.iter().filter(|r| r.locale != "und") {
        available
            .entry((record.kind.clone(), record.namespace.clone()))
            .or_default()
            .insert(record.locale.clone());
    }
    let lookups: BTreeMap<_, _> = records
        .iter()
        .enumerate()
        .map(|(i, r)| {
            (
                (
                    r.kind.clone(),
                    r.namespace.clone(),
                    r.id.clone(),
                    r.locale.clone(),
                ),
                i,
            )
        })
        .collect();
    let mut additions = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let Some(locales) = available.get(&(record.kind.clone(), record.namespace.clone())) else {
            continue;
        };
        for locale in locales.iter().filter(|l| *l != &record.locale) {
            if let Some(other) = lookups.get(&(
                record.kind.clone(),
                record.namespace.clone(),
                record.id.clone(),
                locale.clone(),
            )) {
                for (key, value) in &record.fields {
                    if value.status != ValueStatus::Descriptive && key != "icon" {
                        if let Some(second) = records[*other].fields.get(key) {
                            if value.value != second.value {
                                additions.push((index, format!("locale_conflict:{key}")));
                            }
                        }
                    }
                }
                for (key, value) in &record.stats {
                    if let Some(second) = records[*other].stats.get(key) {
                        if value.value != second.value {
                            additions.push((index, format!("locale_conflict:stats.{key}")));
                        }
                    }
                }
            } else {
                additions.push((index, format!("missing_locale:{locale}")));
            }
        }
    }
    for (index, issue) in additions {
        records[index].coverage.issues.push(issue);
    }
}

fn ids(record: &CatalogRecord, field: &str) -> Vec<String> {
    record
        .fields
        .get(field)
        .filter(|v| v.status == ValueStatus::Verified)
        .and_then(|v| v.value.as_array())
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn check_recipes(records: &mut [CatalogRecord]) {
    type ItemKey = (String, String, String);
    let lookup: BTreeMap<_, _> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| r.kind == "item")
        .map(|(i, r)| ((r.namespace.clone(), r.locale.clone(), r.id.clone()), i))
        .collect();
    let mut graph = BTreeMap::<ItemKey, BTreeSet<ItemKey>>::new();
    let mut issues = Vec::new();
    for (key, index) in &lookup {
        let record = &records[*index];
        for field in ["builds_from", "builds_into"] {
            for reference in ids(record, field) {
                let other = (
                    record.namespace.clone(),
                    record.locale.clone(),
                    reference.clone(),
                );
                let Some(other_index) = lookup.get(&other) else {
                    issues.push((*index, format!("unknown_recipe_reference:{reference}")));
                    continue;
                };
                let inverse = if field == "builds_from" {
                    "builds_into"
                } else {
                    "builds_from"
                };
                if records[*other_index]
                    .fields
                    .get(inverse)
                    .is_some_and(|v| v.status == ValueStatus::Verified)
                    && !ids(&records[*other_index], inverse).contains(&record.id)
                {
                    issues.push((*index, format!("inconsistent_recipe:{reference}")));
                }
                let (parent, component) = if field == "builds_from" {
                    (key.clone(), other)
                } else {
                    (other, key.clone())
                };
                graph.entry(parent).or_default().insert(component);
            }
        }
    }
    for (key, index) in &lookup {
        let mut visited = BTreeSet::new();
        let mut pending = graph
            .get(key)
            .map(|v| v.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        while let Some(next) = pending.pop() {
            if &next == key {
                issues.push((*index, "recipe_cycle".into()));
                break;
            }
            if visited.insert(next.clone()) {
                if let Some(neighbours) = graph.get(&next) {
                    pending.extend(neighbours.iter().cloned());
                }
            }
        }
    }
    for (index, issue) in issues {
        records[index].coverage.issues.push(issue);
    }
}

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod tests;
