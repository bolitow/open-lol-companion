use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::ImportError;
use crate::LcuClient;

/// Ensemble d'objets choisi par l'utilisateur, indépendant du format interne LCU.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportItemsRequest {
    pub champion_id: u32,
    pub champion_name: String,
    pub map_id: u32,
    pub blocks: Vec<ItemBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemBlock {
    pub label: String,
    pub items: Vec<ItemStack>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemStack {
    pub id: u32,
    pub count: u32,
}

impl LcuClient {
    /// Importe le set choisi sans supprimer les ensembles personnels du joueur.
    /// Source du GET/PUT et du schéma (client 26.16, vérifié le 01/10/2026) :
    /// https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json
    pub async fn import_items(&self, request: &ImportItemsRequest) -> Result<(), ImportError> {
        self.prepare_items(request).await?.apply(self).await
    }

    pub(super) async fn prepare_items(
        &self,
        request: &ImportItemsRequest,
    ) -> Result<PreparedItems, ImportError> {
        let item_set = build_item_set(request)?;
        let summoner: Value = self.get_json("/lol-summoner/v1/current-summoner").await?;
        let summoner_id = summoner
            .get("summonerId")
            .and_then(Value::as_u64)
            .filter(|id| *id > 0)
            .ok_or(ImportError::InvalidClientData)?;
        let path = format!("/lol-item-sets/v1/item-sets/{summoner_id}/sets");
        let bundle = self.get_json(&path).await?;
        let merged = merge_item_sets(bundle, item_set)?;
        Ok(PreparedItems { path, merged })
    }
}

pub(super) struct PreparedItems {
    path: String,
    merged: Value,
}
impl PreparedItems {
    pub(super) async fn apply(&self, client: &LcuClient) -> Result<(), ImportError> {
        client
            .write_json(reqwest::Method::PUT, &self.path, &self.merged)
            .await?;
        Ok(())
    }
    pub(super) async fn confirmed(&self, client: &LcuClient) -> bool {
        let Ok(actual) = client.get_json::<Value>(&self.path).await else {
            return false;
        };
        let expected = &self.merged["itemSets"][0];
        actual["itemSets"].as_array().is_some_and(|sets| {
            let matches: Vec<_> = sets
                .iter()
                .filter(|set| set["uid"] == expected["uid"])
                .collect();
            matches.len() == 1
                && ["associatedChampions", "associatedMaps", "blocks"]
                    .iter()
                    .all(|field| matches[0][field] == expected[field])
        })
    }
}

fn valid_number(value: u32) -> bool {
    (1..=i32::MAX as u32).contains(&value)
}

fn valid_label(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

/// Les évolutions de la Larme ne sont pas achetables et n'ont pas de lien `from`.
/// Identifiants et `gold.purchasable` vérifiés dans les données officielles :
/// https://ddragon.leagueoflegends.com/cdn/16.19.1/data/en_US/item.json
fn purchasable_item_id(id: u32) -> u32 {
    match id {
        3042 => 3004, // Muramana → Manamune.
        3040 => 3003, // Étreinte du Séraphin → Bâton de l'archange.
        3121 => 3119, // Fimbulvetr → Approche de l'hiver.
        _ => id,
    }
}

fn build_item_set(request: &ImportItemsRequest) -> Result<Value, ImportError> {
    if !valid_number(request.champion_id)
        || !valid_number(request.map_id)
        || !valid_label(&request.champion_name)
        || request.blocks.is_empty()
        || request.blocks.iter().any(|block| {
            !valid_label(&block.label)
                || block.items.is_empty()
                || block
                    .items
                    .iter()
                    .any(|item| !valid_number(item.id) || !valid_number(item.count))
        })
    {
        return Err(ImportError::InvalidItems);
    }
    let blocks: Vec<_> = request
        .blocks
        .iter()
        .map(|block| {
            let items: Vec<_> = block
                .items
                .iter()
                .map(|item| {
                    json!({"id": purchasable_item_id(item.id).to_string(), "count": item.count})
                })
                .collect();
            json!({
                "type": block.label.trim(),
                "items": items,
                "showIfSummonerSpell": "",
                "hideIfSummonerSpell": ""
            })
        })
        .collect();
    Ok(json!({
        "uid": format!("open-lol-companion-{}-{}", request.champion_id, request.map_id),
        "title": format!("{} : {}", super::APP_NAME, request.champion_name.trim()),
        "type": "custom",
        "map": "any",
        "mode": "any",
        "startedFrom": "",
        "sortrank": 0,
        "associatedChampions": [request.champion_id],
        "associatedMaps": [request.map_id],
        "preferredItemSlots": [],
        "blocks": blocks
    }))
}

fn merge_item_sets(mut bundle: Value, mut item_set: Value) -> Result<Value, ImportError> {
    if bundle
        .get("accountId")
        .and_then(Value::as_u64)
        .filter(|id| *id > 0)
        .is_none()
        || bundle.get("timestamp").and_then(Value::as_u64).is_none()
    {
        return Err(ImportError::InvalidClientData);
    }
    let own_uid = item_set
        .get("uid")
        .and_then(Value::as_str)
        .ok_or(ImportError::InvalidItems)?;
    let sets = bundle
        .get_mut("itemSets")
        .and_then(Value::as_array_mut)
        .ok_or(ImportError::InvalidClientData)?;
    let mut highest_rank = 0;
    for existing in sets.iter() {
        let uid = existing
            .get("uid")
            .and_then(Value::as_str)
            .filter(|uid| !uid.is_empty())
            .ok_or(ImportError::InvalidClientData)?;
        if uid == own_uid {
            continue;
        }
        let rank = existing
            .get("sortrank")
            .and_then(Value::as_i64)
            .and_then(|rank| i32::try_from(rank).ok())
            .ok_or(ImportError::InvalidClientData)?;
        highest_rank = highest_rank.max(rank);
    }
    // Les rangs sont décroissants ; `priority` n'est pas exposé par le schéma LCU.
    // Ancienne documentation officielle Riot conservée par CommunityDragon :
    // https://raw.githubusercontent.com/CommunityDragon/HexDocs/master/lol/misc/itemsets.md
    let rank = highest_rank
        .checked_add(1)
        .ok_or(ImportError::ItemSetPriorityUnavailable)?;
    // Garder les objets JSON complets préserve aussi les futurs champs du client.
    sets.retain(|existing| existing.get("uid").and_then(Value::as_str) != Some(own_uid));
    item_set["sortrank"] = json!(rank);
    sets.insert(0, item_set);
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::test_support::{mock_client, ExpectedRequest};

    fn request() -> ImportItemsRequest {
        ImportItemsRequest {
            champion_id: 81,
            champion_name: "Ezreal".into(),
            map_id: 11,
            blocks: vec![ItemBlock {
                label: "Objets principaux".into(),
                items: vec![
                    ItemStack { id: 3042, count: 1 },
                    ItemStack { id: 3040, count: 1 },
                    ItemStack { id: 3121, count: 1 },
                    ItemStack { id: 3070, count: 2 },
                ],
            }],
        }
    }

    fn bundle(sets: Vec<Value>) -> Value {
        json!({"accountId": 17, "timestamp": 1234, "itemSets": sets})
    }

    fn personal_set() -> Value {
        json!({
            "uid": "private-set",
            "title": "Open LoL Companion : Ezreal",
            "sortrank": 25,
            "associatedChampions": [81],
            "blocks": [{"items": [{"id": "1001", "count": 1}]}],
            "futureField": {"nested": [1, 2, 3]}
        })
    }

    #[test]
    fn serialise_le_contrat_partage_en_camel_case() {
        let value = serde_json::to_value(request()).unwrap();
        assert_eq!(value["championId"], 81);
        assert_eq!(value["championName"], "Ezreal");
        assert_eq!(value["mapId"], 11);
        assert_eq!(
            value["blocks"][0]["items"][0],
            json!({"id": 3042, "count": 1})
        );
        assert!(value.get("champion_id").is_none());
    }

    #[test]
    fn convertit_les_trois_objets_larme_vers_leur_forme_achetable() {
        let set = build_item_set(&request()).unwrap();
        assert_eq!(
            set["blocks"][0]["items"],
            json!([
                {"id": "3004", "count": 1},
                {"id": "3003", "count": 1},
                {"id": "3119", "count": 1},
                {"id": "3070", "count": 2}
            ])
        );
        assert_eq!(set["uid"], "open-lol-companion-81-11");
        assert_eq!(set["title"], "Open LoL Companion : Ezreal");
        assert_eq!(set["associatedChampions"], json!([81]));
        assert_eq!(set["associatedMaps"], json!([11]));
        assert_eq!(set["type"], "custom");
        assert_eq!(set["map"], "any");
        assert_eq!(set["mode"], "any");
        assert_eq!(set["startedFrom"], "");
        assert_eq!(set["preferredItemSlots"], json!([]));
        assert_eq!(set["blocks"][0]["type"], "Objets principaux");
        assert_eq!(set["blocks"][0]["showIfSummonerSpell"], "");
        assert_eq!(set["blocks"][0]["hideIfSummonerSpell"], "");
    }

    #[test]
    fn refuse_les_identifiants_et_quantites_hors_du_format_lcu() {
        for invalid in [0, i32::MAX as u32 + 1, u32::MAX] {
            let mut input = request();
            input.champion_id = invalid;
            assert!(matches!(
                build_item_set(&input),
                Err(ImportError::InvalidItems)
            ));
            input = request();
            input.map_id = invalid;
            assert!(matches!(
                build_item_set(&input),
                Err(ImportError::InvalidItems)
            ));
            input = request();
            input.blocks[0].items[0].id = invalid;
            assert!(matches!(
                build_item_set(&input),
                Err(ImportError::InvalidItems)
            ));
            input = request();
            input.blocks[0].items[0].count = invalid;
            assert!(matches!(
                build_item_set(&input),
                Err(ImportError::InvalidItems)
            ));
        }
    }

    #[test]
    fn refuse_les_noms_et_blocs_vides() {
        for invalid in ["", "  ", "Ezreal\n", "Ezreal\0"] {
            let mut input = request();
            input.champion_name = invalid.into();
            assert!(matches!(
                build_item_set(&input),
                Err(ImportError::InvalidItems)
            ));
            input = request();
            input.blocks[0].label = invalid.into();
            assert!(matches!(
                build_item_set(&input),
                Err(ImportError::InvalidItems)
            ));
        }
        let mut input = request();
        input.blocks.clear();
        assert!(matches!(
            build_item_set(&input),
            Err(ImportError::InvalidItems)
        ));
        input = request();
        input.blocks[0].items.clear();
        assert!(matches!(
            build_item_set(&input),
            Err(ImportError::InvalidItems)
        ));
    }

    #[test]
    fn preserve_les_ensembles_personnels_et_toutes_leurs_metadonnees() {
        let private = personal_set();
        let mut original = bundle(vec![private.clone()]);
        original["futureBundleField"] = json!({"source": "client"});
        let merged = merge_item_sets(original, build_item_set(&request()).unwrap()).unwrap();
        assert_eq!(merged["itemSets"][0]["uid"], "open-lol-companion-81-11");
        assert_eq!(merged["itemSets"][0]["sortrank"], 26);
        assert_eq!(merged["itemSets"][1], private);
        assert_eq!(merged["accountId"], 17);
        assert_eq!(merged["timestamp"], 1234);
        assert_eq!(merged["futureBundleField"], json!({"source": "client"}));
    }

    #[test]
    fn remplace_uniquement_l_uid_exact_de_l_app_sans_accumuler_les_imports() {
        let own = build_item_set(&request()).unwrap();
        let another = json!({"uid": "open-lol-companion-81-12", "sortrank": 3});
        let merged = merge_item_sets(
            bundle(vec![
                own.clone(),
                personal_set(),
                another.clone(),
                own.clone(),
            ]),
            own.clone(),
        )
        .unwrap();
        assert_eq!(merged["itemSets"].as_array().unwrap().len(), 3);
        assert_eq!(merged["itemSets"][1], personal_set());
        assert_eq!(merged["itemSets"][2], another);
        assert_eq!(merge_item_sets(merged.clone(), own).unwrap(), merged);
    }

    #[test]
    fn ne_depasse_pas_la_capacite_du_rang_et_ne_modifie_pas_un_set_prive() {
        let private = json!({"uid": "private-set", "sortrank": i32::MAX});
        assert_eq!(
            merge_item_sets(bundle(vec![private]), build_item_set(&request()).unwrap()),
            Err(ImportError::ItemSetPriorityUnavailable)
        );
    }

    #[test]
    fn accepte_le_premier_import_et_le_remplacement_de_l_ancien_rang_maximal() {
        let own = build_item_set(&request()).unwrap();
        let first = merge_item_sets(bundle(vec![]), own.clone()).unwrap();
        assert_eq!(first["itemSets"].as_array().unwrap().len(), 1);
        let mut old = own.clone();
        old["sortrank"] = json!(i32::MAX);
        assert_eq!(merge_item_sets(bundle(vec![old]), own).unwrap(), first);
    }

    #[test]
    fn refuse_un_bundle_client_incomplet_ou_malforme() {
        let malformed = [
            Value::Null,
            json!({}),
            json!({"accountId": 17, "itemSets": []}),
            json!({"timestamp": 1, "itemSets": []}),
            json!({"accountId": -1, "timestamp": 1, "itemSets": []}),
            json!({"accountId": 17, "timestamp": "invalid", "itemSets": []}),
            json!({"accountId": 17, "timestamp": 1, "itemSets": {}}),
            bundle(vec![Value::Null]),
            bundle(vec![json!({"uid": "private"})]),
            bundle(vec![json!({"uid": 1, "sortrank": 1})]),
            bundle(vec![json!({"uid": "private", "sortrank": i64::MAX})]),
        ];
        for invalid in malformed {
            assert!(matches!(
                merge_item_sets(invalid, build_item_set(&request()).unwrap()),
                Err(ImportError::InvalidClientData)
            ));
        }
    }

    #[tokio::test]
    async fn importe_apres_lecture_du_compte_et_preserve_les_sets_du_joueur() {
        let expected_set = json!({
            "uid": "open-lol-companion-81-11",
            "title": "Open LoL Companion : Ezreal",
            "type": "custom", "map": "any", "mode": "any", "startedFrom": "",
            "sortrank": 26, "associatedChampions": [81], "associatedMaps": [11],
            "preferredItemSlots": [],
            "blocks": [{
                "type": "Objets principaux", "showIfSummonerSpell": "", "hideIfSummonerSpell": "",
                "items": [
                    {"id": "3004", "count": 1}, {"id": "3003", "count": 1},
                    {"id": "3119", "count": 1}, {"id": "3070", "count": 2}
                ]
            }]
        });
        let (client, server) = mock_client(vec![
            ExpectedRequest {
                method: "GET",
                path: "/lol-summoner/v1/current-summoner".into(),
                body: None,
                status: 200,
                response: json!({"summonerId": 42, "accountId": 17}),
            },
            ExpectedRequest {
                method: "GET",
                path: "/lol-item-sets/v1/item-sets/42/sets".into(),
                body: None,
                status: 200,
                response: bundle(vec![personal_set()]),
            },
            ExpectedRequest {
                method: "PUT",
                path: "/lol-item-sets/v1/item-sets/42/sets".into(),
                body: Some(bundle(vec![expected_set, personal_set()])),
                status: 204,
                response: Value::Null,
            },
        ])
        .await;
        client.import_items(&request()).await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn valide_la_requete_avant_tout_acces_au_client() {
        let (client, server) = mock_client(vec![]).await;
        let mut input = request();
        input.blocks.clear();
        assert_eq!(
            client.import_items(&input).await,
            Err(ImportError::InvalidItems)
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn refuse_un_invocateur_invalide_sans_ecrire() {
        for response in [
            json!({}),
            json!({"summonerId": 0}),
            json!({"summonerId": -1}),
        ] {
            let (client, server) = mock_client(vec![ExpectedRequest {
                method: "GET",
                path: "/lol-summoner/v1/current-summoner".into(),
                body: None,
                status: 200,
                response,
            }])
            .await;
            assert_eq!(
                client.import_items(&request()).await,
                Err(ImportError::InvalidClientData)
            );
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn refuse_un_bundle_invalide_sans_le_remplacer() {
        let (client, server) = mock_client(vec![
            ExpectedRequest {
                method: "GET",
                path: "/lol-summoner/v1/current-summoner".into(),
                body: None,
                status: 200,
                response: json!({"summonerId": 42}),
            },
            ExpectedRequest {
                method: "GET",
                path: "/lol-item-sets/v1/item-sets/42/sets".into(),
                body: None,
                status: 200,
                response: json!({"accountId": 17, "timestamp": 1}),
            },
        ])
        .await;
        assert_eq!(
            client.import_items(&request()).await,
            Err(ImportError::InvalidClientData)
        );
        server.await.unwrap();
    }
}
