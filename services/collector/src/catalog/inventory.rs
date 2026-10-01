//! Inventaire de toutes les branches sources, sans assimiler provenance et interprétation.
use super::{CatalogRecord, CatalogSource, RawBranch, SourceInventory};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Compte toutes les feuilles, y compris les ressources auxiliaires jamais projetées.
/// Une branche reliée possède une provenance ; ce lien ne prouve aucune interprétation.
pub fn inventory(sources: &[CatalogSource], records: &[CatalogRecord]) -> Vec<SourceInventory> {
    let mut links: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for record in records {
        let values = record.fields.values().chain(record.stats.values()).chain(
            record
                .effects
                .iter()
                .flat_map(|effect| effect.parameters.values().chain(effect.calculation.iter())),
        );
        for value in values {
            for origin in &value.sources {
                if !origin.pointer.is_empty() {
                    links
                        .entry(&origin.source_id)
                        .or_default()
                        .insert(&origin.pointer);
                }
            }
        }
    }
    let mut result = Vec::with_capacity(sources.len());
    for source in sources {
        let references: BTreeSet<&str> = links
            .get(source.id.as_str())
            .into_iter()
            .flat_map(|values| values.iter().copied())
            .filter(|pointer| source.data.pointer(pointer).is_some())
            .collect();
        let mut branches = branches(source);
        branches.sort_by(|a, b| a.0.cmp(&b.0));
        let mut entry = SourceInventory {
            source_id: source.id.clone(),
            total_leaf_fields: 0,
            branches: branches.len() as u64,
            linked_branches: 0,
            raw_branches: Vec::new(),
        };
        for (pointer, value) in branches {
            let leaf_fields = leaves(value);
            entry.total_leaf_fields += leaf_fields;
            if linked(&pointer, &references) {
                entry.linked_branches += 1;
            } else {
                let reason = reason(source, &pointer, value);
                entry.raw_branches.push(RawBranch {
                    pointer,
                    leaf_fields,
                    reason,
                });
            }
        }
        result.push(entry);
    }
    result.sort_by(|a, b| a.source_id.cmp(&b.source_id));
    result
}

fn escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn branches(source: &CatalogSource) -> Vec<(String, &Value)> {
    match &source.data {
        Value::Object(object) if !object.is_empty() => {
            let mut result = Vec::new();
            for (key, value) in object {
                if source.provider == "ddragon" && key == "data" {
                    if let Some(entries) = value.as_object().filter(|entries| !entries.is_empty()) {
                        result.extend(
                            entries
                                .iter()
                                .map(|(id, value)| (format!("/data/{}", escape(id)), value)),
                        );
                        continue;
                    }
                }
                result.push((format!("/{}", escape(key)), value));
            }
            result
        }
        Value::Array(values) if !values.is_empty() => values
            .iter()
            .enumerate()
            .map(|(index, value)| (format!("/{index}"), value))
            .collect(),
        _ => vec![(String::new(), &source.data)],
    }
}

fn leaves(value: &Value) -> u64 {
    match value {
        Value::Object(values) if !values.is_empty() => values.values().map(leaves).sum(),
        Value::Array(values) if !values.is_empty() => values.iter().map(leaves).sum(),
        _ => 1,
    }
}

fn linked(pointer: &str, references: &BTreeSet<&str>) -> bool {
    if references.contains(pointer) {
        return true;
    }
    // La recherche par préfixe évite branches × liens sur les gros exports BIN.
    let prefix = format!("{pointer}/");
    if references
        .range(prefix.as_str()..)
        .next()
        .is_some_and(|candidate| candidate.starts_with(&prefix))
    {
        return true;
    }
    let mut parent = pointer;
    while let Some((ancestor, _)) = parent.rsplit_once('/') {
        if ancestor.is_empty() {
            break;
        }
        if references.contains(ancestor) {
            return true;
        }
        parent = ancestor;
    }
    false
}

fn reason(source: &CatalogSource, pointer: &str, value: &Value) -> String {
    if source.key.ends_with("content-metadata.json")
        || (source.provider == "ddragon"
            && source.data.get("data").is_some()
            && pointer != "/data"
            && !pointer.starts_with("/data/"))
    {
        return "metadata_only".into();
    }
    let bin = source.key == "items.bin" || source.key.ends_with("items.cdtb.bin.json");
    let item_id = pointer
        .strip_prefix("/Items~1")
        .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()));
    let auxiliary = (bin && !item_id) || source.key.ends_with("perkstyles.json");
    let reason = if auxiliary {
        "resource_auxiliary"
    } else {
        "non_projected"
    };
    match value.get("__type").and_then(Value::as_str).filter(|kind| {
        !kind.is_empty()
            && kind.len() <= 128
            && kind.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    }) {
        Some(kind) => format!("{reason}:{kind}"),
        None => reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{
        CatalogEffect, CatalogValue, RawBranch, RecordCoverage, ValueSource, ValueStatus,
    };
    use serde_json::{json, Value};
    use std::collections::BTreeMap;

    fn source(id: &str, provider: &str, key: &str, data: Value) -> CatalogSource {
        CatalogSource {
            id: id.into(),
            provider: provider.into(),
            key: key.into(),
            version: "16.19.1".into(),
            locale: None,
            url: "https://example.test/public.json".into(),
            observed_at: "2026-10-01T00:00:00Z".into(),
            data,
        }
    }
    fn record(source_id: &str, pointer: &str) -> CatalogRecord {
        CatalogRecord {
            kind: "item".into(),
            id: "1001".into(),
            namespace: "standard".into(),
            locale: "en_US".into(),
            name: "Item".into(),
            description: None,
            icon: None,
            fields: BTreeMap::from([(
                "name".into(),
                CatalogValue {
                    value: json!("Item"),
                    unit: None,
                    status: ValueStatus::Descriptive,
                    sources: vec![ValueSource {
                        source_id: source_id.into(),
                        pointer: pointer.into(),
                    }],
                },
            )]),
            stats: BTreeMap::new(),
            effects: vec![],
            coverage: RecordCoverage::default(),
        }
    }
    #[test]
    fn chaque_branche_ddragon_et_cle_inconnue_est_comptee_une_seule_fois() {
        let source = source(
            "dd",
            "ddragon",
            "en_US/item.json",
            json!({"type":"item","version":"16.19.1","basic":{},"data":{"1001":{"name":"Item","future":{"values":[0,false,null]}},"1002":{"name":"Unknown","empty":[]}}}),
        );
        let result = inventory(&[source], &[record("dd", "/data/1001/name")]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].total_leaf_fields, 9);
        assert_eq!(result[0].branches, 5);
        assert_eq!(result[0].linked_branches, 1);
        assert_eq!(
            result[0].raw_branches,
            vec![
                RawBranch {
                    pointer: "/basic".into(),
                    leaf_fields: 1,
                    reason: "metadata_only".into()
                },
                RawBranch {
                    pointer: "/data/1002".into(),
                    leaf_fields: 2,
                    reason: "non_projected".into()
                },
                RawBranch {
                    pointer: "/type".into(),
                    leaf_fields: 1,
                    reason: "metadata_only".into()
                },
                RawBranch {
                    pointer: "/version".into(),
                    leaf_fields: 1,
                    reason: "metadata_only".into()
                },
            ]
        );
    }
    #[test]
    fn les_ressources_bin_auxiliaires_et_metadonnees_ne_disparaissent_pas() {
        let bin = source(
            "bin",
            "cdragon",
            "items.cdtb.bin.json",
            json!({"Items/1001":{"__type":"ItemData","mFlatHPPoolMod":100},"Spells/aux":{"__type":"SpellObject","mSpell":{"future":[1,2,3]}}}),
        );
        let metadata = source(
            "meta",
            "cdragon",
            "content-metadata.json",
            json!({"version":"16.19","build":{"revision":12}}),
        );
        let result = inventory(
            &[metadata, bin],
            &[record("bin", "/Items~11001/mFlatHPPoolMod")],
        );
        assert_eq!(result[0].source_id, "bin");
        assert_eq!(result[0].total_leaf_fields, 6);
        assert_eq!(
            result[0].raw_branches,
            vec![RawBranch {
                pointer: "/Spells~1aux".into(),
                leaf_fields: 4,
                reason: "resource_auxiliary:SpellObject".into()
            }]
        );
        assert_eq!(result[1].total_leaf_fields, 2);
        assert!(result[1]
            .raw_branches
            .iter()
            .all(|branch| branch.reason == "metadata_only"));
    }
    #[test]
    fn un_chemin_items_non_numerique_est_une_ressource_auxiliaire() {
        let source = source(
            "bin",
            "cdragon",
            "items.bin",
            json!({"Items/Spells/TrinketSweeperLvl3":{"__type":"SpellObject","future":null}}),
        );
        let result = inventory(&[source], &[]);
        assert_eq!(
            result[0].raw_branches[0].reason,
            "resource_auxiliary:SpellObject"
        );
        assert_eq!(result[0].total_leaf_fields, 2);
    }

    #[test]
    fn effets_parents_et_limites_de_pointeurs_sont_respectes() {
        let source = source(
            "array",
            "cdragon",
            "perks.json",
            json!([{"nested":{"value":1}},{"value":2},null,null,null,null,null,null,null,null,{"value":10}]),
        );
        let mut record = record("array", "/1/value");
        record.effects.push(CatalogEffect {
            id: "effect".into(),
            description: None,
            parameters: BTreeMap::new(),
            calculation: Some(CatalogValue {
                value: json!(1),
                unit: None,
                status: ValueStatus::Unsupported,
                sources: vec![ValueSource {
                    source_id: "array".into(),
                    pointer: "/0/nested".into(),
                }],
            }),
        });
        // Un lien racine vide ne permet pas de déclarer toutes les entrées reliées.
        record.stats.insert(
            "unknown".into(),
            CatalogValue {
                value: Value::Null,
                unit: None,
                status: ValueStatus::Missing,
                sources: vec![ValueSource {
                    source_id: "array".into(),
                    pointer: "".into(),
                }],
            },
        );
        let result = inventory(&[source], &[record]);
        assert_eq!(result[0].branches, 11);
        assert_eq!(result[0].linked_branches, 2);
        assert!(result[0]
            .raw_branches
            .iter()
            .any(|branch| branch.pointer == "/10"));
        assert_eq!(result[0].total_leaf_fields, 11);
    }
    #[test]
    fn les_pointeurs_parents_non_vides_lient_les_branches_et_les_ids_sont_echappes() {
        let source = source(
            "dd",
            "ddragon",
            "en_US/item.json",
            json!({"type":"item","data":{"a/b":{"name":"Slash"},"a~b":{"name":"Tilde"}}}),
        );
        let raw = inventory(std::slice::from_ref(&source), &[]);
        assert!(raw[0]
            .raw_branches
            .iter()
            .any(|b| b.pointer == "/data/a~1b"));
        assert!(raw[0]
            .raw_branches
            .iter()
            .any(|b| b.pointer == "/data/a~0b"));
        let linked = inventory(&[source], &[record("dd", "/data")]);
        assert_eq!(linked[0].linked_branches, 2);
        assert_eq!(linked[0].raw_branches.len(), 1);
    }

    #[test]
    fn ordre_sources_stable_et_racines_vides_restent_inventoriees() {
        let sources = vec![
            source("c", "ddragon", "a", json!([])),
            source("a", "ddragon", "b", Value::Null),
            source("b", "ddragon", "c", json!({})),
        ];
        let mut reversed = sources.clone();
        reversed.reverse();
        let result = inventory(&sources, &[]);
        assert_eq!(result, inventory(&reversed, &[]));
        assert_eq!(result.len(), 3);
        for entry in result {
            assert_eq!(entry.total_leaf_fields, 1);
            assert_eq!(entry.branches, 1);
            assert_eq!(entry.raw_branches[0].pointer, "");
        }
    }
}
