//! Parcours manuel du front #13, borné au champion et au mode actuels.
use super::{DraftRuneImportError, ImportSpellsRequest};
use crate::LcuClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportDraftSpellsRequest {
    pub champion_id: u32,
    pub spells: ImportSpellsRequest,
    /// Faux pour une édition explicite ; vrai pour la variante observée non modifiée.
    #[serde(default)]
    pub preserve_equipped_slot: bool,
}
impl LcuClient {
    pub async fn import_draft_spells(
        &self,
        request: &ImportDraftSpellsRequest,
    ) -> Result<(), DraftRuneImportError> {
        let draft = self.require_draft_champion(request.champion_id).await?;
        let prepared = self.prepare_spells(&request.spells).await?;
        let prepared = if request.preserve_equipped_slot {
            prepared.preserve_equipped_slot(draft.local_spells)
        } else {
            prepared
        };
        prepared.apply(self).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imports::{
        DraftRuneGuardError, DraftRuneImportError, FlashSlot, ImportSpellsRequest,
    };
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
    fn request() -> ImportDraftSpellsRequest {
        ImportDraftSpellsRequest {
            champion_id: 103,
            preserve_equipped_slot: false,
            spells: ImportSpellsRequest {
                spell_ids: [14, 4],
                flash_slot: FlashSlot::D,
            },
        }
    }
    fn flow(phase: &str, queue: u32) -> Value {
        json!({"phase":phase,"gameData":{"queue":{"id":queue,"mapId":11,"gameMode":"CLASSIC"}}})
    }
    fn draft(champion: u32) -> Value {
        json!({"localPlayerCellId":0,"myTeam":(0..5).map(|cell|json!({"cellId":cell,"championId":if cell==0 {champion}else{0},"team":1})).collect::<Vec<_>>(),"theirTeam":(5..10).map(|cell|json!({"cellId":cell,"team":2})).collect::<Vec<_>>(),"actions":[]})
    }
    #[tokio::test]
    async fn distingue_la_variante_observee_d_une_edition_explicite_et_ancienne_requete() {
        for (flag, equipped, pair, slot, expected) in [
            (Some(true), [14, 4], [6, 14], "F", [14, 6]),
            (Some(false), [14, 4], [6, 14], "F", [6, 14]),
            (None, [14, 4], [6, 14], "F", [6, 14]),
            (Some(true), [7, 21], [14, 6], "D", [14, 6]),
            (Some(true), [14, 4], [14, 4], "D", [4, 14]),
            (Some(true), [4, 14], [4, 14], "F", [14, 4]),
        ] {
            let mut value = json!({"championId":103,"spells":{"spellIds":pair,"flashSlot":slot}});
            if let Some(flag) = flag {
                value["preserveEquippedSlot"] = json!(flag);
            }
            let request: ImportDraftSpellsRequest = serde_json::from_value(value).unwrap();
            let mut current = draft(103);
            current["myTeam"][0]["spell1Id"] = json!(equipped[0]);
            current["myTeam"][0]["spell2Id"] = json!(equipped[1]);
            let (client, server) = mock_client(vec![
                read("/lol-gameflow/v1/session", flow("ChampSelect", 420)),
                read("/lol-champ-select/v1/session", current),
                read("/lol-gameflow/v1/gameflow-phase", json!("ChampSelect")),
                ExpectedRequest {
                    method: "PATCH",
                    path: "/lol-champ-select/v1/session/my-selection".into(),
                    body: Some(json!({"spell1Id":expected[0],"spell2Id":expected[1]})),
                    status: 204,
                    response: Value::Null,
                },
            ])
            .await;
            assert_eq!(client.import_draft_spells(&request).await, Ok(()));
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn importe_en_personnalisee_faille_avec_un_joueur_et_un_bot() {
        let flow = json!({"phase":"ChampSelect","gameData":{"isCustomGame":true,"queue":{"id":0}},"map":{"id":11,"gameMode":"CLASSIC"}});
        let draft = json!({"localPlayerCellId":0,"myTeam":[{"cellId":0,"championId":103,"team":1}],"theirTeam":[{"cellId":5,"championId":222,"team":2}],"actions":[]});
        let (client, server) = mock_client(vec![
            read("/lol-gameflow/v1/session", flow),
            read("/lol-champ-select/v1/session", draft),
            read("/lol-gameflow/v1/gameflow-phase", json!("ChampSelect")),
            ExpectedRequest {
                method: "PATCH",
                path: "/lol-champ-select/v1/session/my-selection".into(),
                body: Some(json!({"spell1Id":4,"spell2Id":14})),
                status: 204,
                response: Value::Null,
            },
        ])
        .await;
        assert_eq!(client.import_draft_spells(&request()).await, Ok(()));
        server.await.unwrap();
    }
    #[tokio::test]
    async fn refuse_ancien_champion_mode_et_phase_sans_patch() {
        for (phase, queue, champion, error) in [
            (
                "InProgress",
                420,
                103,
                DraftRuneGuardError::DraftContextChanged,
            ),
            (
                "ChampSelect",
                720,
                103,
                DraftRuneGuardError::UnsupportedMode,
            ),
            (
                "ChampSelect",
                420,
                222,
                DraftRuneGuardError::DraftContextChanged,
            ),
        ] {
            let mut steps = vec![read("/lol-gameflow/v1/session", flow(phase, queue))];
            if phase == "ChampSelect" && queue == 420 {
                steps.push(read("/lol-champ-select/v1/session", draft(champion)))
            }
            let (client, server) = mock_client(steps).await;
            assert_eq!(
                client.import_draft_spells(&request()).await,
                Err(DraftRuneImportError::Guard(error))
            );
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn garde_le_contexte_puis_delegue_les_deux_champs_seulement_a_la_pr58() {
        let (client, server) = mock_client(vec![
            read("/lol-gameflow/v1/session", flow("ChampSelect", 420)),
            read("/lol-champ-select/v1/session", draft(103)),
            read("/lol-gameflow/v1/gameflow-phase", json!("ChampSelect")),
            ExpectedRequest {
                method: "PATCH",
                path: "/lol-champ-select/v1/session/my-selection".into(),
                body: Some(json!({"spell1Id":4,"spell2Id":14})),
                status: 204,
                response: Value::Null,
            },
        ])
        .await;
        assert_eq!(client.import_draft_spells(&request()).await, Ok(()));
        server.await.unwrap();
    }
}
