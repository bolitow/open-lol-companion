use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::*;
use crate::catalog::{CatalogValue, RecordCoverage, ValueSource, ValueStatus};
use crate::static_data::{StaticError, StaticResponse, StaticTransport};

const VERSION: &str = "16.19.1";
const BUILD: &str = "16.19.8217343+branch.releases-16-19.content.release";

fn source(key: &str, data: Value) -> CatalogSource {
    let (locale, path) = if let Some((locale, resource)) = key.split_once('/') {
        let language = if locale == "fr_FR" {
            "fr_fr"
        } else {
            "default"
        };
        (
            Some(locale),
            format!("plugins/rcp-be-lol-game-data/global/{language}/v1/{resource}"),
        )
    } else {
        (
            None,
            if key == "items.bin" {
                "game/items.cdtb.bin.json"
            } else {
                "content-metadata.json"
            }
            .into(),
        )
    };
    CatalogSource {
        id: key.into(),
        provider: "cdragon".into(),
        key: key.into(),
        version: BUILD.into(),
        locale: locale.map(str::to_owned),
        url: format!("https://raw.communitydragon.org/16.19/{path}"),
        observed_at: "2026-10-01".into(),
        data,
    }
}

fn sources() -> Vec<CatalogSource> {
    let mut result = vec![
        source("content-metadata.json", json!({"version": BUILD})),
        source(
            "items.bin",
            json!({"Items/3078": {
                "__type": "ItemData", "itemID": 3078, "price": 133,
                "recipeItemLinks": ["Items/3057", "Items/3044", "Items/3051"],
                "mAbilityHasteMod": 15.0, "mFlatHPPoolMod": 333.0,
                "mPercentAttackSpeedMod": 0.30000001192092896,
                "mDataValues": [{"mName": "SpellbladeCooldown", "mValue": 1.5, "__type": "ItemDataValue"}],
                "mItemCalculations": {"SpellbladeDamage": {"mFormulaParts": [{"mStat": 2, "mStatFormula": 1, "mDataValue": "SpellbladeMultiplier", "__type": "StatByNamedDataValueCalculationPart"}], "__type": "GameCalculation"}},
                "mFutureStat": 4,
            }}),
        ),
        source(
            "fr_FR/items.json",
            json!([{"id":3078,"name":"Force de la trinité","description":"<mainText>Texte <attention>15</attention></mainText>","active":false,"inStore":true,"price":133,"priceTotal":3333,"from":[3057,3044,3051],"to":[],"maxStacks":1,"requiredChampion":""}]),
        ),
        source(
            "en_US/items.json",
            json!([{"id":3078,"name":"Trinity Force","description":"<mainText>Text <attention>15</attention></mainText>","active":false,"inStore":true,"price":133,"priceTotal":3333,"from":[3057,3044,3051],"to":[],"maxStacks":1,"requiredChampion":""}]),
        ),
    ];
    for locale in ["fr_FR", "en_US"] {
        result.push(source(&format!("{locale}/perks.json"), json!([
            {"id":5007,"name":if locale == "fr_FR" { "Accélération de compétence" } else { "Ability Haste" },"longDesc":"<attention>+8 Ability Haste</attention>","tooltip":"@value@","iconPath":"/lol-game-data/assets/v1/perk-images/StatMods/StatModsCDRScalingIcon.png"}
        ])));
        result.push(source(&format!("{locale}/perkstyles.json"), json!({"schemaVersion":2,"styles":[
            {"id":8000,"name":"Precision","slots":[{"type":"kStatMod","slotLabel":"Offense","perks":[5007]}]}
        ]})));
    }
    result
}

fn record() -> CatalogRecord {
    CatalogRecord {
        kind: "item".into(),
        id: "3078".into(),
        namespace: "standard".into(),
        locale: "fr_FR".into(),
        name: "Force de la trinité".into(),
        description: None,
        icon: None,
        fields: BTreeMap::new(),
        stats: BTreeMap::new(),
        effects: vec![],
        coverage: RecordCoverage::default(),
    }
}

#[test]
fn numeric_haste_and_parameters_keep_source_paths_and_unsupported_calculations() {
    let mut records = vec![record()];
    enrich(VERSION, &mut records, &sources()).unwrap();
    let item = &records[0];
    assert_eq!(item.stats["ability_haste"].value, 15.0);
    assert_eq!(item.stats["ability_haste"].status, ValueStatus::Verified);
    assert_eq!(item.stats["ability_haste"].unit.as_deref(), Some("points"));
    assert_eq!(
        item.stats["ability_haste"].sources[0].pointer,
        "/Items~13078/mAbilityHasteMod"
    );
    assert_eq!(item.stats["attack_speed"].unit.as_deref(), Some("ratio"));
    let effect = item
        .effects
        .iter()
        .find(|e| e.parameters.contains_key("SpellbladeCooldown"))
        .unwrap();
    assert_eq!(effect.parameters["SpellbladeCooldown"].value, 1.5);
    assert_eq!(effect.parameters["SpellbladeCooldown"].unit, None);
    assert!(item.effects.iter().any(|e| e
        .calculation
        .as_ref()
        .is_some_and(|c| c.status == ValueStatus::Unsupported)));
    assert!(item
        .coverage
        .unmapped_fields
        .iter()
        .any(|f| f.contains("mFutureStat")));
}

#[test]
fn contradictory_values_preserve_all_sources_without_silent_override() {
    let mut item = record();
    item.stats.insert(
        "ability_haste".into(),
        CatalogValue {
            value: json!(10),
            unit: Some("points".into()),
            status: ValueStatus::Verified,
            sources: vec![ValueSource {
                source_id: "ddragon".into(),
                pointer: "/data/3078/stats/abilityHaste".into(),
            }],
        },
    );
    let mut records = vec![item];
    enrich(VERSION, &mut records, &sources()).unwrap();
    let haste = &records[0].stats["ability_haste"];
    assert_eq!(haste.status, ValueStatus::Conflict);
    assert_eq!(haste.value["candidates"][0]["value"], 10);
    assert_eq!(haste.value["candidates"][1]["value"], 15.0);
    assert_eq!(haste.sources.len(), 2);
    assert!(records[0]
        .coverage
        .issues
        .iter()
        .any(|i| i.contains("conflict")));
}

#[test]
fn absence_null_zero_and_false_are_never_conflated() {
    let mut input = sources();
    input[1].data["Items/3078"]["mAbilityHasteMod"] = Value::Null;
    input[1].data["Items/3078"]["mFlatHPPoolMod"] = json!(0);
    let mut records = vec![record()];
    enrich(VERSION, &mut records, &input).unwrap();
    assert_eq!(
        records[0].stats["ability_haste"].status,
        ValueStatus::Missing
    );
    assert_eq!(records[0].stats["health"].value, 0);
    assert_eq!(records[0].fields["active"].value, false);
    assert!(!records[0].stats.contains_key("armor"));
}

#[test]
fn unknown_items_are_added_in_both_languages_without_raw_html() {
    let mut records = vec![];
    enrich(VERSION, &mut records, &sources()).unwrap();
    assert_eq!(records.iter().filter(|r| r.kind == "item").count(), 2);
    assert!(records.iter().all(|r| r.namespace == "standard"));
    assert!(records
        .iter()
        .all(|r| !r.description.as_ref().unwrap().contains('<')));
    assert_eq!(
        records
            .iter()
            .find(|r| r.locale == "fr_FR" && r.kind == "item")
            .unwrap()
            .name,
        "Force de la trinité"
    );
    assert_eq!(
        records[0].fields["builds_from"].value,
        json!(["3057", "3044", "3051"])
    );
}

#[test]
fn patch_mismatch_and_locale_mismatch_are_rejected_before_mutation() {
    for change in 0..4 {
        let mut input = sources();
        match change {
            0 => {
                input[0].data["version"] = json!("16.18.123+branch.releases-16-18.content.release")
            }
            1 => input[1].version = "16.18.123".into(),
            2 => input[3].data[0]["id"] = json!(999),
            _ => input[1].data["Items/3078"]["itemID"] = json!(999),
        }
        let mut records = vec![record()];
        let original = records.clone();
        assert!(enrich(VERSION, &mut records, &input).is_err());
        assert_eq!(records, original);
    }
}

#[derive(Clone)]
struct MockTransport {
    responses: Arc<BTreeMap<String, StaticResponse>>,
    requested: Arc<Mutex<Vec<String>>>,
}

impl MockTransport {
    fn valid() -> Self {
        Self {
            responses: Arc::new(
                sources()
                    .into_iter()
                    .map(|s| {
                        (
                            s.url,
                            StaticResponse {
                                status: 200,
                                body: serde_json::to_vec(&s.data).unwrap(),
                            },
                        )
                    })
                    .collect(),
            ),
            requested: Arc::default(),
        }
    }
}

impl StaticTransport for MockTransport {
    async fn get(&self, url: &str) -> Result<StaticResponse, StaticError> {
        self.requested.lock().unwrap().push(url.into());
        self.responses.get(url).cloned().ok_or(StaticError::Network)
    }
}

#[tokio::test]
async fn fetch_pins_patch_and_checks_exact_build() {
    let transport = MockTransport::valid();
    let fetched = fetch_sources_with(VERSION, transport.clone())
        .await
        .unwrap();
    assert_eq!(fetched.len(), 8);
    assert!(fetched.iter().all(|s| s.version == BUILD));
    let requested = transport.requested.lock().unwrap();
    assert!(requested
        .iter()
        .all(|u| u.starts_with("https://raw.communitydragon.org/16.19/") && !u.contains("latest")));
    assert_eq!(
        requested
            .iter()
            .filter(|u| u.ends_with("content-metadata.json"))
            .count(),
        2
    );
}

#[tokio::test]
async fn malformed_version_fails_before_network_and_rate_limits_are_not_retried() {
    let transport = MockTransport::valid();
    assert!(fetch_sources_with("latest", transport.clone())
        .await
        .is_err());
    assert!(transport.requested.lock().unwrap().is_empty());
    let mut responses = (*transport.responses).clone();
    for r in responses.values_mut() {
        r.status = 429;
    }
    let limited = MockTransport {
        responses: Arc::new(responses),
        ..transport
    };
    assert!(fetch_sources_with(VERSION, limited.clone()).await.is_err());
    assert_eq!(limited.requested.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn http_failure_oversized_document_and_foreign_patch_abort_fetch() {
    for scenario in 0..3 {
        let mut transport = MockTransport::valid();
        let mut responses = (*transport.responses).clone();
        let metadata = responses
            .get_mut("https://raw.communitydragon.org/16.19/content-metadata.json")
            .unwrap();
        match scenario {
            0 => metadata.status = 403,
            1 => metadata.body = vec![b' '; 32 * 1024 * 1024 + 1],
            _ => {
                metadata.body =
                    br#"{"version":"16.18.8217343+branch.releases-16-18.content.release"}"#.to_vec()
            }
        }
        transport.responses = Arc::new(responses);
        assert!(fetch_sources_with(VERSION, transport.clone())
            .await
            .is_err());
        assert_eq!(transport.requested.lock().unwrap().len(), 1);
    }
}

#[test]
fn malformed_numeric_value_stays_unsupported_and_source_url_is_validated() {
    let mut input = sources();
    input[1].data["Items/3078"]["mAbilityHasteMod"] = json!("15");
    let mut records = vec![record()];
    enrich(VERSION, &mut records, &input).unwrap();
    assert_eq!(
        records[0].stats["ability_haste"].status,
        ValueStatus::Unsupported
    );
    assert_eq!(records[0].stats["ability_haste"].value, "15");
    input[1].url =
        "https://raw.communitydragon.org.evil.invalid/16.19/game/items.cdtb.bin.json".into();
    assert!(enrich(VERSION, &mut records, &input).is_err());
}

#[test]
fn fragments_are_identified_from_source_slots_without_parsing_numeric_descriptions() {
    let mut records = vec![];
    enrich(VERSION, &mut records, &sources()).unwrap();
    let shard = records
        .iter()
        .find(|r| r.kind == "rune_shard" && r.locale == "fr_FR")
        .unwrap();
    assert_eq!(shard.id, "5007");
    assert_eq!(shard.name, "Accélération de compétence");
    assert_eq!(shard.description.as_deref(), Some("+8 Ability Haste"));
    assert_eq!(shard.fields["listed_in_perk_styles"].value, true);
    assert!(shard.stats.is_empty());
    assert!(shard
        .coverage
        .issues
        .iter()
        .any(|i| i == "missing:structured_effect_parameters"));
    assert!(shard
        .coverage
        .issues
        .iter()
        .any(|i| i == "unresolved_placeholder:tooltip"));
    assert!(shard
        .icon
        .as_deref()
        .unwrap()
        .starts_with("https://raw.communitydragon.org/16.19/"));
}

#[test]
fn real_bin_keys_and_mode_specific_values_are_not_mixed_with_base_values() {
    let mut input = sources();
    let item = &mut input[1].data["Items/3078"];
    item["flatMPPoolMod"] = json!(600);
    item["PhysicalLethality"] = json!(18);
    item["PercentOmnivampMod"] = json!(0.15);
    item["mPercentTenacityItemMod"] = json!(0.3);
    item["DataValuesModeOverride"] =
        json!({"ARAM":{"DataValues":[{"mName":"SpellbladeCooldown","mValue":3}]}});
    let mut records = vec![record()];
    enrich(VERSION, &mut records, &input).unwrap();
    assert_eq!(records[0].stats["mana"].value, 600);
    assert_eq!(records[0].stats["lethality"].value, 18);
    assert_eq!(records[0].stats["omnivamp"].value, 0.15);
    assert_eq!(records[0].stats["tenacity"].value, 0.3);
    // #116 : la surcharge ARAM est interprétée, jamais fondue dans la valeur de base.
    assert!(!records[0].fields.contains_key("mode_parameter_overrides"));
    assert_eq!(
        records[0]
            .effects
            .iter()
            .find(|e| e.id == "cdragon_parameters")
            .unwrap()
            .parameters["SpellbladeCooldown"]
            .value,
        1.5
    );
    assert_eq!(
        records[0]
            .effects
            .iter()
            .find(|e| e.id == "cdragon_parameters:ARAM")
            .unwrap()
            .parameters["SpellbladeCooldown"]
            .value,
        3
    );
}

fn with_override(raw: Value) -> Vec<CatalogRecord> {
    let mut input = sources();
    input[1].data["Items/3078"]["DataValuesModeOverride"] = raw;
    let mut records = vec![record()];
    enrich(VERSION, &mut records, &input).unwrap();
    records
}

fn mode_effect<'a>(record: &'a CatalogRecord, mode: &str) -> Option<&'a CatalogEffect> {
    record
        .effects
        .iter()
        .find(|e| e.id == format!("cdragon_parameters:{mode}"))
}

#[test]
fn aram_override_becomes_a_verified_effect_with_its_own_provenance() {
    let records = with_override(json!({"ARAM": {"DataValues": [
        {"mName": "SpellbladeCooldown", "mValue": 3.0, "__type": "ItemDataValue"},
        {"mName": "BonusADRatio", "mValue": 1, "__type": "ItemDataValue"}
    ], "__type": "ItemDataValues"}}));
    let fr = &records[0];
    let effect = mode_effect(fr, "ARAM").unwrap();
    let cooldown = &effect.parameters["SpellbladeCooldown"];
    assert_eq!(cooldown.value, 3.0);
    assert_eq!(cooldown.status, ValueStatus::Verified);
    assert_eq!(
        cooldown.sources[0].pointer,
        "/Items~13078/DataValuesModeOverride/ARAM/DataValues/0/mValue"
    );
    assert_eq!(effect.parameters["BonusADRatio"].value, 1);
    assert!(!fr.fields.contains_key("mode_parameter_overrides"));
    assert!(!fr
        .coverage
        .unmapped_fields
        .iter()
        .any(|field| field.contains("DataValuesModeOverride")));
    assert!(!fr
        .coverage
        .issues
        .iter()
        .any(|issue| issue.contains("mode_parameter_overrides")));
}

#[test]
fn each_mode_gets_a_separate_effect_and_unresolved_keys_are_kept_and_reported() {
    let records = with_override(json!({
        "ARAM": {"DataValues": [{"mName": "A", "mValue": 1.0}]},
        "cherry": {"DataValues": [{"mName": "A", "mValue": 2.0}]},
        "{bffdf499}": {"DataValues": [{"mName": "A", "mValue": 4.0}]}
    }));
    let fr = &records[0];
    assert_eq!(mode_effect(fr, "ARAM").unwrap().parameters["A"].value, 1.0);
    assert_eq!(
        mode_effect(fr, "cherry").unwrap().parameters["A"].value,
        2.0
    );
    assert_eq!(
        mode_effect(fr, "{bffdf499}").unwrap().parameters["A"].value,
        4.0
    );
    assert!(fr
        .coverage
        .issues
        .iter()
        .any(|issue| issue == "unresolved_mode_key:{bffdf499}"));
    assert!(!fr
        .coverage
        .issues
        .iter()
        .any(|issue| issue.starts_with("unresolved_mode_key:ARAM")));
}

#[test]
fn a_non_numeric_mode_value_is_unsupported_and_a_duplicate_is_a_conflict() {
    let records = with_override(json!({"ARAM": {"DataValues": [
        {"mName": "Text", "mValue": "3"},
        {"mName": "Empty", "mValue": null},
        {"mName": "Twice", "mValue": 1.0},
        {"mName": "Twice", "mValue": 2.0}
    ]}}));
    let fr = &records[0];
    let effect = mode_effect(fr, "ARAM").unwrap();
    assert_eq!(effect.parameters["Text"].status, ValueStatus::Unsupported);
    assert_eq!(effect.parameters["Empty"].status, ValueStatus::Missing);
    assert_eq!(effect.parameters["Twice"].status, ValueStatus::Conflict);
    assert!(fr
        .coverage
        .issues
        .iter()
        .any(|issue| issue == "conflict:effect_parameter.ARAM.Twice"));
    assert!(fr
        .coverage
        .unmapped_fields
        .iter()
        .any(|field| field.ends_with("/DataValuesModeOverride/ARAM/DataValues/0/mValue")));
}

#[test]
fn an_unparseable_override_stays_unsupported_without_inventing_an_effect() {
    for raw in [
        json!({"ARAM": 3}),
        json!({"ARAM": {"DataValues": []}}),
        json!([1, 2]),
        Value::Null,
    ] {
        let records = with_override(raw.clone());
        let fr = &records[0];
        assert_eq!(
            fr.fields["mode_parameter_overrides"].status,
            ValueStatus::Unsupported,
            "{raw}"
        );
        assert_eq!(fr.fields["mode_parameter_overrides"].value, raw);
        assert!(fr
            .effects
            .iter()
            .all(|effect| !effect.id.starts_with("cdragon_parameters:")));
    }
}

#[test]
fn store_presence_does_not_override_purchase_eligibility_and_float_noise_is_not_conflict() {
    let mut item = record();
    item.fields.insert(
        "purchasable".into(),
        CatalogValue {
            value: json!(false),
            unit: None,
            status: ValueStatus::Verified,
            sources: vec![],
        },
    );
    item.stats.insert(
        "attack_speed".into(),
        CatalogValue {
            value: json!(0.3),
            unit: Some("ratio".into()),
            status: ValueStatus::Verified,
            sources: vec![],
        },
    );
    let mut records = vec![item];
    enrich(VERSION, &mut records, &sources()).unwrap();
    assert_eq!(records[0].fields["purchasable"].value, false);
    assert_eq!(records[0].fields["in_store"].value, true);
    assert_eq!(
        records[0].stats["attack_speed"].status,
        ValueStatus::Verified
    );
    assert_eq!(records[0].stats["attack_speed"].value, 0.3);
}

#[test]
fn unlisted_shards_are_retained_and_unsafe_icon_paths_are_not_urls() {
    let mut input = sources();
    for source in input.iter_mut().filter(|s| s.key.ends_with("/perks.json")) {
        source.data.as_array_mut().unwrap().push(json!({"id":5002,"name":"Armor","longDesc":"+6 Armor", "iconPath":"/lol-game-data/assets/v1/perk-images/StatMods/../outside.png"}));
    }
    let mut records = vec![];
    enrich(VERSION, &mut records, &input).unwrap();
    let old = records
        .iter()
        .find(|r| r.id == "5002" && r.locale == "fr_FR")
        .unwrap();
    assert_eq!(old.kind, "rune_shard");
    assert_eq!(old.fields["listed_in_perk_styles"].value, false);
    assert_eq!(old.icon, None);
}

#[test]
fn missing_bin_entry_and_invalid_perk_reference_prevent_partial_projection() {
    for scenario in 0..2 {
        let mut input = sources();
        if scenario == 0 {
            input[1].data.as_object_mut().unwrap().insert(
                "Items/999".into(),
                json!({"itemID":999,"__type":"ItemData"}),
            );
        } else {
            for source in input
                .iter_mut()
                .filter(|s| s.key.ends_with("/perkstyles.json"))
            {
                source.data["styles"][0]["slots"][0]["perks"] = json!([999]);
            }
        }
        let mut records = vec![record()];
        let original = records.clone();
        assert!(enrich(VERSION, &mut records, &input).is_err());
        assert_eq!(records, original);
    }
}

#[test]
fn reserved_item_without_name_is_retained_under_its_id() {
    let mut input = sources();
    for source in input.iter_mut().filter(|s| s.key.ends_with("/items.json")) {
        source.data[0]["name"] = json!("");
    }
    let mut records = vec![];
    enrich(VERSION, &mut records, &input).unwrap();
    let item = records
        .iter()
        .find(|r| r.kind == "item" && r.locale == "fr_FR")
        .unwrap();
    assert_eq!(item.name, "3078");
    assert_eq!(item.fields["community_name"].value, "");
    assert_eq!(item.fields["community_name"].sources[0].pointer, "/0/name");
    assert!(item
        .coverage
        .issues
        .iter()
        .any(|i| i == "missing:community_name"));
}

#[test]
fn community_only_names_and_ids_keep_direct_provenance() {
    let mut records = vec![];
    enrich(VERSION, &mut records, &sources()).unwrap();
    for record in &records {
        let name = &record.fields["community_name"];
        assert_eq!(name.value, record.name);
        assert_eq!(name.status, ValueStatus::Descriptive);
        assert_eq!(name.sources[0].pointer, "/0/name");
        assert_eq!(record.fields["community_id"].value, record.id);
        assert_eq!(record.fields["community_id"].sources[0].pointer, "/0/id");
    }
}

#[derive(Clone)]
struct ChangingMetadata(MockTransport);

impl StaticTransport for ChangingMetadata {
    async fn get(&self, url: &str) -> Result<StaticResponse, StaticError> {
        let mut response = self.0.get(url).await?;
        if url.ends_with("content-metadata.json") && self.0.requested.lock().unwrap().len() > 1 {
            response.body =
                br#"{"version":"16.19.9999999+branch.releases-16-19.content.release"}"#.to_vec();
        }
        Ok(response)
    }
}

#[tokio::test]
async fn changing_build_during_download_prevents_publication() {
    let transport = ChangingMetadata(MockTransport::valid());
    let result = fetch_sources_with(VERSION, transport.clone()).await;
    assert!(matches!(result, Err(CatalogError::InvalidSource)));
    assert_eq!(
        transport
            .0
            .requested
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.ends_with("content-metadata.json"))
            .count(),
        2
    );
}

#[test]
fn recipes_and_categories_ignore_order_but_keep_component_multiplicity() {
    let mut input = sources();
    for source in input.iter_mut().filter(|s| s.key.ends_with("/items.json")) {
        source.data[0]["from"] = json!([1002, 1001, 1001]);
        source.data[0]["to"] = json!([3158, 3006]);
        source.data[0]["categories"] = json!(["Health", "AbilityHaste"]);
    }
    input[1].data["Items/3078"]["recipeItemLinks"] =
        json!(["Items/1002", "Items/1001", "Items/1001"]);
    let mut item = record();
    for (key, value) in [
        ("builds_from", json!(["1001", "1001", "1002"])),
        ("builds_into", json!(["3006", "3158"])),
        ("categories", json!(["AbilityHaste", "Health"])),
    ] {
        item.fields.insert(
            key.into(),
            CatalogValue {
                value,
                unit: None,
                status: ValueStatus::Verified,
                sources: vec![],
            },
        );
    }
    let mut records = vec![item.clone()];
    enrich(VERSION, &mut records, &input).unwrap();
    for key in ["builds_from", "builds_into", "categories"] {
        assert_eq!(
            records[0].fields[key].status,
            ValueStatus::Verified,
            "{key}"
        );
        assert!(!records[0].fields[key].sources.is_empty());
    }
    item.fields.get_mut("builds_from").unwrap().value = json!(["1001", "1002"]);
    let mut records = vec![item];
    enrich(VERSION, &mut records, &input).unwrap();
    assert_eq!(
        records[0].fields["builds_from"].status,
        ValueStatus::Conflict
    );
}

#[test]
#[ignore = "recette explicite sur exports publics locaux, aucun réseau"]
fn real_public_exports_project_all_items_and_shards() {
    let directory =
        std::env::var("OLC_CATALOG_FIXTURE_DIR").expect("répertoire des fixtures publiques");
    let metadata: Value = serde_json::from_slice(
        &std::fs::read(format!("{directory}/content-metadata.json")).unwrap(),
    )
    .unwrap();
    let build = metadata["version"].as_str().unwrap();
    let patch = build.split('+').next().unwrap().rsplit_once('.').unwrap().0;
    let version = format!("{patch}.1");
    let mut sources = vec![];
    for (key, locale, path) in http::RESOURCES {
        let data = serde_json::from_slice(
            &std::fs::read(format!("{directory}/{}", key.replace('/', "-"))).unwrap(),
        )
        .unwrap();
        sources.push(crate::catalog::make_source(
            "cdragon",
            key,
            build,
            locale,
            &http::url(patch, path),
            "fixture",
            data,
        ));
    }
    let item_count = sources
        .iter()
        .find(|s| s.key == "fr_FR/items.json")
        .unwrap()
        .data
        .as_array()
        .unwrap()
        .len();
    let mut records = vec![];
    enrich(&version, &mut records, &sources).unwrap();
    assert_eq!(
        records.iter().filter(|r| r.kind == "item").count(),
        item_count * 2
    );
    let trinity = records
        .iter()
        .find(|r| r.kind == "item" && r.id == "3078" && r.locale == "fr_FR")
        .unwrap();
    assert_eq!(trinity.stats["ability_haste"].value, 15.0);
    assert!(records.iter().any(|r| r.kind == "rune_shard"));
    println!(
        "patch={patch} items={} shards={} runes={} records={}",
        item_count * 2,
        records.iter().filter(|r| r.kind == "rune_shard").count(),
        records.iter().filter(|r| r.kind == "rune").count(),
        records.len()
    );
}
