use super::download::download_release;
use super::model::{select_versions, validate_document, Kind};
use super::test_support::{response, FakeCdn};
use super::transport::fetch_json;
use super::StaticError;
use serde_json::{json, Value};

#[tokio::test(start_paused = true)]
async fn retente_les_erreurs_temporaires_et_borne_les_echecs_sans_exposer_le_corps() {
    let fake = FakeCdn::new();
    let url = "https://ddragon.leagueoflegends.com/api/versions.json";
    fake.script(
        url,
        vec![
            response(503, json!({"internal":"private"})),
            response(200, json!(["16.19.1"])),
        ],
    );
    assert_eq!(
        fetch_json(fake.clone(), url.into()).await.unwrap(),
        json!(["16.19.1"])
    );
    fake.deny("versions.json");
    assert_eq!(
        fetch_json(fake.clone(), url.into()).await.unwrap_err(),
        StaticError::Network
    );
    assert_eq!(fake.calls().len(), 5);
}

#[tokio::test]
async fn telecharge_un_bundle_complet_bilingue_avec_namespace_classic() {
    let bundle = download_release(FakeCdn::new(), "16.19.1").await.unwrap();
    assert_eq!(bundle.patch, "16.19");
    assert_eq!(
        (bundle.champion_count, bundle.classic_champion_count),
        (1, 1)
    );
    assert_eq!(bundle.documents.len(), 18);
    let encoded = serde_json::to_value(&bundle).unwrap();
    assert_eq!(
        encoded["classic_asset_base"],
        "https://ddragon.leagueoflegends.com/cdn/16.19.1/img/mode/classic/"
    );
    assert_eq!(
        encoded["rune_asset_base"],
        "https://ddragon.leagueoflegends.com/cdn/img/"
    );
    for locale in ["fr_FR", "en_US"] {
        assert!(bundle
            .documents
            .contains_key(&format!("{locale}/champion/Aatrox.json")));
        assert!(bundle
            .documents
            .contains_key(&format!("{locale}/mode/classic/champion/Jade_Ahri.json")));
    }
}

#[test]
fn choisit_deux_patchs_distincts_sans_prerelease_ni_confusion_avec_le_numero_marketing() {
    let versions = json!([
        "16.20.1",
        "16.19.2",
        "16.19.1",
        "16.18.2",
        "16.18.1",
        "16.9.1",
        "lolpatch_7.20"
    ]);
    let realm = json!({"v":"16.19.1", "n":{"rune":"7.23.1"}});
    assert_eq!(
        select_versions(&versions, &realm, 2).unwrap(),
        ["16.19.2", "16.18.2"]
    );
    assert_eq!(
        select_versions(&json!(["16.9.1", "16.18.1"]), &realm, 2),
        Err(StaticError::InvalidManifest)
    );
}

#[test]
fn refuse_manifeste_incomplet_versions_invalides_et_compte_hors_limites() {
    let realm = json!({"v":"16.19.1"});
    for versions in [
        json!([]),
        json!(["16.19.1"]),
        json!(["16.19.1", "../private"]),
        json!(["16.19.1", 123]),
    ] {
        assert!(select_versions(&versions, &realm, 2).is_err());
    }
    assert_eq!(
        select_versions(&json!([]), &realm, 0),
        Err(StaticError::InvalidCount)
    );
    assert_eq!(
        select_versions(&json!([]), &realm, 11),
        Err(StaticError::InvalidCount)
    );
    assert!(select_versions(&json!(["16.19.1"]), &json!({"v":"26.19"}), 1).is_err());
}

#[test]
fn rejette_document_vide_version_etrangere_et_identifiant_incoherent() {
    for kind in [
        Kind::ChampionList,
        Kind::Item,
        Kind::Summoner,
        Kind::Map,
        Kind::ProfileIcon,
        Kind::Runes,
        Kind::Queues,
        Kind::Maps,
        Kind::Modes,
        Kind::GameTypes,
    ] {
        assert_eq!(
            validate_document(&kind, "16.19.1", &Value::Null),
            Err(StaticError::InvalidDocument)
        );
    }
    let champion = json!({"type":"champion", "version":"16.18.1", "data":{"Aatrox":{"id":"Aatrox","key":"266","name":"Aatrox"}}});
    assert!(validate_document(&Kind::ChampionList, "16.19.1", &champion).is_err());
    let mismatch = json!({"type":"champion", "version":"16.19.1", "data":{"Aatrox":{"id":"Ahri","key":"266","name":"Aatrox"}}});
    assert!(validate_document(&Kind::ChampionList, "16.19.1", &mismatch).is_err());
}

#[test]
fn exige_quatre_sorts_et_un_passif_pour_chaque_champion() {
    let kind = Kind::Champion {
        id: "Aatrox".into(),
        key: "266".into(),
    };
    let detail = json!({"type":"champion","version":"16.19.1","data":{"Aatrox":{"id":"Aatrox","key":"266","name":"Aatrox","spells":[],"passive":null}}});
    assert_eq!(
        validate_document(&kind, "16.19.1", &detail),
        Err(StaticError::InvalidDocument)
    );
}

#[test]
fn accepte_le_format_reel_des_runes_sans_enveloppe_de_version() {
    let runes = json!([{"id":8000,"key":"Precision","name":"Précision","icon":"perk-images/Styles/7201_Precision.png","slots":[{"runes":[{"id":8005,"key":"PressTheAttack","name":"Attaque soutenue","icon":"perk-images/Styles/Precision/PressTheAttack/PressTheAttack.png","shortDesc":"description","longDesc":"description longue"}]}]}]);
    assert!(validate_document(&Kind::Runes, "16.19.1", &runes).is_ok());
    assert!(validate_document(&Kind::Runes, "16.19.1", &json!([])).is_err());
}

#[test]
fn conserve_les_identifiants_reserves_riot_dont_le_libelle_est_vide() {
    // Présents dans 16.19.1 et 16.18.1 en FR/EN : objet 2008 et carte 453.
    let item = json!({"type":"item","version":"16.19.1","data":{"2008":{"name":"","image":{"full":"2008.png"}}}});
    let map = json!({"type":"map","version":"16.19.1","data":{"453":{"MapId":"453","MapName":"","image":{"full":"map453.png"}}}});
    assert!(validate_document(&Kind::Item, "16.19.1", &item).is_ok());
    assert!(validate_document(&Kind::Map, "16.19.1", &map).is_ok());
}

#[test]
fn accepte_les_identifiants_icones_riot_numeriques_ou_textuels_sans_accepter_un_id_absent() {
    // Le catalogue officiel mélange 0 (nombre) et "50" (chaîne), dans les deux langues.
    let icons = json!({"type":"profileicon","version":"16.19.1","data":{
        "0":{"id":0,"image":{"full":"0.png"}},
        "50":{"id":"50","image":{"full":"50.png"}}
    }});
    assert!(validate_document(&Kind::ProfileIcon, "16.19.1", &icons).is_ok());
    for id in [Value::Null, json!(-1), json!("invalid")] {
        let invalid = json!({"type":"profileicon","version":"16.19.1","data":{"invalid":{"id":id,"image":{"full":"0.png"}}}});
        assert!(validate_document(&Kind::ProfileIcon, "16.19.1", &invalid).is_err());
    }
}

#[tokio::test]
async fn refuse_json_malforme_et_ne_retente_pas_un_404() {
    let url = "https://ddragon.leagueoflegends.com/api/versions.json";
    let fake = FakeCdn::new();
    fake.script(
        url,
        vec![super::StaticResponse {
            status: 200,
            body: b"{invalid".to_vec(),
        }],
    );
    assert_eq!(
        fetch_json(fake.clone(), url.into()).await,
        Err(StaticError::InvalidDocument)
    );
    assert_eq!(fake.calls().len(), 1);
    fake.script(url, vec![response(404, json!({"private":"private"}))]);
    assert_eq!(
        fetch_json(fake.clone(), url.into()).await,
        Err(StaticError::Network)
    );
    assert_eq!(fake.calls().len(), 2);
}
