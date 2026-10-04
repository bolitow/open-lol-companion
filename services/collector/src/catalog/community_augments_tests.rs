//! Augments Arena et Mayhem (#118) : fixture minimale tirée de la forme réelle de
//! `cherry-augments.json`, `augment-lists.json` et `cdragon/arena/{fr_fr,en_us}.json`
//! (CommunityDragon 16.19, relevée le 4 octobre 2026). Aucune statistique de performance :
//! seulement le catalogue statique.
use serde_json::{json, Value};

use std::sync::Arc;

use super::tests::{source, sources, MockTransport, VERSION};
use super::*;
use crate::catalog::{CatalogRecord, ValueStatus};

const ICON_ROOT: &str = "/lol-game-data/assets/ASSETS/UX";

fn entry(id: u64, technical: &str, name: &str, short: &str, icon: &str, rarity: &str) -> Value {
    json!({
        "id": id, "augmentNameId": technical, "nameTRA": name, "simpleNameTRA": short,
        "augmentSmallIconPath": format!("{ICON_ROOT}/{icon}"), "rarity": rarity,
    })
}

/// Fiche de l'export `cdragon/arena` : seuls `id`, `apiName`, `desc` portent le catalogue, le reste
/// (`calculations`, `dataValues`, `tooltip`, icônes) reprend la forme réelle du patch 16.19.
fn arena(id: u64, api_name: &str, name: &str, desc: &str, rarity: u64, data: Value) -> Value {
    json!({
        "apiName": api_name, "calculations": {}, "dataValues": data, "desc": desc,
        "iconLarge": "assets/ux/cherry/augments/icons/x_large.png",
        "iconSmall": "assets/ux/cherry/augments/icons/x_small.png",
        "id": id, "name": name, "rarity": rarity, "tooltip": "{{ Summary }}",
    })
}

/// Les cinq sources d'augments ; la liste de modes contient un augment par mode au moins. Seuls
/// 341 et 93 figurent dans l'export Arena (comme les 329 augments Mayhem absents du vrai fichier).
pub(super) fn augment_sources() -> Vec<CatalogSource> {
    let names = [
        (
            1205,
            "ARAM_ADAPt",
            "ADAPt",
            "Adaptation",
            "",
            "Cherry/Augments/Icons/ADAPt_small.png",
            "kSilver",
        ),
        (
            1327,
            "Adamant",
            "Adamant",
            "Inflexible",
            "",
            "Kiwi/Augments/Icons/Adamant_small.png",
            "kSilver",
        ),
        (
            2103,
            "ARAM_BangBang",
            "From Downtown",
            "Sniper explosif",
            "same",
            "Kiwi/Augments/Icons/QuestBangBang_small.png",
            "kGold",
        ),
        (
            341,
            "CraftingAugmentSlot",
            "Gain an Augment slot",
            "Vous gagnez un emplacement d'optimisation.",
            "",
            "Cherry/Augments/Icons/Crafting_Slot_small.png",
            "kEventChoice",
        ),
        (
            93,
            "WarmupRoutine",
            "Warmup Routine",
            "Échauffement",
            "",
            "Cherry/Augments/Icons/WarmupRoutine_small.png",
            "kSilver",
        ),
    ];
    let build = |french: bool| -> Value {
        names
            .iter()
            .map(|(id, technical, en, fr, short, icon, rarity)| {
                let name = if french { fr } else { en };
                let short = if *short == "same" { name } else { "" };
                entry(*id, technical, name, short, icon, rarity)
            })
            .collect()
    };
    let arena_file = |french: bool| -> Value {
        let (crafting_name, crafting, warmup_name, warmup) = if french {
            (
                "Vous gagnez un emplacement d'optimisation.",
                "Vous déverrouillez votre <keywordMajor>@TooltipSlotToUnlock@e %i:Augment% emplacement d'optimisation</keywordMajor>.<br><br>Dans @RoundsUntilFreeAugment@ manches, vous gagnez une <keywordMajor>%i:Augment% optimisation argent</keywordMajor>.",
                "Échauffement",
                "Vous obtenez le sort d'invocateur <spellName>Échauffement</spellName>.<br><br>Vous canalisez une danse qui augmente vos dégâts de 2% par seconde (jusqu'à @MaxStacks@% max).",
            )
        } else {
            (
                "Gain an Augment slot",
                "Unlock your <keywordMajor>@TooltipSlotToUnlock@th %i:Augment% Augment Slot</keywordMajor>.<br><br>In @RoundsUntilFreeAugment@ rounds gain a <keywordMajor>%i:Augment% Silver Augment</keywordMajor>.",
                "Warmup Routine",
                "Gain the <spellName>Warmup Routine</spellName> Summoner Spell.<br><br>Channel your inner dancer to increase your damage by 2% per second, up to @MaxStacks@% max.",
            )
        };
        json!({"augments": [
            arena(341, "CraftingAugmentSlot", crafting_name, crafting, 4,
                json!({"RoundsUntilFreeAugment": [4.0, 4.0], "SlotsToGrant": [1.0, 1.0]})),
            arena(93, "WarmupRoutine", warmup_name, warmup, 0,
                json!({"MaxStacks": [20.0, 20.0, 40.0]})),
        ]})
    };
    vec![
        source("fr_FR/cherry-augments.json", build(true)),
        source("en_US/cherry-augments.json", build(false)),
        source("fr_FR/arena-augments.json", arena_file(true)),
        source("en_US/arena-augments.json", arena_file(false)),
        source(
            "augment-lists.json",
            json!([
                {"modeName": "CHERRY", "augmentList": ["Maps/ModeSpecificData/Augments/Adamant"]},
                {"modeName": "KIWI", "augmentList": [
                    "Maps/ModeSpecificData/Augments/ARAM_ADAPt",
                    "Maps/ModeSpecificData/Augments/Adamant",
                    "Maps/ModeSpecificData/Augments/ARAM_BangBang"]},
                {"modeName": "KIWI_JADE", "augmentList": [
                    "Maps/ModeSpecificData/Augments/ARAM_ADAPt",
                    "Maps/ModeSpecificData/Augments/Adamant"]},
            ]),
        ),
    ]
}

fn augments(records: &[CatalogRecord], locale: &str) -> Vec<CatalogRecord> {
    let mut found: Vec<_> = records
        .iter()
        .filter(|r| r.kind == "augment" && r.locale == locale)
        .cloned()
        .collect();
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found
}

fn projected(input: &[CatalogSource]) -> Vec<CatalogRecord> {
    let mut records = vec![];
    enrich(VERSION, &mut records, input).unwrap();
    records
}

fn find<'a>(records: &'a [CatalogRecord], locale: &str, id: &str) -> &'a CatalogRecord {
    records
        .iter()
        .find(|r| r.kind == "augment" && r.locale == locale && r.id == id)
        .unwrap()
}

#[test]
fn chaque_augment_devient_une_fiche_par_langue_avec_nom_rarete_et_icone() {
    let records = projected(&sources());
    assert_eq!(augments(&records, "fr_FR").len(), 5);
    assert_eq!(augments(&records, "en_US").len(), 5);
    let fr = find(&records, "fr_FR", "1205");
    let en = find(&records, "en_US", "1205");
    assert_eq!(
        (fr.namespace.as_str(), en.namespace.as_str()),
        ("standard", "standard")
    );
    assert_eq!(
        (fr.name.as_str(), en.name.as_str()),
        ("Adaptation", "ADAPt")
    );
    assert_eq!(fr.fields["technical_id"].value, "ARAM_ADAPt");
    assert_eq!(fr.fields["technical_id"].status, ValueStatus::Verified);
    assert_eq!(fr.fields["rarity"].value, "kSilver");
    assert_eq!(fr.fields["rarity"].status, ValueStatus::Verified);
    assert_eq!(fr.fields["rarity"].sources[0].pointer, "/0/rarity");
    assert_eq!(fr.fields["community_name"].value, "Adaptation");
    assert_eq!(fr.fields["community_id"].value, "1205");
    assert_eq!(
        fr.icon.as_deref(),
        Some("https://raw.communitydragon.org/16.19/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/adapt_small.png")
    );
    assert_eq!(
        fr.fields["community_icon_path"].value,
        "/lol-game-data/assets/ASSETS/UX/Cherry/Augments/Icons/ADAPt_small.png"
    );
    assert_eq!(
        find(&records, "en_US", "341").fields["rarity"].value,
        "kEventChoice"
    );
    // Le nom court n'existe que lorsque la source le donne.
    assert!(!fr.fields.contains_key("community_short_name"));
    assert_eq!(
        find(&records, "fr_FR", "2103").fields["community_short_name"].value,
        "Sniper explosif"
    );
}

#[test]
fn la_description_vient_de_l_export_arena_dans_chaque_langue() {
    let records = projected(&sources());
    let fr = find(&records, "fr_FR", "93");
    let en = find(&records, "en_US", "93");
    assert_eq!(
        fr.description.as_deref(),
        Some("Vous obtenez le sort d'invocateur Échauffement . Vous canalisez une danse qui augmente vos dégâts de 2% par seconde (jusqu'à @MaxStacks@% max).")
    );
    assert!(en
        .description
        .as_deref()
        .unwrap()
        .starts_with("Gain the Warmup Routine Summoner Spell."));
    for record in [fr, en] {
        // Texte brut : aucun balisage ; le placeholder reste visible, jamais résolu ni inventé.
        let text = record.description.as_deref().unwrap();
        assert!(
            !text.contains('<') && text.contains("@MaxStacks@"),
            "{text}"
        );
        assert!(record
            .coverage
            .issues
            .contains(&"unresolved_placeholder:description".to_string()));
        assert!(!record
            .coverage
            .issues
            .contains(&"missing:description".to_string()));
        let field = &record.fields["community_description"];
        assert_eq!(field.value, json!(text));
        assert_eq!(field.status, ValueStatus::Descriptive);
        // Le pointeur vise la fiche d'augment de l'export (index 1), pas celle de cherry-augments.
        assert_eq!(field.sources.len(), 1);
        assert_eq!(field.sources[0].pointer, "/augments/1/desc");
        assert_eq!(
            field.sources[0].source_id,
            format!("{}/arena-augments.json", record.locale)
        );
    }
    // Les paramètres chiffrés de l'export ne sont pas interprétés : ils restent tracés non mappés.
    let unmapped = &fr.coverage.unmapped_fields;
    assert!(unmapped
        .iter()
        .any(|f| f == "fr_FR/arena-augments.json:/augments/1/dataValues/MaxStacks/0"));
    assert!(unmapped
        .iter()
        .any(|f| f.ends_with("/augments/1/iconLarge")));
    assert!(!unmapped.iter().any(|f| f.ends_with("/desc")));
    assert!(fr.coverage.source_fields > 6);
    assert_eq!(
        fr.coverage.source_fields as usize,
        fr.coverage.normalized_fields as usize + unmapped.len()
    );
    // Un texte sans balisage de la source (« {{ clé }} » non résolu) est signalé comme tel.
    let crafting = find(&records, "en_US", "341");
    assert!(crafting
        .description
        .as_deref()
        .unwrap()
        .contains("@RoundsUntilFreeAugment@"));
    // Le nom et la rareté restent ceux de cherry-augments ; l'export n'ajoute aucun champ.
    assert_eq!(fr.name, "Échauffement");
    assert_eq!(fr.fields["rarity"].value, "kSilver");
    assert!(!fr.fields.contains_key("tooltip"));
}

#[test]
fn un_augment_absent_de_l_export_arena_n_a_pas_de_description_inventee() {
    let records = projected(&sources());
    for locale in ["fr_FR", "en_US"] {
        for id in ["1205", "1327", "2103"] {
            let record = find(&records, locale, id);
            assert_eq!(record.description, None, "{locale} {id}");
            assert!(!record.fields.contains_key("community_description"));
            assert!(record
                .coverage
                .issues
                .contains(&"missing:description".to_string()));
            assert!(
                record.coverage.unmapped_fields.is_empty(),
                "{:?}",
                record.coverage
            );
            assert_eq!(record.coverage.source_fields, 6);
            assert_eq!(record.coverage.normalized_fields, 6);
        }
        for id in ["93", "341"] {
            assert!(find(&records, locale, id).description.is_some());
        }
    }
}

#[test]
fn une_description_vide_est_une_absence_et_non_un_texte() {
    let mut input = sources();
    let index = input
        .iter()
        .position(|s| s.key == "en_US/arena-augments.json")
        .unwrap();
    input[index].data["augments"][1]["desc"] = json!("");
    let records = projected(&input);
    let record = find(&records, "en_US", "93");
    assert_eq!(record.description, None);
    assert!(!record.fields.contains_key("community_description"));
    assert!(record
        .coverage
        .issues
        .contains(&"missing:description".to_string()));
    assert!(find(&records, "fr_FR", "93").description.is_some());
}

#[test]
fn un_jeton_d_icone_seul_est_un_placeholder_non_resolu_et_n_est_jamais_resolu() {
    // Forme de l'id 238 du patch 16.19 : ni `@…@` ni `{{ … }}`, seulement le jeton d'icône du client.
    let mut input = sources();
    let index = input
        .iter()
        .position(|s| s.key == "fr_FR/arena-augments.json")
        .unwrap();
    input[index].data["augments"][1]["desc"] =
        json!("Vous gagnez une %i:Augment% optimisation or aléatoire.");
    let records = projected(&input);
    let record = find(&records, "fr_FR", "93");
    let text = "Vous gagnez une %i:Augment% optimisation or aléatoire.";
    assert_eq!(record.description.as_deref(), Some(text));
    assert_eq!(record.fields["community_description"].value, json!(text));
    assert!(record
        .coverage
        .issues
        .contains(&"unresolved_placeholder:description".to_string()));
    assert!(!record
        .coverage
        .issues
        .contains(&"missing:description".to_string()));
    // Les autres jetons du client (`%i:StatAnvil%`, `%i:AugmentLevel%`) sont traités de même.
    for token in ["%i:StatAnvil%", "%i:AugmentLevel%"] {
        input[index].data["augments"][1]["desc"] = json!(format!("Gagnez {token} une forge."));
        let records = projected(&input);
        assert!(find(&records, "fr_FR", "93")
            .coverage
            .issues
            .contains(&"unresolved_placeholder:description".to_string()));
    }
    // Un texte sans placeholder n'est pas signalé à tort.
    input[index].data["augments"][1]["desc"] = json!("Gagnez 100% de vitesse.");
    let records = projected(&input);
    assert!(!find(&records, "fr_FR", "93")
        .coverage
        .issues
        .contains(&"unresolved_placeholder:description".to_string()));
}

#[test]
fn les_modes_viennent_des_listes_de_la_source_et_un_augment_hors_liste_est_conserve() {
    let records = projected(&sources());
    let adamant = find(&records, "en_US", "1327");
    assert_eq!(
        adamant.fields["modes"].value,
        json!(["CHERRY", "KIWI", "KIWI_JADE"])
    );
    assert_eq!(adamant.fields["modes"].status, ValueStatus::Derived);
    let pointers: Vec<_> = adamant.fields["modes"]
        .sources
        .iter()
        .map(|s| s.pointer.as_str())
        .collect();
    assert_eq!(
        pointers,
        ["/0/augmentList", "/1/augmentList", "/2/augmentList"]
    );
    assert_eq!(
        find(&records, "en_US", "2103").fields["modes"].value,
        json!(["KIWI"])
    );
    let unlisted = find(&records, "fr_FR", "341");
    assert_eq!(unlisted.fields["modes"].value, json!([]));
    assert_eq!(unlisted.fields["modes"].status, ValueStatus::Derived);
    assert_eq!(unlisted.fields["modes"].sources[0].pointer, "");
}

#[test]
fn le_catalogue_d_augments_ne_porte_aucune_statistique_de_performance() {
    let records = projected(&sources());
    for record in augments(&records, "en_US")
        .iter()
        .chain(&augments(&records, "fr_FR"))
    {
        assert!(record.stats.is_empty() && record.effects.is_empty());
        for key in record.fields.keys() {
            let key = key.to_lowercase();
            assert!(
                !["win", "pick", "tier", "popular", "rate", "games"]
                    .iter()
                    .any(|w| key.contains(w)),
                "champ interdit : {key}"
            );
        }
    }
}

#[test]
fn une_rarete_inconnue_ou_non_textuelle_reste_non_supportee_et_visible() {
    for raw in [json!("kLegendary"), json!(3), Value::Null] {
        let mut input = sources();
        let index = input
            .iter()
            .position(|s| s.key == "en_US/cherry-augments.json")
            .unwrap();
        input[index].data[0]["rarity"] = raw.clone();
        let records = projected(&input);
        let rarity = &find(&records, "en_US", "1205").fields["rarity"];
        assert_eq!(rarity.value, raw);
        assert_eq!(
            rarity.status,
            if raw.is_null() {
                ValueStatus::Missing
            } else {
                ValueStatus::Unsupported
            }
        );
        if !raw.is_null() {
            let record = find(&records, "en_US", "1205");
            assert!(record
                .coverage
                .issues
                .contains(&"unsupported:fields.rarity".to_string()));
            assert!(record
                .coverage
                .unmapped_fields
                .iter()
                .any(|f| f.ends_with("/0/rarity")));
        }
    }
}

#[test]
fn une_icone_hors_du_schema_public_n_est_jamais_une_url() {
    let mut input = sources();
    let index = input
        .iter()
        .position(|s| s.key == "fr_FR/cherry-augments.json")
        .unwrap();
    input[index].data[0]["augmentSmallIconPath"] = json!("/lol-game-data/assets/../secret.png");
    input[index].data[1]["augmentSmallIconPath"] = json!("https://example.test/a.png");
    let records = projected(&input);
    assert_eq!(find(&records, "fr_FR", "1205").icon, None);
    assert_eq!(find(&records, "fr_FR", "1327").icon, None);
    assert!(find(&records, "en_US", "1205").icon.is_some());
}

#[test]
fn le_texte_du_nom_est_nettoye_comme_les_autres_fiches() {
    let mut input = sources();
    let index = input
        .iter()
        .position(|s| s.key == "en_US/cherry-augments.json")
        .unwrap();
    input[index].data[0]["nameTRA"] = json!("<b>ADAPt</b>");
    let records = projected(&input);
    assert_eq!(find(&records, "en_US", "1205").name, "ADAPt");
}

#[test]
fn une_source_d_augment_incoherente_est_refusee_sans_modifier_les_fiches() {
    for change in 0..17 {
        let mut input = sources();
        let fr = input
            .iter()
            .position(|s| s.key == "fr_FR/cherry-augments.json")
            .unwrap();
        let lists = input
            .iter()
            .position(|s| s.key == "augment-lists.json")
            .unwrap();
        let arena_fr = input
            .iter()
            .position(|s| s.key == "fr_FR/arena-augments.json")
            .unwrap();
        let arena_en = input
            .iter()
            .position(|s| s.key == "en_US/arena-augments.json")
            .unwrap();
        match change {
            // Identifiants différents entre les deux langues.
            0 => input[fr].data[0]["id"] = json!(9999),
            // Même identifiant mais identité technique différente.
            1 => input[fr].data[0]["augmentNameId"] = json!("Other"),
            // Identifiant dupliqué.
            2 => input[fr].data[1]["id"] = json!(1205),
            // Nom absent.
            3 => input[fr].data[0]["nameTRA"] = Value::Null,
            // Une liste cite un augment inconnu.
            4 => {
                input[lists].data[0]["augmentList"] =
                    json!(["Maps/ModeSpecificData/Augments/Ghost"])
            }
            // Chemin de liste hors du dossier attendu.
            5 => input[lists].data[0]["augmentList"] = json!(["Other/Adamant"]),
            // Deux listes portent le même mode.
            6 => input[lists].data[1]["modeName"] = json!("CHERRY"),
            // Liste de modes vide.
            7 => input[lists].data = json!([]),
            // Un augment cité deux fois dans la même liste.
            8 => {
                input[lists].data[1]["augmentList"] = json!([
                    "Maps/ModeSpecificData/Augments/Adamant",
                    "Maps/ModeSpecificData/Augments/Adamant"
                ])
            }
            // Export Arena : identifiants différents entre les deux langues.
            9 => input[arena_fr].data["augments"][0]["id"] = json!(9999),
            // Export Arena : même identifiant, `apiName` différent entre les deux langues.
            10 => input[arena_fr].data["augments"][0]["apiName"] = json!("Other"),
            // Export Arena : id et `apiName` concordent entre langues mais pas avec cherry-augments.
            11 => {
                for index in [arena_fr, arena_en] {
                    input[index].data["augments"][0]["apiName"] = json!("Other");
                }
            }
            // Export Arena : un augment inconnu de cherry-augments.
            12 => {
                for index in [arena_fr, arena_en] {
                    input[index].data["augments"][0]["id"] = json!(9999);
                }
            }
            // Export Arena : identifiant dupliqué.
            13 => input[arena_en].data["augments"][1]["id"] = json!(341),
            // Export Arena : description qui n'est pas du texte.
            14 => input[arena_en].data["augments"][0]["desc"] = json!(7),
            // Export Arena : liste vide, ou racine sans `augments`.
            15 => input[arena_en].data = json!({"augments": []}),
            _ => input[arena_en].data = json!([]),
        }
        let mut records = vec![];
        assert!(
            enrich(VERSION, &mut records, &input).is_err(),
            "cas {change}"
        );
        assert!(records.is_empty(), "cas {change}");
    }
}

#[test]
fn une_archive_sans_augment_se_rejoue_encore_mais_un_jeu_partiel_est_refuse() {
    let complete = sources();
    let legacy: Vec<_> = complete
        .iter()
        .filter(|s| !s.key.contains("augment"))
        .cloned()
        .collect();
    assert_eq!(legacy.len(), 8);
    let records = projected(&legacy);
    assert!(records.iter().any(|r| r.kind == "item"));
    assert!(records.iter().all(|r| r.kind != "augment"));
    for missing in [
        "fr_FR/cherry-augments.json",
        "en_US/cherry-augments.json",
        "augment-lists.json",
        "fr_FR/arena-augments.json",
        "en_US/arena-augments.json",
    ] {
        let partial: Vec<_> = complete
            .iter()
            .filter(|s| s.key != missing)
            .cloned()
            .collect();
        let mut records = vec![];
        assert!(
            enrich(VERSION, &mut records, &partial).is_err(),
            "{missing}"
        );
        assert!(records.is_empty());
    }
}

#[test]
fn les_sources_d_augment_sont_epinglees_au_patch_et_a_leur_url() {
    for change in 0..6 {
        let mut input = sources();
        let key = if change < 3 {
            "augment-lists.json"
        } else {
            "fr_FR/arena-augments.json"
        };
        let index = input.iter().position(|s| s.key == key).unwrap();
        match change {
            0 | 3 => input[index].version = "16.18.1".into(),
            1 => input[index].url = "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/augment-lists.json".into(),
            // L'export Arena est épinglé au patch comme les autres ressources, jamais à `latest`.
            4 => input[index].url = "https://raw.communitydragon.org/latest/cdragon/arena/fr_fr.json".into(),
            5 => input[index].locale = Some("en_US".into()),
            _ => input[index].locale = Some("fr_FR".into()),
        }
        let mut records = vec![];
        assert!(
            enrich(VERSION, &mut records, &input).is_err(),
            "cas {change}"
        );
    }
}

#[tokio::test]
async fn le_telechargement_demande_les_cinq_ressources_d_augments_et_echoue_si_une_manque() {
    let transport = MockTransport::valid();
    let fetched = fetch_sources_with(VERSION, transport.clone())
        .await
        .unwrap();
    for key in [
        "fr_FR/cherry-augments.json",
        "en_US/cherry-augments.json",
        "augment-lists.json",
        "fr_FR/arena-augments.json",
        "en_US/arena-augments.json",
    ] {
        assert!(fetched.iter().any(|s| s.key == key), "{key}");
    }
    {
        let requested = transport.requested.lock().unwrap();
        for url in [
            "https://raw.communitydragon.org/16.19/cdragon/arena/fr_fr.json",
            "https://raw.communitydragon.org/16.19/cdragon/arena/en_us.json",
        ] {
            assert!(requested.iter().any(|r| r == url), "{url}");
        }
    }
    for missing in [
        "cherry-augments.json",
        "augment-lists.json",
        "arena/fr_fr.json",
        "arena/en_us.json",
    ] {
        let mut responses = (*transport.responses).clone();
        responses.retain(|url, _| !url.ends_with(missing));
        let broken = MockTransport {
            responses: Arc::new(responses),
            ..MockTransport::valid()
        };
        assert!(
            fetch_sources_with(VERSION, broken).await.is_err(),
            "{missing}"
        );
    }
}
