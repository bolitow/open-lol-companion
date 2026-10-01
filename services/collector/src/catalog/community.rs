//! Complément CommunityDragon versionné ; aucun calcul implicite des mécaniques.
use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use super::{
    CatalogEffect, CatalogError, CatalogRecord, CatalogSource, CatalogValue, RecordCoverage,
    ValueSource, ValueStatus,
};

#[path = "community_http.rs"]
mod http;
pub use http::{fetch_sources, fetch_sources_with};
#[path = "community_perks.rs"]
mod perks;
use super::normalize::plain_text;

type ItemIndex<'a> = BTreeMap<String, (usize, &'a Value)>;

fn item_index(data: &Value) -> Result<ItemIndex<'_>, CatalogError> {
    let mut items = BTreeMap::new();
    let list = data
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or(CatalogError::InvalidSource)?;
    for (index, value) in list.iter().enumerate() {
        let id = value
            .get("id")
            .and_then(Value::as_u64)
            .ok_or(CatalogError::InvalidSource)?;
        if value.get("name").and_then(Value::as_str).is_none()
            || items.insert(id.to_string(), (index, value)).is_some()
        {
            return Err(CatalogError::InvalidSource);
        }
    }
    Ok(items)
}

fn validate_sources<'a>(
    version: &str,
    sources: &'a [CatalogSource],
) -> Result<BTreeMap<&'a str, &'a CatalogSource>, CatalogError> {
    let patch = http::patch(version)?;
    let mut indexed = BTreeMap::new();
    for source in sources.iter().filter(|s| s.provider == "cdragon") {
        if indexed.insert(source.key.as_str(), source).is_some() {
            return Err(CatalogError::InvalidSource);
        }
    }
    if indexed.len() != http::RESOURCES.len() {
        return Err(CatalogError::InvalidSource);
    }
    let metadata = indexed
        .get("content-metadata.json")
        .ok_or(CatalogError::InvalidSource)?;
    let build = http::build(patch, &metadata.data)?;
    for (key, locale, path) in http::RESOURCES {
        let source = indexed.get(key).ok_or(CatalogError::InvalidSource)?;
        if source.version != build
            || source.locale.as_deref() != locale
            || source.url != http::url(patch, path)
        {
            return Err(CatalogError::InvalidSource);
        }
    }
    let fr = item_index(&indexed["fr_FR/items.json"].data)?;
    let en = item_index(&indexed["en_US/items.json"].data)?;
    if fr.keys().ne(en.keys()) {
        return Err(CatalogError::InvalidSource);
    }
    let bin = indexed["items.bin"]
        .data
        .as_object()
        .filter(|v| !v.is_empty())
        .ok_or(CatalogError::InvalidSource)?;
    let mut bin_ids = BTreeSet::new();
    for (key, value) in bin {
        if let Some(id) = key
            .strip_prefix("Items/")
            .filter(|id| id.bytes().all(|b| b.is_ascii_digit()) && !id.is_empty())
        {
            bin_ids.insert(id.to_owned());
            if value
                .get("itemID")
                .and_then(Value::as_u64)
                .map(|v| v.to_string())
                .as_deref()
                != Some(id)
                || value.get("__type").and_then(Value::as_str) != Some("ItemData")
            {
                return Err(CatalogError::InvalidSource);
            }
        }
    }
    if !bin_ids.iter().eq(fr.keys()) {
        return Err(CatalogError::InvalidSource);
    }
    perks::validate(&indexed)?;
    Ok(indexed)
}

fn escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn observed(
    source: &CatalogSource,
    pointer: String,
    value: Value,
    unit: Option<&str>,
    status: ValueStatus,
) -> CatalogValue {
    CatalogValue {
        value,
        unit: unit.map(str::to_owned),
        status,
        sources: vec![ValueSource {
            source_id: source.id.clone(),
            pointer,
        }],
    }
}

fn numeric(source: &CatalogSource, pointer: String, value: &Value, unit: &str) -> CatalogValue {
    let status = if value.is_null() {
        ValueStatus::Missing
    } else if value.is_number() {
        ValueStatus::Verified
    } else {
        ValueStatus::Unsupported
    };
    observed(source, pointer, value.clone(), Some(unit), status)
}

fn equivalent(a: &Value, b: &Value) -> bool {
    if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
        // Les exports BIN portent des floats 32 bits ; la tolérance évite un faux conflit 0,3/0,30000001.
        return (a - b).abs() <= 1e-7 * a.abs().max(b.abs()).max(1.0);
    }
    a == b
}

fn equivalent_field(name: &str, a: &Value, b: &Value) -> bool {
    if matches!(name, "builds_from" | "builds_into" | "categories") {
        if let (Some(a), Some(b)) = (a.as_array(), b.as_array()) {
            if let (Some(mut a), Some(mut b)) = (
                a.iter().map(Value::as_str).collect::<Option<Vec<_>>>(),
                b.iter().map(Value::as_str).collect::<Option<Vec<_>>>(),
            ) {
                // L'ordre ne porte pas de sens, mais deux composants identiques comptent deux fois.
                a.sort_unstable();
                b.sort_unstable();
                return a == b;
            }
        }
    }
    equivalent(a, b)
}

fn merge(target: &mut BTreeMap<String, CatalogValue>, name: &str, incoming: CatalogValue) -> bool {
    let Some(previous) = target.get_mut(name) else {
        target.insert(name.into(), incoming);
        return false;
    };
    if previous.unit == incoming.unit
        && previous.status == incoming.status
        && equivalent_field(name, &previous.value, &incoming.value)
    {
        for source in incoming.sources {
            if !previous.sources.contains(&source) {
                previous.sources.push(source);
            }
        }
        return false;
    }
    let mut candidates = if previous.status == ValueStatus::Conflict {
        previous
            .value
            .get("candidates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    } else {
        vec![json!(previous)]
    };
    candidates.push(json!(incoming));
    previous.sources.extend(incoming.sources);
    previous.value = json!({"candidates": candidates});
    previous.status = ValueStatus::Conflict;
    previous.unit = None;
    true
}

fn leaves(value: &Value, pointer: &str, paths: &mut Vec<String>) {
    match value {
        Value::Object(fields) if !fields.is_empty() => {
            for (key, value) in fields {
                leaves(value, &format!("{pointer}/{}", escape(key)), paths);
            }
        }
        Value::Array(values) if !values.is_empty() => {
            for (index, value) in values.iter().enumerate() {
                leaves(value, &format!("{pointer}/{index}"), paths);
            }
        }
        _ => paths.push(pointer.into()),
    }
}

fn coverage(
    record: &mut CatalogRecord,
    data: &Value,
    pointer: &str,
    mapped: &BTreeSet<String>,
    source: &CatalogSource,
) {
    let mut paths = vec![];
    leaves(data, pointer, &mut paths);
    record.coverage.source_fields += paths.len() as u32;
    for path in paths {
        if mapped
            .iter()
            .any(|p| path == *p || path.strip_prefix(p).is_some_and(|s| s.starts_with('/')))
        {
            record.coverage.normalized_fields += 1;
        } else {
            record
                .coverage
                .unmapped_fields
                .push(format!("{}:{path}", source.id));
        }
    }
}

fn ids(value: &Value, prefix: &str) -> Option<Value> {
    value
        .as_array()?
        .iter()
        .map(|v| {
            if prefix.is_empty() {
                v.as_u64().map(|id| Value::String(id.to_string()))
            } else {
                v.as_str()?
                    .strip_prefix(prefix)
                    .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
                    .map(|s| Value::String(s.into()))
            }
        })
        .collect::<Option<Vec<_>>>()
        .map(Value::Array)
}

fn add_field(record: &mut CatalogRecord, name: &str, value: CatalogValue) {
    let status = value.status.clone();
    if merge(&mut record.fields, name, value) {
        record
            .coverage
            .issues
            .push(format!("conflict:fields.{name}"));
    } else if status == ValueStatus::Unsupported {
        record
            .coverage
            .issues
            .push(format!("unsupported:fields.{name}"));
    }
}

fn identity(record: &mut CatalogRecord, source: &CatalogSource, pointer: &str, data: &Value) {
    add_field(
        record,
        "community_name",
        observed(
            source,
            format!("{pointer}/name"),
            json!(plain_text(data["name"].as_str().unwrap_or_default())),
            None,
            ValueStatus::Descriptive,
        ),
    );
    add_field(
        record,
        "community_id",
        observed(
            source,
            format!("{pointer}/id"),
            json!(record.id),
            None,
            ValueStatus::Derived,
        ),
    );
}

fn basic_item(record: &mut CatalogRecord, source: &CatalogSource, index: usize, item: &Value) {
    let pointer = format!("/{index}");
    identity(record, source, &pointer, item);
    if item["name"].as_str().is_some_and(str::is_empty) {
        record.coverage.issues.push("missing:community_name".into());
    }
    let mut mapped: BTreeSet<_> = ["id", "name"]
        .map(|k| format!("{pointer}/{k}"))
        .into_iter()
        .collect();
    for (key, name, unit) in [
        ("price", "price_base", Some("gold")),
        ("priceTotal", "price_total", Some("gold")),
        ("active", "active", None),
        ("inStore", "in_store", None),
        ("categories", "categories", None),
        ("maxStacks", "max_stacks", Some("count")),
        ("requiredChampion", "required_champion", None),
        ("requiredAlly", "required_ally", None),
        (
            "requiredBuffCurrencyName",
            "required_buff_currency_name",
            None,
        ),
        (
            "requiredBuffCurrencyCost",
            "required_buff_currency_cost",
            None,
        ),
        ("specialRecipe", "special_recipe", None),
        ("isEnchantment", "is_enchantment", None),
        ("displayInItemSets", "display_in_item_sets", None),
        ("from", "builds_from", None),
        ("to", "builds_into", None),
    ] {
        let Some(raw) = item.get(key) else {
            continue;
        };
        let value = if matches!(key, "from" | "to") {
            ids(raw, "")
        } else {
            Some(raw.clone())
        };
        let valid = match key {
            "price" | "priceTotal" | "maxStacks" | "requiredBuffCurrencyCost" | "specialRecipe" => {
                raw.is_number()
            }
            "active" | "inStore" | "isEnchantment" | "displayInItemSets" => raw.is_boolean(),
            "categories" => raw
                .as_array()
                .is_some_and(|v| v.iter().all(Value::is_string)),
            "requiredChampion" | "requiredAlly" | "requiredBuffCurrencyName" => raw.is_string(),
            _ => value.is_some(),
        };
        let status = if raw.is_null() {
            ValueStatus::Missing
        } else if valid {
            ValueStatus::Verified
        } else {
            ValueStatus::Unsupported
        };
        if valid {
            mapped.insert(format!("{pointer}/{key}"));
        }
        add_field(
            record,
            name,
            observed(
                source,
                format!("{pointer}/{key}"),
                value.unwrap_or_else(|| raw.clone()),
                unit,
                status,
            ),
        );
    }
    if let Some(description) = item.get("description").and_then(Value::as_str) {
        let text = plain_text(description);
        if record.description.is_none() {
            record.description = Some(text.clone());
        }
        if text.contains('@') || text.contains("{{") {
            record
                .coverage
                .issues
                .push("unresolved_placeholder:description".into());
        }
        add_field(
            record,
            "community_description",
            observed(
                source,
                format!("{pointer}/description"),
                json!(text),
                None,
                ValueStatus::Descriptive,
            ),
        );
        mapped.insert(format!("{pointer}/description"));
    }
    if let Some(icon) = item.get("iconPath").and_then(Value::as_str) {
        // Le chemin reste une référence source, jamais une URL externe exécutée par l'interface.
        add_field(
            record,
            "community_icon_path",
            observed(
                source,
                format!("{pointer}/iconPath"),
                json!(icon),
                None,
                ValueStatus::Verified,
            ),
        );
        mapped.insert(format!("{pointer}/iconPath"));
    }
    coverage(record, item, &pointer, &mapped, source);
}

// Clés vérifiées dans l'export 16.19 ; absence d'une clé != valeur nulle ou zéro.
const STATS: [(&str, &str, &str); 24] = [
    ("mAbilityHasteMod", "ability_haste", "points"),
    ("mFlatHPPoolMod", "health", "points"),
    ("flatMPPoolMod", "mana", "points"),
    ("mFlatPhysicalDamageMod", "attack_damage", "points"),
    ("mFlatMagicDamageMod", "ability_power", "points"),
    ("mFlatArmorMod", "armor", "points"),
    ("mFlatSpellBlockMod", "magic_resistance", "points"),
    ("mPercentAttackSpeedMod", "attack_speed", "ratio"),
    ("mFlatMovementSpeedMod", "movement_speed", "points"),
    ("mPercentMovementSpeedMod", "movement_speed_ratio", "ratio"),
    ("mFlatCritChanceMod", "critical_strike_chance", "ratio"),
    ("mPercentLifeStealMod", "life_steal", "ratio"),
    ("mPercentBaseHPRegenMod", "health_regen_percent", "ratio"),
    ("percentBaseMPRegenMod", "mana_regen_percent", "ratio"),
    (
        "mFlatArmorPenetrationMod",
        "flat_armor_penetration",
        "points",
    ),
    (
        "mPercentArmorPenetrationMod",
        "armor_penetration_percent",
        "ratio",
    ),
    (
        "mFlatMagicPenetrationMod",
        "flat_magic_penetration",
        "points",
    ),
    (
        "mPercentMagicPenetrationMod",
        "magic_penetration_percent",
        "ratio",
    ),
    ("PercentOmnivampMod", "omnivamp", "ratio"),
    ("mPercentTenacityItemMod", "tenacity", "ratio"),
    ("PhysicalLethality", "lethality", "points"),
    ("mPercentHealingAmountMod", "heal_and_shield_power", "ratio"),
    ("mPercentSlowResistMod", "slow_resistance", "ratio"),
    ("mFlatAttackRangeMod", "attack_range", "points"),
];

fn bin_item(record: &mut CatalogRecord, source: &CatalogSource, item: &Value) {
    let pointer = format!("/Items~1{}", record.id);
    let mut mapped: BTreeSet<_> = ["itemID", "__type"]
        .map(|k| format!("{pointer}/{k}"))
        .into_iter()
        .collect();
    for (key, name, unit) in STATS {
        let Some(raw) = item.get(key) else {
            continue;
        };
        let value = numeric(source, format!("{pointer}/{key}"), raw, unit);
        if value.status == ValueStatus::Verified {
            mapped.insert(format!("{pointer}/{key}"));
        }
        if merge(&mut record.stats, name, value) {
            record
                .coverage
                .issues
                .push(format!("conflict:stats.{name}"));
        }
    }
    for (key, name, unit) in [
        ("price", "price_base", Some("gold")),
        ("maxStack", "max_stacks", Some("count")),
        ("mCanBeSold", "sellable", None),
    ] {
        if let Some(raw) = item.get(key) {
            let valid = if key == "mCanBeSold" {
                raw.is_boolean()
            } else {
                raw.is_number()
            };
            let status = if raw.is_null() {
                ValueStatus::Missing
            } else if valid {
                ValueStatus::Verified
            } else {
                ValueStatus::Unsupported
            };
            if valid {
                mapped.insert(format!("{pointer}/{key}"));
            }
            add_field(
                record,
                name,
                observed(
                    source,
                    format!("{pointer}/{key}"),
                    raw.clone(),
                    unit,
                    status,
                ),
            );
        }
    }
    for (key, name) in [
        ("consumed", "consumed"),
        ("consumeOnAcquire", "consume_on_acquire"),
        ("mRequiredLevel", "required_level"),
        ("mRequiredChampion", "required_champion"),
        ("mRequiredSpellName", "required_spell_name"),
        ("RestrictedBuffName", "restricted_buff_name"),
        ("sellBackModifier", "sell_back_ratio"),
    ] {
        if let Some(raw) = item.get(key) {
            let valid = match key {
                "consumed" | "consumeOnAcquire" => raw.is_boolean(),
                "mRequiredLevel" | "sellBackModifier" => raw.is_number(),
                _ => raw.is_string(),
            };
            let status = if raw.is_null() {
                ValueStatus::Missing
            } else if valid {
                ValueStatus::Verified
            } else {
                ValueStatus::Unsupported
            };
            if valid {
                mapped.insert(format!("{pointer}/{key}"));
            }
            add_field(
                record,
                name,
                observed(
                    source,
                    format!("{pointer}/{key}"),
                    raw.clone(),
                    if key == "sellBackModifier" {
                        Some("ratio")
                    } else {
                        None
                    },
                    status,
                ),
            );
        }
    }
    if let Some(raw) = item.pointer("/mItemDataAvailability/mInStore") {
        let status = if raw.is_null() {
            ValueStatus::Missing
        } else if raw.is_boolean() {
            ValueStatus::Verified
        } else {
            ValueStatus::Unsupported
        };
        if raw.is_boolean() {
            mapped.insert(format!("{pointer}/mItemDataAvailability/mInStore"));
        }
        add_field(
            record,
            "in_store",
            observed(
                source,
                format!("{pointer}/mItemDataAvailability/mInStore"),
                raw.clone(),
                None,
                status,
            ),
        );
    }
    if let Some(raw) = item.get("DataValuesModeOverride") {
        add_field(
            record,
            "mode_parameter_overrides",
            observed(
                source,
                format!("{pointer}/DataValuesModeOverride"),
                raw.clone(),
                None,
                ValueStatus::Unsupported,
            ),
        );
    }
    if let Some(raw) = item.get("recipeItemLinks") {
        let value = ids(raw, "Items/");
        let status = if raw.is_null() {
            ValueStatus::Missing
        } else if value.is_some() {
            ValueStatus::Verified
        } else {
            ValueStatus::Unsupported
        };
        if value.is_some() {
            mapped.insert(format!("{pointer}/recipeItemLinks"));
        }
        add_field(
            record,
            "builds_from",
            observed(
                source,
                format!("{pointer}/recipeItemLinks"),
                value.unwrap_or_else(|| raw.clone()),
                None,
                status,
            ),
        );
    }
    let mut parameters = BTreeMap::new();
    if let Some(values) = item.get("mDataValues").and_then(Value::as_array) {
        for (index, parameter) in values.iter().enumerate() {
            let (Some(name), Some(value)) = (
                parameter.get("mName").and_then(Value::as_str),
                parameter.get("mValue"),
            ) else {
                continue;
            };
            let prefix = format!("{pointer}/mDataValues/{index}");
            let status = if value.is_null() {
                ValueStatus::Missing
            } else if value.is_number() {
                ValueStatus::Verified
            } else {
                ValueStatus::Unsupported
            };
            if status == ValueStatus::Verified {
                mapped.insert(format!("{prefix}/mName"));
                mapped.insert(format!("{prefix}/mValue"));
                if parameter.get("__type").and_then(Value::as_str) == Some("ItemDataValue") {
                    mapped.insert(format!("{prefix}/__type"));
                }
            }
            if merge(
                &mut parameters,
                name,
                observed(
                    source,
                    format!("{prefix}/mValue"),
                    value.clone(),
                    None,
                    status,
                ),
            ) {
                record
                    .coverage
                    .issues
                    .push(format!("conflict:effect_parameter.{name}"));
            }
        }
    }
    if !parameters.is_empty() {
        record.effects.push(CatalogEffect {
            id: "cdragon_parameters".into(),
            description: None,
            parameters,
            calculation: None,
        });
    }
    if let Some(calculations) = item.get("mItemCalculations").and_then(Value::as_object) {
        for (name, calculation) in calculations {
            record.effects.push(CatalogEffect {
                id: format!("cdragon:{name}"),
                description: None,
                parameters: BTreeMap::new(),
                calculation: Some(observed(
                    source,
                    format!("{pointer}/mItemCalculations/{}", escape(name)),
                    calculation.clone(),
                    None,
                    ValueStatus::Unsupported,
                )),
            });
            record
                .coverage
                .issues
                .push(format!("unsupported:calculation.{name}"));
        }
    }
    coverage(record, item, &pointer, &mapped, source);
}

/// Rapproche les sources validées par identité et conserve les valeurs contradictoires.
pub fn enrich(
    version: &str,
    records: &mut Vec<CatalogRecord>,
    sources: &[CatalogSource],
) -> Result<(), CatalogError> {
    let sources = validate_sources(version, sources)?;
    let bin = sources["items.bin"]
        .data
        .as_object()
        .ok_or(CatalogError::InvalidSource)?;
    for locale in ["fr_FR", "en_US"] {
        let source = sources[format!("{locale}/items.json").as_str()];
        let items = item_index(&source.data)?;
        for (id, (index, item)) in items {
            let record_index = records.iter().position(|r| {
                r.kind == "item" && r.namespace == "standard" && r.locale == locale && r.id == id
            });
            let record_index = record_index.unwrap_or_else(|| {
                records.push(CatalogRecord {
                    kind: "item".into(),
                    id: id.clone(),
                    namespace: "standard".into(),
                    locale: locale.into(),
                    name: plain_text(
                        item["name"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&id),
                    ),
                    description: None,
                    icon: None,
                    fields: BTreeMap::new(),
                    stats: BTreeMap::new(),
                    effects: vec![],
                    coverage: RecordCoverage::default(),
                });
                records.len() - 1
            });
            let record = &mut records[record_index];
            basic_item(record, source, index, item);
            if let Some(item) = bin.get(&format!("Items/{id}")) {
                bin_item(record, sources["items.bin"], item);
            } else {
                record
                    .coverage
                    .issues
                    .push("missing:cdragon_item_bin".into());
            }
        }
    }
    perks::enrich(version, records, &sources);
    Ok(())
}

#[cfg(test)]
#[path = "community_tests.rs"]
mod tests;
