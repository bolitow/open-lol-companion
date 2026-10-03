//! Imports activés par le joueur, bornés à un prépick précis (#63).
//! Schémas gameId, gameflow et currentpage vérifiés le 02/10/2026 :
//! https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json
use super::{
    DraftRuneGuardError, DraftRuneImportError, ImportError, ImportItemsRequest, ImportRunesRequest,
    ImportSpellsRequest,
};
use crate::draft::{DraftMode, FLOW_ENDPOINT};
use crate::{DraftSession, LcuClient, DRAFT_ENDPOINT};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutoImportContext {
    pub draft_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_id: Option<String>,
    pub champion_id: u32,
    pub role: String,
    pub queue_id: u32,
}
impl AutoImportContext {
    /// L'identité Riot peut apparaître après le prépick, sans nouvelle sélection.
    pub fn same_selection(&self, other: &Self) -> bool {
        self.draft_id == other.draft_id
            && self.champion_id == other.champion_id
            && self.role == other.role
            && self.queue_id == other.queue_id
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", content = "request", rename_all = "camelCase")]
pub enum AutoImportSelection {
    Runes(ImportRunesRequest),
    Items(ImportItemsRequest),
    Spells(ImportSpellsRequest),
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutoImportRequest {
    pub context: AutoImportContext,
    pub selection: AutoImportSelection,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AutoImportReceipt {
    pub confirmed: bool,
}

impl LcuClient {
    pub async fn import_selected_build(
        &self,
        request: &AutoImportRequest,
        current_draft: impl Fn() -> bool,
    ) -> Result<AutoImportReceipt, DraftRuneImportError> {
        let confirmed = match &request.selection {
            AutoImportSelection::Runes(runes) => {
                let prepared = self.prepare_runes(runes).await?;
                self.require_selected_context(&request.context).await?;
                if !current_draft() {
                    return Err(DraftRuneImportError::Guard(
                        DraftRuneGuardError::DraftContextChanged,
                    ));
                }
                prepared.apply(self).await?;
                // Une écriture acceptée reste un succès même si sa relecture échoue.
                // Aucun nouvel envoi automatique dans ce cas incertain.
                self.get_json::<serde_json::Value>(crate::RUNES_ENDPOINT)
                    .await
                    .ok()
                    .and_then(crate::RunePage::parse)
                    .is_some_and(|page| {
                        page.is_valid
                            && !page.is_temporary
                            && page.primary_style_id == runes.primary_style_id
                            && page.sub_style_id == runes.sub_style_id
                            && page.selected_perk_ids == runes.selected_perk_ids
                    })
            }
            AutoImportSelection::Spells(spells) => {
                let prepared = self.prepare_spells(spells).await?;
                self.selected_context(&request.context).await?;
                if !current_draft() {
                    return Err(DraftRuneImportError::Guard(
                        DraftRuneGuardError::DraftContextChanged,
                    ));
                }
                prepared.apply(self).await?;
                // Une paire identique appartenant à un autre champion ne confirme pas cet import.
                self.selected_context(&request.context)
                    .await
                    .ok()
                    .is_some_and(|draft| current_draft() && prepared.matches(draft.local_spells))
            }
            AutoImportSelection::Items(items) => {
                if items.champion_id != request.context.champion_id || items.map_id != 11 {
                    return Err(ImportError::InvalidItems.into());
                }
                let prepared = self.prepare_items(items).await?;
                self.require_selected_context(&request.context).await?;
                if !current_draft() {
                    return Err(DraftRuneImportError::Guard(
                        DraftRuneGuardError::DraftContextChanged,
                    ));
                }
                prepared.apply(self).await?;
                prepared.confirmed(self).await
            }
        };
        Ok(AutoImportReceipt { confirmed })
    }
    async fn require_selected_context(
        &self,
        context: &AutoImportContext,
    ) -> Result<(), DraftRuneImportError> {
        self.selected_context(context).await.map(|_| ())
    }
    async fn selected_context(
        &self,
        context: &AutoImportContext,
    ) -> Result<DraftSession, DraftRuneImportError> {
        let changed = DraftRuneImportError::Guard(DraftRuneGuardError::DraftContextChanged);
        if context.champion_id == 0
            || context.draft_id.is_empty()
            || context
                .game_id
                .as_ref()
                .is_some_and(|id| id.parse::<u64>().ok().filter(|n| *n > 0).is_none())
            || !matches!(
                context.role.as_str(),
                "TOP" | "JUNGLE" | "MIDDLE" | "BOTTOM" | "UTILITY"
            )
        {
            return Err(changed);
        }
        let flow = self.get_json(FLOW_ENDPOINT).await?;
        let mode = DraftMode::from_flow(&flow);
        if !matches!(mode, DraftMode::StandardRift(Some(queue)) | DraftMode::CustomRift(Some(queue)) if queue == context.queue_id)
        {
            return Err(changed);
        }
        let draft = DraftSession::parse_for_mode(self.get_json(DRAFT_ENDPOINT).await?, mode)
            .ok_or(ImportError::InvalidClientData)?;
        let mut local = draft.allies.iter().filter(|player| player.local);
        let matches = local.next().is_some_and(|player| {
            player.champion_id == Some(context.champion_id)
                && match player.position.as_deref() {
                    Some(role) => role.eq_ignore_ascii_case(&context.role),
                    None => matches!(mode, DraftMode::CustomRift(_)),
                }
        });
        if !draft.supported
            || context
                .game_id
                .as_ref()
                .is_some_and(|id| draft.game_id.as_ref() != Some(id))
            || !matches
            || local.next().is_some()
        {
            return Err(changed);
        }
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{mock_client, ExpectedRequest};
    use serde_json::{json, Value};
    fn read(path: &str, response: Value) -> ExpectedRequest {
        ExpectedRequest {
            method: "GET",
            path: path.into(),
            body: None,
            status: 200,
            response,
        }
    }
    fn context() -> AutoImportContext {
        AutoImportContext {
            draft_id: "draft-1".into(),
            game_id: Some("123".into()),
            champion_id: 432,
            role: "UTILITY".into(),
            queue_id: 420,
        }
    }
    #[test]
    fn dedoublonne_le_prepick_au_verrouillage_mais_pas_une_nouvelle_selection() {
        let locked = context();
        let pre = AutoImportContext {
            game_id: None,
            ..locked.clone()
        };
        assert!(pre.same_selection(&locked));
        for other in [
            AutoImportContext {
                draft_id: "draft-2".into(),
                ..locked.clone()
            },
            AutoImportContext {
                champion_id: 103,
                ..locked.clone()
            },
            AutoImportContext {
                role: "MIDDLE".into(),
                ..locked.clone()
            },
            AutoImportContext {
                queue_id: 3100,
                ..locked.clone()
            },
        ] {
            assert!(!pre.same_selection(&other));
        }
    }
    fn flow() -> Value {
        json!({"phase":"ChampSelect","gameData":{"queue":{"id":420,"mapId":11,"gameMode":"CLASSIC"}}})
    }
    fn draft() -> Value {
        json!({"gameId":123,"localPlayerCellId":0,
            "myTeam":(0..5).map(|cell|json!({"cellId":cell,"championId":432,"assignedPosition":"utility"})).collect::<Vec<_>>(),
            "theirTeam":(5..10).map(|cell|json!({"cellId":cell})).collect::<Vec<_>>(),
            "actions":[[{"actorCellId":0,"type":"pick","completed":true}]]})
    }
    #[tokio::test]
    async fn refuse_autre_partie_champion_poste_et_intention_sans_ecriture() {
        for (field, value) in [
            ("gameId", json!(124)),
            ("championId", json!(103)),
            ("assignedPosition", json!("middle")),
            ("championId", json!(0)),
        ] {
            let mut d = draft();
            match field {
                "gameId" => d[field] = value,
                "completed" => d["actions"][0][0][field] = value,
                _ => d["myTeam"][0][field] = value,
            }
            let (client, server) =
                mock_client(vec![read(FLOW_ENDPOINT, flow()), read(DRAFT_ENDPOINT, d)]).await;
            assert_eq!(
                client.require_selected_context(&context()).await,
                Err(DraftRuneImportError::Guard(
                    DraftRuneGuardError::DraftContextChanged
                ))
            );
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn refuse_autre_phase_ou_file() {
        for value in [
            json!({"phase":"InProgress"}),
            json!({"phase":"ChampSelect","gameData":{"queue":{"id":440,"mapId":11,"gameMode":"CLASSIC"}}}),
        ] {
            let (client, server) = mock_client(vec![read(FLOW_ENDPOINT, value)]).await;
            assert!(client.require_selected_context(&context()).await.is_err());
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn accepte_le_prepick_local_et_la_personnalisee_avant_verrouillage() {
        for custom in [false, true] {
            let mut d = draft();
            d["myTeam"][0]["championId"] = json!(0);
            d["myTeam"][0]["championPickIntent"] = json!(432);
            d["actions"][0][0]["completed"] = json!(false);
            let mut ctx = context();
            let f = if custom {
                ctx.queue_id = 3100;
                d["myTeam"] = json!([{"cellId":0,"championId":0,"championPickIntent":432,"assignedPosition":""}]);
                d["theirTeam"] = json!([]);
                json!({"phase":"ChampSelect","gameData":{"isCustomGame":true,"queue":{"id":3100}},"map":{"id":11,"gameMode":"CLASSIC"}})
            } else {
                flow()
            };
            let (client, server) =
                mock_client(vec![read(FLOW_ENDPOINT, f), read(DRAFT_ENDPOINT, d)]).await;
            assert_eq!(client.require_selected_context(&ctx).await, Ok(()));
            server.await.unwrap();
        }
    }
    fn spell_request(slot: &str, pair: [u32; 2]) -> AutoImportRequest {
        serde_json::from_value(json!({"context":context(),"selection":{"kind":"spells","request":{"spellIds":pair,"flashSlot":slot}}})).unwrap()
    }
    #[tokio::test]
    async fn sorts_automatiques_respectent_flash_et_confirment_la_paire_sans_toucher_au_skin() {
        for (slot, pair, expected) in [
            ("D", [14, 4], [4, 14]),
            ("F", [4, 14], [14, 4]),
            ("F", [6, 14], [6, 14]),
        ] {
            let request = spell_request(slot, pair);
            let mut after = draft();
            after["myTeam"][0]["spell1Id"] = json!(expected[0]);
            after["myTeam"][0]["spell2Id"] = json!(expected[1]);
            let (client, server) = mock_client(vec![
                read("/lol-gameflow/v1/gameflow-phase", json!("ChampSelect")),
                read(FLOW_ENDPOINT, flow()),
                read(DRAFT_ENDPOINT, draft()),
                ExpectedRequest {
                    method: "PATCH",
                    path: "/lol-champ-select/v1/session/my-selection".into(),
                    body: Some(json!({"spell1Id":expected[0],"spell2Id":expected[1]})),
                    status: 204,
                    response: Value::Null,
                },
                read(FLOW_ENDPOINT, flow()),
                read(DRAFT_ENDPOINT, after),
            ])
            .await;
            assert_eq!(
                client.import_selected_build(&request, || true).await,
                Ok(AutoImportReceipt { confirmed: true })
            );
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn sorts_automatiques_refusent_un_contexte_ou_une_generation_perimee_sans_patch() {
        for stale_champion in [true, false] {
            let request = spell_request("F", [4, 14]);
            let mut d = draft();
            if stale_champion {
                d["myTeam"][0]["championId"] = json!(103);
            }
            let (client, server) = mock_client(vec![
                read("/lol-gameflow/v1/gameflow-phase", json!("ChampSelect")),
                read(FLOW_ENDPOINT, flow()),
                read(DRAFT_ENDPOINT, d),
            ])
            .await;
            assert_eq!(
                client.import_selected_build(&request, || false).await,
                Err(DraftRuneImportError::Guard(
                    DraftRuneGuardError::DraftContextChanged
                ))
            );
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn sorts_acceptes_ne_sont_pas_confirmes_si_la_paire_ou_le_champion_a_change() {
        for changed_champion in [true, false] {
            let request = spell_request("F", [4, 14]);
            let mut after = draft();
            after["myTeam"][0]["spell1Id"] = json!(if changed_champion { 14 } else { 7 });
            after["myTeam"][0]["spell2Id"] = json!(4);
            if changed_champion {
                after["myTeam"][0]["championId"] = json!(103);
            }
            let (client, server) = mock_client(vec![
                read("/lol-gameflow/v1/gameflow-phase", json!("ChampSelect")),
                read(FLOW_ENDPOINT, flow()),
                read(DRAFT_ENDPOINT, draft()),
                ExpectedRequest {
                    method: "PATCH",
                    path: "/lol-champ-select/v1/session/my-selection".into(),
                    body: Some(json!({"spell1Id":14,"spell2Id":4})),
                    status: 204,
                    response: Value::Null,
                },
                read(FLOW_ENDPOINT, flow()),
                read(DRAFT_ENDPOINT, after),
            ])
            .await;
            assert_eq!(
                client.import_selected_build(&request, || true).await,
                Ok(AutoImportReceipt { confirmed: false })
            );
            server.await.unwrap();
        }
    }
    fn runes() -> ImportRunesRequest {
        ImportRunesRequest {
            champion_name: "Bard".into(),
            primary_style_id: 8000,
            sub_style_id: 8200,
            selected_perk_ids: vec![8005, 9111, 9104, 8014, 8233, 8236, 5005, 5008, 5001],
        }
    }
    #[tokio::test]
    async fn importe_les_objets_apres_garde_et_confirme_sans_effacer_les_sets_personnels() {
        use super::super::{ItemBlock, ItemStack};
        let items = ImportItemsRequest {
            champion_id: 432,
            champion_name: "Bard".into(),
            map_id: 11,
            blocks: vec![ItemBlock {
                label: "Inventaire final observé".into(),
                items: vec![ItemStack { id: 1001, count: 1 }],
            }],
        };
        let private = json!({"uid":"personal","sortrank":4,"futureField":true});
        let bundle = json!({"accountId":17,"timestamp":1,"itemSets":[private]});
        let expected = json!({"accountId":17,"timestamp":1,"itemSets":[{
            "uid":"open-lol-companion-432-11","title":"Open LoL Companion : Bard","type":"custom",
            "map":"any","mode":"any","startedFrom":"","sortrank":5,"associatedChampions":[432],
            "associatedMaps":[11],"preferredItemSlots":[],"blocks":[{"type":"Inventaire final observé",
                "items":[{"id":"1001","count":1}],"showIfSummonerSpell":"","hideIfSummonerSpell":""}]
        },private]});
        for stale in [true, false] {
            let mut d = draft();
            if stale {
                d["gameId"] = json!(999);
            }
            let path = "/lol-item-sets/v1/item-sets/42/sets";
            let mut steps = vec![
                read(
                    "/lol-summoner/v1/current-summoner",
                    json!({"summonerId":42}),
                ),
                read(path, bundle.clone()),
                read(FLOW_ENDPOINT, flow()),
                read(DRAFT_ENDPOINT, d),
            ];
            if !stale {
                steps.extend([
                    ExpectedRequest {
                        method: "PUT",
                        path: path.into(),
                        body: Some(expected.clone()),
                        status: 204,
                        response: Value::Null,
                    },
                    read(path, expected.clone()),
                ]);
            }
            let (client, server) = mock_client(steps).await;
            let result = client
                .import_selected_build(
                    &AutoImportRequest {
                        context: context(),
                        selection: AutoImportSelection::Items(items.clone()),
                    },
                    || true,
                )
                .await;
            if stale {
                assert!(result.is_err());
            } else {
                assert_eq!(result, Ok(AutoImportReceipt { confirmed: true }));
            }
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn trois_prepicks_reutilisent_la_meme_page_sans_creation_ni_suppression() {
        let mut steps = Vec::new();
        let mut requests = Vec::new();
        for (champion, name, old_name) in [
            (432, "Bard", "Ahri"),
            (103, "Ahri", "Bard"),
            (432, "Bard", "Ahri"),
        ] {
            let mut selected = runes();
            selected.champion_name = name.into();
            let mut ctx = context();
            ctx.champion_id = champion;
            ctx.game_id = None;
            let mut d = draft();
            d["gameId"] = json!(0);
            d["myTeam"][0]["championId"] = json!(0);
            d["myTeam"][0]["championPickIntent"] = json!(champion);
            d["actions"][0][0]["completed"] = json!(false);
            steps.extend([
                read("/lol-perks/v1/styles",serde_json::from_str(include_str!("../../tests/fixtures/rune-styles.json")).unwrap()),
                read("/lol-perks/v1/pages",json!([{"id":7,"name":format!("Open LoL Companion : {old_name}"),"isEditable":true},{"id":9,"name":"Personnel","isEditable":true}])),
                read(FLOW_ENDPOINT,flow()),read(DRAFT_ENDPOINT,d),
                ExpectedRequest {method:"PUT",path:"/lol-perks/v1/pages/7".into(),body:Some(json!({"id":7,"name":format!("Open LoL Companion : {name}"),"primaryStyleId":8000,"subStyleId":8200,"selectedPerkIds":selected.selected_perk_ids,"current":true})),status:204,response:Value::Null},
                read("/lol-perks/v1/currentpage",json!({"primaryStyleId":8000,"subStyleId":8200,"selectedPerkIds":selected.selected_perk_ids,"isValid":true,"isTemporary":false})),
            ]);
            requests.push(AutoImportRequest {
                context: ctx,
                selection: AutoImportSelection::Runes(selected),
            });
        }
        let (client, server) = mock_client(steps).await;
        for request in requests {
            assert_eq!(
                client.import_selected_build(&request, || true).await,
                Ok(AutoImportReceipt { confirmed: true })
            );
        }
        server.await.unwrap();
    }
    #[tokio::test]
    async fn refuse_une_generation_de_draft_expiree_apres_les_lectures() {
        let (client, server) = mock_client(vec![
            read(
                "/lol-perks/v1/styles",
                serde_json::from_str(include_str!("../../tests/fixtures/rune-styles.json"))
                    .unwrap(),
            ),
            read("/lol-perks/v1/pages", json!([])),
            read(FLOW_ENDPOINT, flow()),
            read(DRAFT_ENDPOINT, draft()),
        ])
        .await;
        assert_eq!(
            client
                .import_selected_build(
                    &AutoImportRequest {
                        context: context(),
                        selection: AutoImportSelection::Runes(runes())
                    },
                    || false
                )
                .await,
            Err(DraftRuneImportError::Guard(
                DraftRuneGuardError::DraftContextChanged
            ))
        );
        server.await.unwrap();
    }
    #[tokio::test]
    async fn confirme_les_runes_apres_garde_sans_ecrire_les_sorts() {
        for confirmed in [true, false] {
            let request = runes();
            let mut page = json!({"primaryStyleId":8000,"subStyleId":8200,"selectedPerkIds":request.selected_perk_ids,"isValid":true,"isTemporary":false});
            if !confirmed {
                page["primaryStyleId"] = json!(8100);
            }
            let (client, server) = mock_client(vec![
                read("/lol-perks/v1/styles", serde_json::from_str(include_str!("../../tests/fixtures/rune-styles.json")).unwrap()),
                read("/lol-perks/v1/pages",json!([])), read(FLOW_ENDPOINT,flow()), read(DRAFT_ENDPOINT,draft()),
                ExpectedRequest { method:"POST",path:"/lol-perks/v1/pages".into(),body:Some(json!({"name":"Open LoL Companion : Bard","primaryStyleId":8000,"subStyleId":8200,"selectedPerkIds":request.selected_perk_ids,"current":true})),status:204,response:Value::Null },
                read("/lol-perks/v1/currentpage",page),
            ]).await;
            assert_eq!(
                client
                    .import_selected_build(
                        &AutoImportRequest {
                            context: context(),
                            selection: AutoImportSelection::Runes(request)
                        },
                        || true
                    )
                    .await,
                Ok(AutoImportReceipt { confirmed })
            );
            server.await.unwrap();
        }
    }
}
