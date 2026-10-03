use serde::{Deserialize, Serialize};

use super::ImportError;
use crate::LcuClient;

/// Emplacement du sort Flash choisi par l'utilisateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum FlashSlot {
    D,
    F,
}

/// Sorts choisis pour l'import, miroir de `ImportSpellsRequest` dans `@olc/shared`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSpellsRequest {
    pub spell_ids: [u32; 2],
    pub flash_slot: FlashSlot,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SpellSelection {
    spell1_id: u32,
    spell2_id: u32,
}

impl ImportSpellsRequest {
    pub(super) fn selection(&self) -> Result<SpellSelection, ImportError> {
        let [first, second] = self.spell_ids;
        if first == 0 || second == 0 || first == second {
            return Err(ImportError::InvalidSpells);
        }

        // Data Dragon identifie Flash par la clé 4 :
        // https://ddragon.leagueoflegends.com/cdn/16.19.1/data/en_US/summoner.json
        let swap = match self.flash_slot {
            FlashSlot::D => second == 4,
            FlashSlot::F => first == 4,
        };
        let [spell1_id, spell2_id] = if swap {
            [second, first]
        } else {
            [first, second]
        };
        Ok(SpellSelection {
            spell1_id,
            spell2_id,
        })
    }
}

impl LcuClient {
    /// Importe les deux sorts explicitement choisis pendant la sélection des champions.
    pub async fn import_spells(&self, request: &ImportSpellsRequest) -> Result<(), ImportError> {
        self.prepare_spells(request).await?.apply(self).await
    }
    pub(super) async fn prepare_spells(
        &self,
        request: &ImportSpellsRequest,
    ) -> Result<SpellSelection, ImportError> {
        let selection = request.selection()?;
        if !self.gameflow_phase().await?.is_champ_select() {
            return Err(ImportError::NotInChampSelect);
        }
        Ok(selection)
    }
}
impl SpellSelection {
    pub(super) fn matches(&self, pair: Option<[u32; 2]>) -> bool {
        pair == Some([self.spell1_id, self.spell2_id])
    }
    pub(super) async fn apply(&self, client: &LcuClient) -> Result<(), ImportError> {
        // Contrat LCU : seuls spell1Id (D) et spell2Id (F) sont modifiés.
        // https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json
        // https://amburgao.github.io/leaguewizard/reference/leaguewizard/core/models/#leaguewizard.core.models.PayloadSpells
        client
            .write_json(
                reqwest::Method::PATCH,
                "/lol-champ-select/v1/session/my-selection",
                self,
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::test_support::{mock_client, ExpectedRequest};

    #[test]
    fn place_flash_sur_d_independamment_de_l_ordre_du_build() {
        for spell_ids in [[4, 14], [14, 4]] {
            let request = ImportSpellsRequest {
                spell_ids,
                flash_slot: FlashSlot::D,
            };
            let selection = request.selection().unwrap();
            assert_eq!((selection.spell1_id, selection.spell2_id), (4, 14));
        }
    }

    #[test]
    fn place_flash_sur_f_independamment_de_l_ordre_du_build() {
        for spell_ids in [[4, 14], [14, 4]] {
            let request = ImportSpellsRequest {
                spell_ids,
                flash_slot: FlashSlot::F,
            };
            let selection = request.selection().unwrap();
            assert_eq!((selection.spell1_id, selection.spell2_id), (14, 4));
        }
    }

    #[test]
    fn conserve_l_ordre_d_un_build_sans_flash() {
        for flash_slot in [FlashSlot::D, FlashSlot::F] {
            let request = ImportSpellsRequest {
                spell_ids: [6, 14],
                flash_slot,
            };
            let selection = request.selection().unwrap();
            assert_eq!((selection.spell1_id, selection.spell2_id), (6, 14));
        }
    }

    #[test]
    fn refuse_les_sorts_nuls_ou_dupliques() {
        for spell_ids in [[0, 14], [4, 0], [0, 0], [4, 4], [14, 14]] {
            let request = ImportSpellsRequest {
                spell_ids,
                flash_slot: FlashSlot::D,
            };
            assert!(matches!(
                request.selection(),
                Err(ImportError::InvalidSpells)
            ));
        }
    }

    #[test]
    fn serialise_uniquement_les_deux_sorts_sans_modifier_le_skin() {
        let request = ImportSpellsRequest {
            spell_ids: [14, 4],
            flash_slot: FlashSlot::D,
        };
        assert_eq!(
            serde_json::to_value(request.selection().unwrap()).unwrap(),
            json!({ "spell1Id": 4, "spell2Id": 14 })
        );
    }

    #[test]
    fn respecte_le_contrat_de_la_requete_interface() {
        let value = json!({ "spellIds": [4, 14], "flashSlot": "F" });
        let request: ImportSpellsRequest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(request.spell_ids, [4, 14]);
        assert_eq!(request.flash_slot, FlashSlot::F);
        assert_eq!(serde_json::to_value(request).unwrap(), value);
    }

    #[test]
    fn refuse_une_position_flash_absente_ou_inconnue() {
        for value in [
            json!({ "spellIds": [4, 14] }),
            json!({ "spellIds": [4, 14], "flashSlot": "A" }),
        ] {
            assert!(serde_json::from_value::<ImportSpellsRequest>(value).is_err());
        }
    }

    #[tokio::test]
    async fn importe_les_sorts_en_selection_des_champions() {
        for (flash_slot, payload, status) in [
            (FlashSlot::D, json!({ "spell1Id": 4, "spell2Id": 14 }), 200),
            (FlashSlot::F, json!({ "spell1Id": 14, "spell2Id": 4 }), 204),
        ] {
            let (client, server) = mock_client(vec![
                ExpectedRequest {
                    method: "GET",
                    path: "/lol-gameflow/v1/gameflow-phase".into(),
                    body: None,
                    status: 200,
                    response: json!("ChampSelect"),
                },
                ExpectedRequest {
                    method: "PATCH",
                    path: "/lol-champ-select/v1/session/my-selection".into(),
                    body: Some(payload),
                    status,
                    response: json!({}),
                },
            ])
            .await;
            let request = ImportSpellsRequest {
                spell_ids: [14, 4],
                flash_slot,
            };
            client.import_spells(&request).await.unwrap();
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn refuse_l_import_hors_selection_des_champions() {
        let (client, server) = mock_client(vec![ExpectedRequest {
            method: "GET",
            path: "/lol-gameflow/v1/gameflow-phase".into(),
            body: None,
            status: 200,
            response: json!("InProgress"),
        }])
        .await;
        let request = ImportSpellsRequest {
            spell_ids: [4, 14],
            flash_slot: FlashSlot::F,
        };
        assert_eq!(
            client.import_spells(&request).await,
            Err(ImportError::NotInChampSelect)
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn valide_les_sorts_avant_toute_requete_reseau() {
        let (client, server) = mock_client(vec![]).await;
        server.await.unwrap();
        for spell_ids in [[0, 14], [4, 4]] {
            let request = ImportSpellsRequest {
                spell_ids,
                flash_slot: FlashSlot::D,
            };
            assert_eq!(
                client.import_spells(&request).await,
                Err(ImportError::InvalidSpells)
            );
        }
    }

    #[tokio::test]
    async fn propage_le_refus_du_client_sans_reponse_brute() {
        let (client, server) = mock_client(vec![
            ExpectedRequest {
                method: "GET",
                path: "/lol-gameflow/v1/gameflow-phase".into(),
                body: None,
                status: 200,
                response: json!("ChampSelect"),
            },
            ExpectedRequest {
                method: "PATCH",
                path: "/lol-champ-select/v1/session/my-selection".into(),
                body: Some(json!({ "spell1Id": 4, "spell2Id": 14 })),
                status: 400,
                response: json!({ "message": "détail interne du client" }),
            },
        ])
        .await;
        let request = ImportSpellsRequest {
            spell_ids: [4, 14],
            flash_slot: FlashSlot::D,
        };
        assert_eq!(
            client.import_spells(&request).await,
            Err(ImportError::ClientRejected)
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn refuse_une_phase_client_invalide() {
        let (client, server) = mock_client(vec![ExpectedRequest {
            method: "GET",
            path: "/lol-gameflow/v1/gameflow-phase".into(),
            body: None,
            status: 200,
            response: json!({}),
        }])
        .await;
        let request = ImportSpellsRequest {
            spell_ids: [4, 14],
            flash_slot: FlashSlot::D,
        };
        assert_eq!(
            client.import_spells(&request).await,
            Err(ImportError::InvalidClientData)
        );
        server.await.unwrap();
    }
}
