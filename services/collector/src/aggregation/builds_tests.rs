use super::*;
use serde_json::json;

fn participant() -> Value {
    json!({
        "item0": 3006, "item1": 2003, "item2": 2003, "item3": 0,
        "item4": 3031, "item5": 0, "item6": 3340,
        "summoner1Id": 14, "summoner2Id": 4,
        "perks": {
            "styles": [
                {"description":"subStyle", "style": 8200,
                 "selections": [{"perk": 8224},{"perk": 8234}]},
                {"description":"primaryStyle", "style": 8000,
                 "selections": [{"perk": 8005},{"perk": 9111},{"perk": 9104},{"perk": 8014}]}
            ],
            "statPerks": {"offense": 5005, "flex": 5008, "defense": 5011}
        },
        "puuid":"identifiant-secret-synthetique", "riotIdGameName":"nom-prive"
    })
}

fn timeline(events: Vec<Value>) -> Value {
    json!({
        "metadata": {"matchId": "EUW1_SYNTHETIQUE", "participants": ["p1","p2"]},
        "info": {
            "participants": [{"participantId": 1,"puuid":"p1"},{"participantId":2,"puuid":"p2"}],
            "frames": [{"timestamp": 0,"events":events}]
        }
    })
}

fn extract(events: Vec<Value>) -> BuildObservation {
    extract_timeline(&timeline(events), "EUW1_SYNTHETIQUE", 1).unwrap()
}

fn skill(at: u64, slot: u32) -> Value {
    json!({"type":"SKILL_LEVEL_UP","participantId":1,"timestamp":at,"skillSlot":slot,"levelUpType":"NORMAL"})
}

fn item(kind: &str, at: u64, id: u32) -> Value {
    json!({"type":kind,"participantId":1,"timestamp":at,"itemId":id})
}

#[test]
fn extrait_les_categories_completes_et_omet_toute_donnee_joueur() {
    let result = extract_detail(&participant());
    assert_eq!(result.variants["final_items"], vec![2003, 3006, 3031]);
    assert_eq!(result.variants["trinket"], vec![3340]);
    assert_eq!(result.variants["summoner_spells"], vec![4, 14]);
    assert_eq!(
        result.variants["runes"],
        vec![8000, 8005, 9111, 9104, 8014, 8200, 8224, 8234, 5005, 5008, 5011]
    );
    let serialized = serde_json::to_string(&result).unwrap();
    for private in ["puuid", "identifiant-secret", "riotId", "nom-prive"] {
        assert!(!serialized.contains(private));
    }
}

#[test]
fn derive_les_categories_de_runes_de_la_page_exacte() {
    let result = extract_detail(&participant());
    let v = &result.variants;
    assert_eq!(v["rune_keystone"], vec![8005]);
    assert_eq!(v["rune_primary_style"], vec![8000]);
    assert_eq!(v["rune_secondary_style"], vec![8200]);
    assert_eq!(v["rune_secondary_pair"], vec![8200, 8224, 8234]);
    // Chaque rune d'emplacement est conditionnée à sa clé de voûte.
    assert_eq!(v["rune_slot_1"], vec![8005, 9111]);
    assert_eq!(v["rune_slot_2"], vec![8005, 9104]);
    assert_eq!(v["rune_slot_3"], vec![8005, 8014]);
    assert_eq!(v["rune_shard_offense"], vec![5005]);
    assert_eq!(v["rune_shard_flex"], vec![5008]);
    assert_eq!(v["rune_shard_defense"], vec![5011]);
    // La page exacte reste publiée telle quelle.
    assert_eq!(v["runes"].len(), 11);
}

#[test]
fn la_paire_secondaire_est_independante_de_l_ordre_transmis() {
    let mut source = participant();
    source["perks"]["styles"][0]["selections"] = json!([{"perk": 8234},{"perk": 8224}]);
    assert_eq!(
        extract_detail(&source).variants["rune_secondary_pair"],
        vec![8200, 8224, 8234]
    );
}

#[test]
fn une_page_de_runes_incomplete_ne_produit_aucune_categorie_derivee() {
    let mut source = participant();
    source["perks"]["statPerks"]["flex"] = json!(0);
    let result = extract_detail(&source);
    assert!(!result.variants.keys().any(|c| c.starts_with("rune")));
}

#[test]
fn une_categorie_incomplete_ne_contamine_pas_les_autres() {
    let mut source = participant();
    source.as_object_mut().unwrap().remove("item2");
    source["summoner1Id"] = json!(-1);
    source["item6"] = json!(-1);
    let result = extract_detail(&source);
    // Seules les catégories de runes subsistent (page exacte et ses dérivées).
    assert!(result
        .variants
        .keys()
        .all(|category| category == "runes" || category.starts_with("rune_")));
    // Page exacte et ses dix dérivées : aucune catégorie dérivée ne doit disparaître.
    assert_eq!(result.variants.len(), 11);
    assert!(result.variants.contains_key("runes"));
    source["perks"]["styles"][1]["selections"] = json!([]);
    assert!(extract_detail(&source).variants.is_empty());
    assert!(extract_detail(&json!({})).variants.is_empty());
}

#[test]
fn trie_les_evenements_et_conserve_un_ordre_stable_a_timestamp_egal() {
    let result = extract(vec![
        skill(3000, 3),
        skill(1000, 1),
        skill(1000, 2),
        item("ITEM_PURCHASED", 2000, 2003),
        item("ITEM_PURCHASED", 100, 1001),
        json!({"type":"LEVEL_UP","participantId":1,"timestamp":2500,"level":9}),
    ]);
    assert_eq!(result.variants["skill_order"], vec![1, 2, 3]);
    assert_eq!(
        result
            .skill_steps
            .iter()
            .map(|s| (s.level, s.slot, s.timestamp_ms))
            .collect::<Vec<_>>(),
        vec![(1, 1, 1000), (2, 2, 1000), (3, 3, 3000)]
    );
    assert_eq!(result.variants["purchase_order"], vec![1001, 2003]);
}

#[test]
fn une_annulation_retire_uniquement_le_dernier_achat_concerne() {
    let result = extract(vec![
        item("ITEM_PURCHASED", 1, 2003),
        item("ITEM_PURCHASED", 2, 1001),
        item("ITEM_PURCHASED", 3, 2003),
        json!({"type":"ITEM_UNDO","participantId":1,"timestamp":4,"beforeId":2003,"afterId":0,"goldGain":50}),
        item("ITEM_SOLD", 5, 1001),
        json!({"type":"ITEM_UNDO","participantId":1,"timestamp":6,"beforeId":0,"afterId":1001,"goldGain":-210}),
        item("ITEM_DESTROYED", 7, 2003),
    ]);
    assert_eq!(result.variants["purchase_order"], vec![2003, 1001]);
    assert!(result
        .item_events
        .iter()
        .any(|e| e.kind == "ITEM_UNDO_REMOVE" && e.item_id == 2003));
    assert!(result
        .item_events
        .iter()
        .any(|e| e.kind == "ITEM_UNDO_RESTORE" && e.item_id == 1001));
    assert_eq!(result.item_events.last().unwrap().kind, "ITEM_DESTROYED");
}

#[test]
fn valide_le_match_et_le_participant_avant_toute_extraction() {
    let source = timeline(vec![skill(100, 1)]);
    assert!(extract_timeline(&source, "EUW1_AUTRE", 1).is_err());
    assert!(extract_timeline(&source, "EUW1_SYNTHETIQUE", 3).is_err());
    assert!(extract_timeline(&source, "EUW1_SYNTHETIQUE", 0).is_err());
    assert!(extract_timeline(&json!({}), "EUW1_SYNTHETIQUE", 1).is_err());
}

#[test]
fn ignore_les_evenements_des_autres_participants_et_les_types_inconnus() {
    let result = extract(vec![
        json!({"type":"SKILL_LEVEL_UP","participantId":2,"timestamp":1,"skillSlot":99}),
        json!({"type":"EVENEMENT_FUTUR","timestamp":2,"riotIdGameName":"nom-prive"}),
        skill(3, 1),
    ]);
    assert_eq!(result.variants["skill_order"], vec![1]);
    assert_eq!(result.item_events.len(), 0);
    assert!(!serde_json::to_string(&result)
        .unwrap()
        .contains("nom-prive"));
}

#[test]
fn rejette_les_sequences_partielles_ou_invalides_sans_fabriquer_de_build() {
    for event in [
        skill(1, 0),
        skill(1, 5),
        json!({"type":"SKILL_LEVEL_UP","participantId":1,"skillSlot":1,"levelUpType":"NORMAL"}),
        json!({"type":"SKILL_LEVEL_UP","participantId":1,"timestamp":1,"skillSlot":1}),
        json!({"type":"ITEM_PURCHASED","participantId":1,"timestamp":1}),
        json!({"type":"ITEM_UNDO","participantId":1,"timestamp":1,"beforeId":3006,"afterId":0}),
    ] {
        assert!(
            extract_timeline(&timeline(vec![skill(0, 2), event]), "EUW1_SYNTHETIQUE", 1).is_err()
        );
    }
    let empty = extract(vec![]);
    assert!(empty.variants.is_empty());
    assert!(empty.skill_steps.is_empty());
}

#[test]
fn separe_les_points_speciaux_des_points_de_sort_normaux() {
    let result = extract(vec![
        skill(1, 1),
        json!({"type":"SKILL_LEVEL_UP","participantId":1,"timestamp":2,"skillSlot":2,"levelUpType":"EVOLVE"}),
        skill(3, 2),
    ]);
    assert_eq!(result.variants["skill_order"], vec![1, 2]);
    assert_eq!(result.variants["special_skill_order"], vec![2]);
    assert_eq!(result.skill_steps.len(), 2);
}

#[test]
fn plafonne_les_sequences_sans_tronquer_silencieusement() {
    let at_limit = (0..64).map(|at| skill(at, 1)).collect();
    assert_eq!(extract(at_limit).skill_steps.len(), 64);
    let too_long = (0..65).map(|at| skill(at, 1)).collect();
    assert!(extract_timeline(&timeline(too_long), "EUW1_SYNTHETIQUE", 1).is_err());
}

#[test]
fn distingue_inventaire_vide_et_inventaire_inconnu() {
    let mut source = json!({"item0":0,"item1":0,"item2":0,"item3":0,"item4":0,"item5":0,"item6":0});
    let result = extract_detail(&source);
    assert_eq!(result.variants["final_items"], Vec::<u32>::new());
    assert_eq!(result.variants["trinket"], Vec::<u32>::new());
    source["item1"] = json!(u64::from(u32::MAX) + 1);
    assert!(!extract_detail(&source).variants.contains_key("final_items"));
    assert!(extract_detail(&source).variants.contains_key("trinket"));
}

#[test]
fn fusionne_les_frames_chronologiquement_et_refuse_les_metadonnees_incompletes() {
    let mut source = timeline(Vec::new());
    source["info"]["frames"] = json!([
        {"events":[skill(5,3)]},
        {"events":[skill(1,1),skill(3,2)]}
    ]);
    assert_eq!(
        extract_timeline(&source, "EUW1_SYNTHETIQUE", 1)
            .unwrap()
            .variants["skill_order"],
        vec![1, 2, 3]
    );
    source["info"]["participants"][1]["participantId"] = json!(1);
    assert!(extract_timeline(&source, "EUW1_SYNTHETIQUE", 1).is_err());
    let mut missing_events = timeline(vec![skill(1, 1)]);
    missing_events["info"]["frames"][0]["events"] = Value::Null;
    assert!(extract_timeline(&missing_events, "EUW1_SYNTHETIQUE", 1).is_err());
}

#[test]
fn annuler_un_unique_achat_produit_une_sequence_connue_vide() {
    let result = extract(vec![
        item("ITEM_PURCHASED", 1, 2003),
        json!({"type":"ITEM_UNDO","participantId":1,"timestamp":2,"beforeId":2003,"afterId":0}),
    ]);
    assert_eq!(result.variants["purchase_order"], Vec::<u32>::new());
    assert!(!result.variants.contains_key("final_items"));
    assert_eq!(result.item_events.len(), 2);
}

#[test]
fn un_type_de_point_inconnu_ou_un_evenement_sans_joueur_est_refuse() {
    for event in [
        json!({"type":"SKILL_LEVEL_UP","participantId":1,"timestamp":1,"skillSlot":1,"levelUpType":"AUTOMATIC"}),
        json!({"type":"ITEM_PURCHASED","timestamp":1,"itemId":2003}),
        json!({"type":"ITEM_UNDO","participantId":1,"timestamp":1,"beforeId":0,"afterId":0}),
        item("ITEM_PURCHASED", 1, 0),
    ] {
        assert!(extract_timeline(&timeline(vec![event]), "EUW1_SYNTHETIQUE", 1).is_err());
    }
}

#[test]
fn les_evenements_systeme_sans_joueur_ne_contaminent_pas_sa_sequence() {
    // participantId=0 est le système, observé dans les timelines réelles de la recette.
    let result = extract(vec![
        json!({"type":"SKILL_LEVEL_UP","participantId":0,"timestamp":1}),
        json!({"type":"ITEM_PURCHASED","participantId":0,"timestamp":2}),
        skill(3, 2),
        item("ITEM_PURCHASED", 4, 1055),
    ]);
    assert_eq!(result.variants["skill_order"], vec![2]);
    assert_eq!(result.variants["purchase_order"], vec![1055]);
    assert_eq!(result.skill_steps.len(), 1);
    assert_eq!(result.item_events.len(), 1);
    for id in [Value::Null, json!("0"), json!(-1), json!(0.5)] {
        let event = json!({"type":"ITEM_PURCHASED","participantId":id,"timestamp":1,"itemId":1055});
        assert!(extract_timeline(&timeline(vec![event]), "EUW1_SYNTHETIQUE", 1).is_err());
    }
}

#[test]
fn un_remboursement_sans_objet_preserve_les_sorts_sans_inventer_un_ordre_achats() {
    let r = extract(vec![
        skill(1, 2),
        item("ITEM_PURCHASED", 2, 1055),
        json!({"type":"ITEM_UNDO","participantId":1,"timestamp":3,"beforeId":0,"afterId":0,"goldGain":300}),
    ]);
    assert_eq!(r.variants["skill_order"], vec![2]);
    assert!(!r.variants.contains_key("purchase_order"));
    assert_eq!(r.item_events.len(), 1);
    assert_eq!(
        serde_json::to_value(&r).unwrap()["unidentified_item_undos"],
        1
    );
}

#[test]
fn les_achats_nets_gardent_leur_horodatage_pour_les_etapes() {
    let result = extract(vec![
        item("ITEM_PURCHASED", 1_000, 2003),
        item("ITEM_PURCHASED", 2_000, 1055),
        item("ITEM_PURCHASED", 3_000, 2003),
        json!({"type":"ITEM_UNDO","participantId":1,"timestamp":4_000,"beforeId":2003,"afterId":0,"goldGain":50}),
        item("ITEM_SOLD", 500_000, 1055),
    ]);
    // L'empreinte existante reste inchangée ; les étapes lisent la même séquence nette.
    assert_eq!(result.variants["purchase_order"], vec![2003, 1055]);
    let purchases = result.net_purchases.as_ref().unwrap();
    assert_eq!(
        purchases
            .iter()
            .map(|p| (p.item_id, p.timestamp_ms))
            .collect::<Vec<_>>(),
        vec![(2003, 1_000), (1055, 2_000)]
    );
    // Projection interne : jamais sérialisée avec la variante publiable.
    assert!(serde_json::to_value(&result)
        .unwrap()
        .get("net_purchases")
        .is_none());
}

#[test]
fn sans_ordre_net_fiable_aucune_etape_n_est_derivee() {
    let undo = extract(vec![
        item("ITEM_PURCHASED", 2, 1055),
        json!({"type":"ITEM_UNDO","participantId":1,"timestamp":3,"beforeId":0,"afterId":0,"goldGain":300}),
    ]);
    assert!(undo.net_purchases.is_none());
    assert!(extract(vec![skill(1, 1)]).net_purchases.is_none());
}
