use serde_json::{json, Value};

use super::*;

/// Champs normalisés #61 (`data.fields`) d'un objet : valeur et statut de source.
fn record(fields: Value) -> Value {
    let mut result = serde_json::Map::new();
    for (name, value) in fields.as_object().unwrap() {
        result.insert(
            name.clone(),
            json!({"value": value, "unit": null, "status": "verified", "sources": []}),
        );
    }
    Value::Object(result)
}

/// Extrait synthétique du catalogue 16.19.1 (valeurs relevées sur Data Dragon).
fn catalog() -> ItemCatalog {
    let records = [
        (
            "1001",
            json!({"price_total":300,"purchasable":true,"categories":["Boots"],"builds_into":["3006","3009"]}),
        ),
        (
            "2422",
            json!({"price_total":300,"purchasable":false,"in_store":false,"categories":["Boots"],"builds_into":["3006"]}),
        ),
        (
            "3006",
            json!({"price_total":1100,"purchasable":true,"categories":["AttackSpeed","Boots"],"builds_from":["1001","1042","1042"],"builds_into":["3172"]}),
        ),
        (
            "3172",
            json!({"price_total":1100,"purchasable":true,"categories":["AttackSpeed","LifeSteal"],"builds_from":["3006"]}),
        ),
        (
            "3009",
            json!({"price_total":1000,"purchasable":true,"categories":["Boots"],"builds_from":["1001"],"builds_into":["3170"]}),
        ),
        (
            "1055",
            json!({"price_total":450,"purchasable":true,"categories":["Health","Damage","Lane"]}),
        ),
        (
            "2003",
            json!({"price_total":50,"purchasable":true,"categories":["HealthRegen","Consumable"]}),
        ),
        (
            "3340",
            json!({"price_total":0,"purchasable":true,"categories":["Trinket","Vision"]}),
        ),
        (
            "1036",
            json!({"price_total":350,"purchasable":true,"categories":["Damage"],"builds_into":["3004"]}),
        ),
        (
            "3004",
            json!({"price_total":2900,"purchasable":true,"categories":["Damage","Mana"],"builds_from":["3070","3133","1036"]}),
        ),
        (
            "3042",
            json!({"price_total":2900,"purchasable":false,"in_store":false,"categories":["Damage","Mana"],"special_recipe":3004}),
        ),
        (
            "3031",
            json!({"price_total":3500,"purchasable":true,"categories":["CriticalStrike","Damage"],"builds_from":["1038","1037","1018"]}),
        ),
        (
            "3089",
            json!({"price_total":3500,"purchasable":true,"categories":["SpellDamage"]}),
        ),
        (
            "6672",
            json!({"price_total":3000,"purchasable":true,"categories":["Damage","AttackSpeed"]}),
        ),
        (
            "3072",
            json!({"price_total":3400,"purchasable":true,"categories":["Damage","LifeSteal"]}),
        ),
        (
            "3026",
            json!({"price_total":3200,"purchasable":true,"categories":["Armor","Damage"]}),
        ),
        (
            "3036",
            json!({"price_total":3300,"purchasable":true,"categories":["Damage","CriticalStrike"]}),
        ),
        (
            "3041",
            json!({"price_total":1500,"purchasable":true,"categories":["Health","SpellDamage"],"builds_from":["1082"]}),
        ),
        (
            "3869",
            json!({"price_total":400,"purchasable":true,"categories":["Health","GoldPer"],"builds_from":["3867"]}),
        ),
        (
            "3002",
            json!({"price_total":2400,"purchasable":false,"in_store":false,"categories":["Health","Armor"]}),
        ),
        (
            "2065",
            json!({"price_total":2200,"purchasable":true,"categories":["SpellDamage"]}),
        ),
    ];
    let records: Vec<_> = records
        .into_iter()
        .map(|(id, fields)| (id, record(fields)))
        .collect();
    ItemCatalog::from_records("16.19.1", records.iter().map(|(id, f)| (*id, f)))
}

fn buy(item_id: u32, timestamp_ms: u64) -> Purchase {
    Purchase {
        item_id,
        timestamp_ms,
    }
}

#[test]
fn reconnait_objets_complets_bottes_et_transformations_par_le_catalogue() {
    let c = catalog();
    assert_eq!(c.version, "16.19.1");
    // Profondeur 2 (Infinity Edge) : seuls prix, achat et absence d'évolution comptent.
    for completed in [3031, 3089, 3004, 2065] {
        assert!(
            c.is_completed(completed),
            "{completed} devrait être complet"
        );
    }
    // Mejai, objets de quête, bottes, consommables et objets retirés de la boutique.
    for other in [3041, 3869, 3006, 3172, 2003, 3340, 1036, 3002, 1001, 9999] {
        assert!(
            !c.is_completed(other),
            "{other} ne devrait pas être complet"
        );
    }
    // La transformation Muramana rejoint Manamune via special_recipe.
    assert_eq!(c.canonical(3042), 3004);
    assert!(c.is_completed(c.canonical(3042)));
    // Gunmetal Greaves n'a pas l'étiquette Boots : la chaîne builds_from la rattache.
    for boots in [3006, 3172, 3009] {
        assert!(c.is_boots(boots), "{boots} devrait être des bottes");
    }
    for other in [1001, 2422, 3031, 1055] {
        assert!(!c.is_boots(other), "{other} ne devrait pas être des bottes");
    }
}

#[test]
fn derive_depart_bottes_core_et_emplacements_dans_l_ordre_des_achats() {
    let steps = catalog().derive_steps(&[
        buy(1055, 1_000),
        buy(2003, 1_200),
        buy(2003, 1_300),
        buy(3340, 1_400),
        buy(1036, 300_000),
        buy(3006, 400_000),
        buy(3004, 600_000),
        buy(3031, 900_000),
        buy(3172, 1_000_000),
        buy(3042, 1_100_000),
        buy(6672, 1_200_000),
        buy(3072, 1_300_000),
        buy(3026, 1_400_000),
        buy(3036, 1_500_000),
    ]);
    // Départ : multiensemble trié, potions doublées conservées, balise exclue.
    assert_eq!(steps["starter"], vec![1055, 2003, 2003]);
    assert_eq!(steps["boots"], vec![3006]);
    // Core ordonné : Muramana ne double pas Manamune.
    assert_eq!(steps["core"], vec![3004, 3031, 6672]);
    assert_eq!(steps["item_slot_4"], vec![3072]);
    assert_eq!(steps["item_slot_5"], vec![3026]);
    assert_eq!(steps["item_slot_6"], vec![3036]);
    assert_eq!(steps.len(), 6);
}

#[test]
fn la_fenetre_de_depart_exclut_la_borne_de_1_min_30() {
    let steps = catalog().derive_steps(&[buy(1055, 89_999), buy(2003, 90_000)]);
    assert_eq!(steps["starter"], vec![1055]);
}

#[test]
fn moins_de_trois_objets_complets_ne_publient_ni_core_ni_emplacement() {
    let steps = catalog().derive_steps(&[buy(1055, 0), buy(3031, 600_000), buy(3089, 900_000)]);
    assert_eq!(steps["starter"], vec![1055]);
    // Aucune botte achetée : choix observé vide, pas une donnée manquante.
    assert_eq!(steps["boots"], Vec::<u32>::new());
    assert!(!steps.contains_key("core"));
    assert!(!steps.contains_key("item_slot_4"));
    let four = catalog().derive_steps(&[buy(3031, 1), buy(3089, 2), buy(6672, 3), buy(3072, 4)]);
    assert_eq!(four["item_slot_4"], vec![3072]);
    assert!(!four.contains_key("item_slot_5"));
}

#[test]
fn un_objet_revendu_puis_rachete_ne_compte_qu_une_fois() {
    let steps = catalog().derive_steps(&[buy(3031, 1), buy(3031, 2), buy(3089, 3), buy(6672, 4)]);
    assert_eq!(steps["core"], vec![3031, 3089, 6672]);
}

#[test]
fn un_champ_non_verifie_ou_absent_n_est_pas_devine() {
    let mut fields = record(json!({"price_total":3000,"categories":["Damage"]}));
    let conflict = record(json!({"price_total":3000,"purchasable":true,"categories":["Damage"]}));
    let mut conflict = conflict;
    conflict["purchasable"]["status"] = json!("conflict");
    fields["builds_into"] = json!({"value":[],"status":"missing","sources":[]});
    let records = [("5001", fields), ("5002", conflict)];
    let c = ItemCatalog::from_records("16.19.1", records.iter().map(|(id, f)| (*id, f)));
    assert!(!c.is_completed(5001));
    assert!(!c.is_completed(5002));
}

#[test]
fn une_chaine_de_transformation_circulaire_reste_bornee() {
    let records = [
        ("7001", record(json!({"special_recipe":7002}))),
        ("7002", record(json!({"special_recipe":7001}))),
    ];
    let c = ItemCatalog::from_records("16.19.1", records.iter().map(|(id, f)| (*id, f)));
    let root = c.canonical(7001);
    assert!(root == 7001 || root == 7002);
}
