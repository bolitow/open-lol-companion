//! Fragments identifiés par les emplacements kStatMod et les références StatMods.
use super::*;

type Sources<'a> = BTreeMap<&'a str, &'a CatalogSource>;
type SlotIdentity = (u64, usize, String, Vec<u64>);

fn slot_identity(data: &Value) -> Result<Vec<SlotIdentity>, CatalogError> {
    let styles = data
        .get("styles")
        .and_then(Value::as_array)
        .filter(|s| !s.is_empty())
        .ok_or(CatalogError::InvalidSource)?;
    let mut identities = vec![];
    let mut seen = BTreeSet::new();
    for style in styles {
        let id = style
            .get("id")
            .and_then(Value::as_u64)
            .ok_or(CatalogError::InvalidSource)?;
        if !seen.insert(id) {
            return Err(CatalogError::InvalidSource);
        }
        let slots = style
            .get("slots")
            .and_then(Value::as_array)
            .ok_or(CatalogError::InvalidSource)?;
        for (index, slot) in slots.iter().enumerate() {
            let kind = slot
                .get("type")
                .and_then(Value::as_str)
                .ok_or(CatalogError::InvalidSource)?;
            let perks = slot
                .get("perks")
                .and_then(Value::as_array)
                .ok_or(CatalogError::InvalidSource)?
                .iter()
                .map(|p| p.as_u64().ok_or(CatalogError::InvalidSource))
                .collect::<Result<Vec<_>, _>>()?;
            identities.push((id, index, kind.into(), perks));
        }
    }
    Ok(identities)
}

pub(super) fn validate(sources: &Sources<'_>) -> Result<(), CatalogError> {
    let fr = item_index(&sources["fr_FR/perks.json"].data)?;
    let en = item_index(&sources["en_US/perks.json"].data)?;
    if fr.keys().ne(en.keys()) {
        return Err(CatalogError::InvalidSource);
    }
    let fr_slots = slot_identity(&sources["fr_FR/perkstyles.json"].data)?;
    let en_slots = slot_identity(&sources["en_US/perkstyles.json"].data)?;
    if fr_slots != en_slots
        || fr_slots
            .iter()
            .any(|(_, _, _, ids)| ids.iter().any(|id| !fr.contains_key(&id.to_string())))
    {
        return Err(CatalogError::InvalidSource);
    }
    Ok(())
}

fn icon_url(version: &str, path: &str) -> Option<String> {
    let path = path.strip_prefix("/lol-game-data/assets/")?;
    if !path
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"/_.-".contains(&b))
        || path.split('/').any(|part| part == ".." || part == ".")
    {
        return None;
    }
    Some(http::url(
        http::patch(version).ok()?,
        &format!(
            "plugins/rcp-be-lol-game-data/global/default/{}",
            path.to_lowercase()
        ),
    ))
}

pub(super) fn enrich(version: &str, records: &mut Vec<CatalogRecord>, sources: &Sources<'_>) {
    for locale in ["fr_FR", "en_US"] {
        let source = sources[format!("{locale}/perks.json").as_str()];
        let styles = sources[format!("{locale}/perkstyles.json").as_str()];
        // Les structures et identités ont été validées avant toute mutation.
        let identities = slot_identity(&styles.data).unwrap_or_default();
        let style_shards: BTreeSet<_> = identities
            .iter()
            .filter(|(_, _, kind, _)| kind == "kStatMod")
            .flat_map(|(_, _, _, ids)| ids.iter().copied())
            .collect();
        let listed: BTreeSet<_> = identities
            .iter()
            .flat_map(|(_, _, _, ids)| ids.iter().copied())
            .collect();
        for (index, perk) in source.data.as_array().into_iter().flatten().enumerate() {
            let id = perk["id"].as_u64().unwrap_or_default();
            let icon = perk.get("iconPath").and_then(Value::as_str);
            let shard = style_shards.contains(&id)
                || icon.is_some_and(|p| {
                    p.starts_with("/lol-game-data/assets/v1/perk-images/StatMods/")
                });
            let kind = if shard { "rune_shard" } else { "rune" };
            let id_string = id.to_string();
            let pos = records.iter().position(|r| {
                r.kind == kind
                    && r.id == id_string
                    && r.locale == locale
                    && r.namespace == "standard"
            });
            let pos = pos.unwrap_or_else(|| {
                records.push(CatalogRecord {
                    kind: kind.into(),
                    id: id_string,
                    namespace: "standard".into(),
                    locale: locale.into(),
                    name: plain_text(perk["name"].as_str().unwrap_or_default()),
                    description: None,
                    icon: None,
                    fields: BTreeMap::new(),
                    stats: BTreeMap::new(),
                    effects: vec![],
                    coverage: RecordCoverage::default(),
                });
                records.len() - 1
            });
            let record = &mut records[pos];
            let pointer = format!("/{index}");
            identity(record, source, &pointer, perk);
            let mut mapped: BTreeSet<_> = ["id", "name"]
                .map(|k| format!("{pointer}/{k}"))
                .into_iter()
                .collect();
            if let Some(path) = icon {
                if record.icon.is_none() {
                    record.icon = icon_url(version, path);
                }
                add_field(
                    record,
                    "community_icon_path",
                    observed(
                        source,
                        format!("{pointer}/iconPath"),
                        json!(path),
                        None,
                        ValueStatus::Verified,
                    ),
                );
                mapped.insert(format!("{pointer}/iconPath"));
            }
            for (key, name) in [
                ("longDesc", "community_description"),
                ("shortDesc", "community_short_description"),
                ("tooltip", "tooltip"),
            ] {
                if let Some(raw) = perk.get(key).and_then(Value::as_str) {
                    let text = plain_text(raw);
                    if key == "longDesc" && record.description.is_none() {
                        record.description = Some(text.clone());
                    }
                    if text.contains('@') || text.contains("{{") {
                        record
                            .coverage
                            .issues
                            .push(format!("unresolved_placeholder:{key}"));
                    }
                    add_field(
                        record,
                        name,
                        observed(
                            source,
                            format!("{pointer}/{key}"),
                            json!(text),
                            None,
                            ValueStatus::Descriptive,
                        ),
                    );
                    mapped.insert(format!("{pointer}/{key}"));
                }
            }
            add_field(
                record,
                "listed_in_perk_styles",
                observed(
                    styles,
                    "/styles".into(),
                    json!(listed.contains(&id)),
                    None,
                    ValueStatus::Derived,
                ),
            );
            let slots: Vec<_> = identities.iter().filter(|(_,_,_,ids)| ids.contains(&id)).map(|(style,index,kind,_)|json!({"style_id":style.to_string(),"slot_index":index,"slot_type":kind})).collect();
            add_field(
                record,
                "rune_page_slots",
                observed(
                    styles,
                    "/styles".into(),
                    json!(slots),
                    None,
                    ValueStatus::Derived,
                ),
            );
            if shard {
                record
                    .coverage
                    .issues
                    .push("missing:structured_effect_parameters".into());
            }
            coverage(record, perk, &pointer, &mapped, source);
        }
    }
}
