//! Collection du compte courant ; la clé de compte reste exclusivement dans Rust.

use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use thiserror::Error;

use crate::account::{ACCOUNT_ENDPOINT, REGION_ENDPOINT};
use crate::{LcuAccount, LcuClient};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkinOwnership {
    Owned,
    Temporary,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CollectionSkin {
    pub id: u32,
    pub champion_id: u32,
    pub name: String,
    pub ownership: SkinOwnership,
    pub tile_url: Option<String>,
    pub splash_url: Option<String>,
    pub obtainable: Option<bool>,
    pub rarity: Option<String>,
    pub series_ids: Vec<u32>,
}

// Ne pas dériver Serialize : account_key est réservé au stockage natif des souhaits.
pub struct CollectionSnapshot {
    pub account_key: String,
    pub skins: Vec<CollectionSkin>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionError {
    #[error("collection indisponible")]
    Unavailable,
    #[error("réponse de collection invalide")]
    InvalidResponse,
    #[error("le compte connecté a changé")]
    AccountChanged,
}

/// Lecture seule du compte attendu. Le producteur doit également annuler la requête
/// lors d'une déconnexion : la LCU ne fournit pas de snapshot atomique.
pub async fn read_collection(
    client: &LcuClient,
    expected: &LcuAccount,
) -> Result<CollectionSnapshot, CollectionError> {
    tokio::time::timeout(Duration::from_secs(15), async {
        let id = read_identity(client, expected).await?;
        // Schéma et réponse locale vérifiés le 3 octobre 2026, sans conserver d'identité :
        // https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json
        let path = format!("/lol-champions/v1/inventories/{id}/skins-minimal");
        let value: Value = client
            .get_json(&path)
            .await
            .map_err(|_| CollectionError::Unavailable)?;
        let skins = parse_skins(&value)?;
        // Catalogue du même client : exclut les identifiants de modes auxiliaires,
        // chromas et apparences de base, sans déduire leur rôle du nom ni d'un seuil.
        // Chemin game-data vérifié publiquement et par GET local le 3 octobre 2026.
        let catalog: Value = client
            .get_json("/lol-game-data/assets/v1/skins.json")
            .await
            .map_err(|_| CollectionError::Unavailable)?;
        let skins = retain_catalog_skins(skins, &catalog)?;
        if read_identity(client, expected).await? != id {
            return Err(CollectionError::AccountChanged);
        }
        Ok(CollectionSnapshot {
            account_key: format!("{}-{id}", expected.platform),
            skins,
        })
    })
    .await
    .map_err(|_| CollectionError::Unavailable)?
}

async fn read_identity(client: &LcuClient, expected: &LcuAccount) -> Result<u64, CollectionError> {
    let first: Value = client
        .get_json(ACCOUNT_ENDPOINT)
        .await
        .map_err(|_| CollectionError::Unavailable)?;
    let region: Value = client
        .get_json(REGION_ENDPOINT)
        .await
        .map_err(|_| CollectionError::Unavailable)?;
    let last: Value = client
        .get_json(ACCOUNT_ENDPOINT)
        .await
        .map_err(|_| CollectionError::Unavailable)?;
    let before = LcuAccount::parse(&first, &region).ok_or(CollectionError::InvalidResponse)?;
    let after = LcuAccount::parse(&last, &region).ok_or(CollectionError::InvalidResponse)?;
    let id = |value: &Value| {
        value
            .get("summonerId")
            .and_then(Value::as_u64)
            .filter(|id| *id > 0)
    };
    let before_id = id(&first).ok_or(CollectionError::InvalidResponse)?;
    let after_id = id(&last).ok_or(CollectionError::InvalidResponse)?;
    if before != *expected || after != *expected || before_id != after_id {
        return Err(CollectionError::AccountChanged);
    }
    Ok(after_id)
}

fn parse_skins(value: &Value) -> Result<Vec<CollectionSkin>, CollectionError> {
    let entries = value
        .as_array()
        .filter(|entries| entries.len() <= 10_000)
        .ok_or(CollectionError::InvalidResponse)?;
    let mut seen = HashMap::with_capacity(entries.len());
    let mut skins = Vec::with_capacity(entries.len());
    let positive_id = |value: Option<&Value>| {
        value
            .and_then(Value::as_u64)
            .and_then(|id| u32::try_from(id).ok())
            .filter(|id| *id > 0)
    };
    for entry in entries {
        let id = positive_id(entry.get("id")).ok_or(CollectionError::InvalidResponse)?;
        let champion_id =
            positive_id(entry.get("championId")).ok_or(CollectionError::InvalidResponse)?;
        if id / 1000 != champion_id {
            return Err(CollectionError::InvalidResponse);
        }
        if let Some(previous) = seen.insert(id, entry) {
            if previous != entry {
                return Err(CollectionError::InvalidResponse);
            }
            continue;
        }
        let is_base = entry
            .get("isBase")
            .and_then(Value::as_bool)
            .ok_or(CollectionError::InvalidResponse)?;
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| name.len() <= 512 && !name.chars().any(char::is_control))
            // Le client peut contenir des espaces de bord dans un nom public valide.
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .ok_or(CollectionError::InvalidResponse)?;
        if is_base {
            continue;
        }
        skins.push(CollectionSkin {
            id,
            champion_id,
            name: name.into(),
            ownership: ownership(entry.get("ownership")),
            tile_url: asset_url(entry.get("tilePath")),
            splash_url: asset_url(entry.get("splashPath")),
            obtainable: entry.get("stillObtainable").and_then(Value::as_bool),
            rarity: None,
            series_ids: Vec::new(),
        });
    }
    Ok(skins)
}

fn ownership(value: Option<&Value>) -> SkinOwnership {
    let Some(value) = value.and_then(Value::as_object) else {
        return SkinOwnership::Unknown;
    };
    let flags = [
        value.get("loyaltyReward"),
        value.get("xboxGPReward"),
        value.get("freeToPlayReward"),
        value.get("rental").and_then(|rental| rental.get("rented")),
    ];
    if flags
        .iter()
        .any(|flag| flag.and_then(Value::as_bool) == Some(true))
    {
        return SkinOwnership::Temporary;
    }
    if value
        .get("rental")
        .is_some_and(|rental| !rental.is_null() && !rental.is_object())
        || flags
            .iter()
            .any(|flag| flag.is_some_and(|flag| !flag.is_null() && !flag.is_boolean()))
    {
        return SkinOwnership::Unknown;
    }
    match value.get("owned").and_then(Value::as_bool) {
        Some(true) => SkinOwnership::Owned,
        Some(false) => SkinOwnership::Missing,
        None => SkinOwnership::Unknown,
    }
}

fn asset_url(value: Option<&Value>) -> Option<String> {
    let path = value?.as_str()?.strip_prefix("/lol-game-data/assets/")?;
    if path.len() > 1024
        || !path.starts_with("ASSETS/")
        || !path.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        })
    {
        return None;
    }
    let path = path.to_ascii_lowercase();
    if ![".jpg", ".jpeg", ".png", ".webp"]
        .iter()
        .any(|extension| path.ends_with(extension))
    {
        return None;
    }
    // Mapping officiel de CommunityDragon : chemin game-data sans préfixe, en minuscules.
    // https://communitydragon.org/documentation/assets (consulté le 3 octobre 2026).
    Some(format!(
        "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/{path}"
    ))
}

fn retain_catalog_skins(
    skins: Vec<CollectionSkin>,
    catalog: &Value,
) -> Result<Vec<CollectionSkin>, CollectionError> {
    let entries = catalog
        .as_object()
        .filter(|entries| !entries.is_empty() && entries.len() <= 10_000)
        .ok_or(CollectionError::InvalidResponse)?;
    let mut ids = HashSet::with_capacity(entries.len());
    for (key, entry) in entries {
        let classification = entry
            .get("skinClassification")
            .and_then(Value::as_str)
            .ok_or(CollectionError::InvalidResponse)?;
        if classification != "kChampion" {
            continue;
        }
        let id = entry
            .get("id")
            .and_then(Value::as_u64)
            .and_then(|id| u32::try_from(id).ok())
            .filter(|id| *id > 0)
            .ok_or(CollectionError::InvalidResponse)?;
        if key != &id.to_string() {
            return Err(CollectionError::InvalidResponse);
        }
        let is_base = entry
            .get("isBase")
            .and_then(Value::as_bool)
            .ok_or(CollectionError::InvalidResponse)?;
        if !is_base {
            ids.insert(id);
        }
    }
    if ids.is_empty() {
        return Err(CollectionError::InvalidResponse);
    }
    Ok(skins
        .into_iter()
        .filter(|skin| ids.contains(&skin.id))
        .map(|mut skin| {
            // Champs explicites de skins.json : jamais inférés du prix ou du nom.
            // Source publique : CommunityDragon 16.19, global/default/v1/skins.json.
            if let Some(entry) = entries.get(&skin.id.to_string()) {
                skin.rarity = entry
                    .get("rarity")
                    .and_then(Value::as_str)
                    .filter(|rarity| {
                        matches!(
                            *rarity,
                            "kNoRarity"
                                | "kRare"
                                | "kEpic"
                                | "kLegendary"
                                | "kMythic"
                                | "kUltimate"
                                | "kExalted"
                                | "kTranscendent"
                        )
                    })
                    .map(str::to_string);
                if let Some(lines) = entry
                    .get("skinLines")
                    .and_then(Value::as_array)
                    .filter(|lines| lines.len() <= 100)
                {
                    skin.series_ids = lines
                        .iter()
                        .filter_map(|line| line.get("id").and_then(Value::as_u64))
                        .filter_map(|id| u32::try_from(id).ok())
                        .filter(|id| *id > 0)
                        .collect();
                    skin.series_ids.sort_unstable();
                    skin.series_ids.dedup();
                }
            }
            skin
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{ACCOUNT_ENDPOINT, REGION_ENDPOINT};
    use crate::test_support::{mock_client, ExpectedRequest};
    use serde_json::json;

    fn skin(id: u32, ownership: Value) -> Value {
        json!({"id":id,"championId":103,"isBase":false,"name":"Ahri dynastique",
            "ownership":ownership,"tilePath":"/lol-game-data/assets/ASSETS/Characters/Ahri/Skins/Skin01/Images/Ahri.jpg",
            "splashPath":"https://untrusted.invalid/image.jpg","stillObtainable":true})
    }

    #[test]
    fn distingue_possession_permanente_pret_absence_et_inconnu() {
        for (ownership, expected) in [
            (json!({"owned":true}), SkinOwnership::Owned),
            (json!({"owned":false}), SkinOwnership::Missing),
            (json!({}), SkinOwnership::Unknown),
            (json!({"owned":"true"}), SkinOwnership::Unknown),
            (
                json!({"owned":true,"rental":"unreadable"}),
                SkinOwnership::Unknown,
            ),
            (
                json!({"owned":true,"loyaltyReward":"unreadable"}),
                SkinOwnership::Unknown,
            ),
            (
                json!({"owned":true,"rental":{"rented":true}}),
                SkinOwnership::Temporary,
            ),
            (
                json!({"owned":false,"loyaltyReward":true}),
                SkinOwnership::Temporary,
            ),
            (
                json!({"owned":true,"xboxGPReward":true}),
                SkinOwnership::Temporary,
            ),
            (
                json!({"owned":false,"freeToPlayReward":true}),
                SkinOwnership::Temporary,
            ),
        ] {
            let skins = parse_skins(&json!([skin(103001, ownership)])).unwrap();
            assert_eq!(skins[0].ownership, expected);
            assert_eq!(skins[0].obtainable, Some(true));
            assert!(skins[0].splash_url.is_none());
        }
    }

    #[test]
    fn exclut_les_skins_de_base_et_refuse_les_catalogues_incoherents() {
        let mut base = skin(103000, json!({"owned":true}));
        base["isBase"] = json!(true);
        assert!(parse_skins(&json!([base])).unwrap().is_empty());
        let valid = skin(103001, json!({"owned":true}));
        for value in [
            json!({}),
            json!([valid.clone(), skin(103001, json!({"owned":false}))]),
            json!([skin(0, json!({}))]),
            json!([skin(104001, json!({}))]),
            json!(vec![valid.clone(); 10001]),
        ] {
            assert_eq!(parse_skins(&value), Err(CollectionError::InvalidResponse));
        }
        for (field, value) in [
            ("name", json!("x".repeat(513))),
            ("name", json!("bad\nname")),
            ("id", json!(-1)),
            ("championId", json!(0)),
            ("isBase", Value::Null),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            assert_eq!(
                parse_skins(&json!([invalid])),
                Err(CollectionError::InvalidResponse)
            );
        }
        assert!(parse_skins(&json!([])).unwrap().is_empty());
    }

    #[test]
    fn normalise_les_espaces_de_bord_des_noms_du_client_sans_accepter_un_nom_vide() {
        let mut spaced = skin(103001, json!({"owned":true}));
        spaced["name"] = json!(" Ahri dynastique ");
        assert_eq!(
            parse_skins(&json!([spaced])).unwrap()[0].name,
            "Ahri dynastique"
        );
        let mut empty = skin(103001, json!({"owned":true}));
        empty["name"] = json!("   ");
        assert_eq!(
            parse_skins(&json!([empty])),
            Err(CollectionError::InvalidResponse)
        );
    }

    #[test]
    fn deduplique_les_ids_identiques_et_refuse_les_conflits_sans_fusionner_les_noms() {
        let first = skin(103001, json!({"owned":true}));
        let second = skin(103002, json!({"owned":true}));
        let parsed = parse_skins(&json!([first.clone(), first.clone(), second])).unwrap();
        assert_eq!(
            parsed.iter().map(|skin| skin.id).collect::<Vec<_>>(),
            vec![103001, 103002]
        );
        assert_eq!(
            parse_skins(&json!([first, skin(103001, json!({"owned":false}))])),
            Err(CollectionError::InvalidResponse)
        );
    }

    #[test]
    fn utilise_les_ids_du_catalogue_pour_exclure_les_variantes_des_autres_modes() {
        let first = skin(103001, json!({"owned":true}));
        let second = skin(103002, json!({"owned":false}));
        let mut other_mode = skin(60103001, json!({"owned":true}));
        other_mode["championId"] = json!(60103);
        let parsed = parse_skins(&json!([first, second, other_mode])).unwrap();
        let catalog = json!({"103000":{"id":103000,"isBase":true,"skinClassification":"kChampion"},
            "103001":{"id":103001,"isBase":false,"skinClassification":"kChampion"},
            "103002":{"id":103002,"isBase":false,"skinClassification":"kChampion"}});
        let kept = retain_catalog_skins(parsed, &catalog).unwrap();
        assert_eq!(
            kept.iter().map(|skin| skin.id).collect::<Vec<_>>(),
            vec![103001, 103002]
        );
        for invalid in [
            json!([]),
            json!({}),
            json!({"103001":{"id":103002,"isBase":false,"skinClassification":"kChampion"}}),
        ] {
            assert_eq!(
                retain_catalog_skins(vec![], &invalid),
                Err(CollectionError::InvalidResponse)
            );
        }
    }

    #[test]
    fn enrichit_par_identifiant_avec_rarete_et_series_multiples_sans_inference() {
        let skins =
            parse_skins(&json!([skin(103001, json!({})), skin(103002, json!({}))])).unwrap();
        let catalog = json!({"103001":{"id":103001,"isBase":false,"skinClassification":"kChampion","rarity":"kEpic","skinLines":[{"id":10},{"id":20},{"id":10}]},
            "103002":{"id":103002,"isBase":false,"skinClassification":"kChampion","rarity":"newUnknown","skinLines":[{"id":-1}]}});
        let skins = retain_catalog_skins(skins, &catalog).unwrap();
        let payload = serde_json::to_value(&skins).unwrap();
        assert_eq!(payload[0]["rarity"], json!("kEpic"));
        assert_eq!(payload[0]["series_ids"], json!([10, 20]));
        assert_eq!(payload[1]["rarity"], Value::Null);
        assert_eq!(payload[1]["series_ids"], json!([]));
    }

    #[test]
    fn transforme_uniquement_les_images_game_data_sur_le_cdn_public() {
        let valid =
            json!("/lol-game-data/assets/ASSETS/Characters/Ahri/Skins/Skin01/Images/Ahri.jpg");
        assert_eq!(asset_url(Some(&valid)).as_deref(),Some("https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/characters/ahri/skins/skin01/images/ahri.jpg"));
        for path in [
            "https://example.com/a.jpg",
            "/lol-game-data/assets/ASSETS/../secret.jpg",
            "/lol-game-data/assets/ASSETS/a.jpg?secret=x",
            "/lol-game-data/assets/ASSETS/a.jpg#fragment",
            "/lol-game-data/assets/ASSETS/%2e%2e/a.jpg",
            "/lol-game-data/assets/ASSETS//a.jpg",
            "/lol-game-data/assets/ASSETS/a.svg",
            "/lol-game-data/assets/ASSETS/a\\b.jpg",
        ] {
            assert!(asset_url(Some(&json!(path))).is_none(), "{path}");
        }
        assert!(asset_url(None).is_none());
    }

    fn get(path: &str, response: Value) -> ExpectedRequest {
        ExpectedRequest {
            method: "GET",
            path: path.into(),
            body: None,
            status: 200,
            response,
        }
    }
    fn account(id: u64) -> Value {
        json!({"summonerId":id,"gameName":"Fixture","tagLine":"TEST","profileIconId":42})
    }
    fn expected() -> LcuAccount {
        LcuAccount::parse(&account(7), &json!({"region":"EUW"})).unwrap()
    }
    fn identity_requests(first: u64, last: u64) -> Vec<ExpectedRequest> {
        vec![
            get(ACCOUNT_ENDPOINT, account(first)),
            get(REGION_ENDPOINT, json!({"region":"EUW"})),
            get(ACCOUNT_ENDPOINT, account(last)),
        ]
    }

    fn catalog_request() -> ExpectedRequest {
        get(
            "/lol-game-data/assets/v1/skins.json",
            json!({"103001":{"id":103001,"isBase":false,"skinClassification":"kChampion"}}),
        )
    }

    #[tokio::test]
    async fn lit_la_collection_en_un_appel_et_garde_la_cle_dans_rust() {
        let mut requests = identity_requests(7, 7);
        requests.push(get(
            "/lol-champions/v1/inventories/7/skins-minimal",
            json!([skin(103001, json!({"owned":true}))]),
        ));
        requests.push(catalog_request());
        requests.extend(identity_requests(7, 7));
        let (client, server) = mock_client(requests).await;
        let snapshot = read_collection(&client, &expected()).await.unwrap();
        assert_eq!(snapshot.account_key, "EUW1-7");
        assert_eq!(snapshot.skins.len(), 1);
        assert!(!serde_json::to_string(&snapshot.skins)
            .unwrap()
            .contains("summonerId"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn refuse_les_courses_de_compte_meme_avec_le_meme_riot_id() {
        for (before, after) in [(true, false), (false, true)] {
            let mut requests = identity_requests(7, if before { 8 } else { 7 });
            if after {
                requests.push(get(
                    "/lol-champions/v1/inventories/7/skins-minimal",
                    json!([]),
                ));
                requests.push(catalog_request());
                requests.extend(identity_requests(8, 8));
            }
            let (client, server) = mock_client(requests).await;
            assert!(matches!(
                read_collection(&client, &expected()).await,
                Err(CollectionError::AccountChanged)
            ));
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn signale_les_erreurs_sans_reveler_la_reponse_du_client() {
        let mut failure = get(ACCOUNT_ENDPOINT, json!({"message":"private fixture"}));
        failure.status = 503;
        let (client, server) = mock_client(vec![failure]).await;
        assert!(matches!(
            read_collection(&client, &expected()).await,
            Err(CollectionError::Unavailable)
        ));
        server.await.unwrap();
        assert_eq!(
            serde_json::to_string(&CollectionError::InvalidResponse).unwrap(),
            "\"invalid_response\""
        );
    }
}
