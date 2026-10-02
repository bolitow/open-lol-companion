//! Import de runes borné à la draft courante (#13).
use super::{ImportError, ImportRunesRequest};
use crate::draft::{DraftMode, FLOW_ENDPOINT};
use crate::{DraftSession, LcuClient, DRAFT_ENDPOINT};
use serde::{Deserialize, Serialize};

/// Garde de contexte du front #13, sans changer le contrat générique #14.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportDraftRunesRequest {
    pub champion_id: u32,
    pub runes: ImportRunesRequest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DraftRuneGuardError {
    DraftContextChanged,
    UnsupportedMode,
    ImportBusy,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum DraftRuneImportError {
    Client(ImportError),
    Guard(DraftRuneGuardError),
}
impl From<ImportError> for DraftRuneImportError {
    fn from(value: ImportError) -> Self {
        Self::Client(value)
    }
}
impl From<crate::ClientError> for DraftRuneImportError {
    fn from(value: crate::ClientError) -> Self {
        Self::Client(value.into())
    }
}

impl LcuClient {
    /// Valide le mode et le champion de la draft courante juste avant l’écriture.
    pub async fn import_draft_runes(
        &self,
        request: &ImportDraftRunesRequest,
    ) -> Result<(), DraftRuneImportError> {
        let prepared = self.prepare_runes(&request.runes).await?;
        // Les lectures des styles/pages précèdent la garde : vérifier le contexte au
        // dernier moment, après acquisition du verrou Tauri, sans importer une attente ancienne.
        self.require_draft_champion(request.champion_id).await?;
        prepared.apply(self).await?;
        Ok(())
    }
    pub(super) async fn require_draft_champion(
        &self,
        champion_id: u32,
    ) -> Result<(), DraftRuneImportError> {
        let flow: serde_json::Value = self.get_json(FLOW_ENDPOINT).await?;
        if flow["phase"] != "ChampSelect" || champion_id == 0 {
            return Err(DraftRuneImportError::Guard(
                DraftRuneGuardError::DraftContextChanged,
            ));
        }
        let mode = DraftMode::from_flow(&flow);
        if mode == DraftMode::Unsupported {
            return Err(DraftRuneImportError::Guard(
                DraftRuneGuardError::UnsupportedMode,
            ));
        }
        let raw = self.get_json(DRAFT_ENDPOINT).await?;
        let draft =
            DraftSession::parse_for_mode(raw, mode).ok_or(ImportError::InvalidClientData)?;
        let mut local = draft.allies.iter().filter(|p| p.local);
        let champion = local.next().and_then(|p| p.champion_id);
        if !draft.supported || local.next().is_some() || champion != Some(champion_id) {
            return Err(DraftRuneImportError::Guard(
                DraftRuneGuardError::DraftContextChanged,
            ));
        }
        Ok(())
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
    fn request() -> ImportDraftRunesRequest {
        ImportDraftRunesRequest {
            champion_id: 103,
            runes: super::super::ImportRunesRequest {
                champion_name: "Ahri".into(),
                primary_style_id: 8000,
                sub_style_id: 8200,
                selected_perk_ids: vec![8005, 9111, 9104, 8014, 8233, 8236, 5005, 5008, 5001],
            },
        }
    }
    fn preparation() -> Vec<ExpectedRequest> {
        vec![
            read(
                "/lol-perks/v1/styles",
                serde_json::from_str(include_str!("../../tests/fixtures/rune-styles.json"))
                    .unwrap(),
            ),
            read("/lol-perks/v1/pages", json!([])),
        ]
    }
    fn flow(phase: &str, queue: u32, mode: &str) -> Value {
        json!({"phase":phase,"gameData":{"queue":{"id":queue,"mapId":11,"gameMode":mode}}})
    }
    fn draft(champion: u32) -> Value {
        let mut d: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/champ-select-public.json"
        ))
        .unwrap();
        // La capture publique est un salon d'entraînement ; compléter en 5v5 pour ce scénario de draft.
        d["myTeam"] = json!((0..5).map(|cell|json!({"cellId":cell,"championId":0,"championPickIntent":0,"assignedPosition":"middle","team":1})).collect::<Vec<_>>());
        d["theirTeam"] = json!((5..10).map(|cell|json!({"cellId":cell,"championId":0,"championPickIntent":0,"assignedPosition":"","team":2})).collect::<Vec<_>>());
        let local = d["localPlayerCellId"].as_i64().unwrap();
        for p in d["myTeam"].as_array_mut().unwrap() {
            if p["cellId"].as_i64() == Some(local) {
                p["championId"] = json!(champion);
                p["championPickIntent"] = json!(champion)
            }
        }
        d
    }
    #[tokio::test]
    async fn refuse_phase_mode_et_file_inconnus_sans_ecrire() {
        for (phase, queue, mode) in [
            ("InProgress", 420, "CLASSIC"),
            ("ChampSelect", 720, "ARAM"),
            ("ChampSelect", 9999, "CLASSIC"),
            ("ChampSelect", 420, "ARAM"),
        ] {
            let mut steps = preparation();
            steps.push(read("/lol-gameflow/v1/session", flow(phase, queue, mode)));
            let (client, server) = mock_client(steps).await;
            assert!(client.import_draft_runes(&request()).await.is_err());
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn refuse_un_champion_different_juste_avant_ecriture() {
        let mut steps = preparation();
        steps.extend([
            read(
                "/lol-gameflow/v1/session",
                flow("ChampSelect", 420, "CLASSIC"),
            ),
            read("/lol-champ-select/v1/session", draft(222)),
        ]);
        let (client, server) = mock_client(steps).await;
        assert_eq!(
            client.import_draft_runes(&request()).await.unwrap_err(),
            DraftRuneImportError::Guard(DraftRuneGuardError::DraftContextChanged)
        );
        server.await.unwrap();
    }
    #[tokio::test]
    async fn importe_apres_verification_du_mode_et_du_champion() {
        let mut steps = preparation();
        steps.extend([read("/lol-gameflow/v1/session",flow("ChampSelect",420,"CLASSIC")),read("/lol-champ-select/v1/session",draft(103)),ExpectedRequest {method:"POST",path:"/lol-perks/v1/pages".into(),body:Some(json!({"name":"Open LoL Companion : Ahri","primaryStyleId":8000,"subStyleId":8200,"selectedPerkIds":[8005,9111,9104,8014,8233,8236,5005,5008,5001],"current":true})),status:204,response:Value::Null}]);
        let (client, server) = mock_client(steps).await;
        assert!(client.import_draft_runes(&request()).await.is_ok());
        server.await.unwrap();
    }
    #[tokio::test]
    async fn importe_les_runes_en_personnalisee_solo_identifiee() {
        let mut steps = preparation();
        let flow = json!({"phase":"ChampSelect","gameData":{"isCustomGame":true,"queue":{"id":0}},"map":{"id":11,"gameMode":"CLASSIC"}});
        let draft = json!({"localPlayerCellId":0,"myTeam":[{"cellId":0,"championId":103,"team":1}],"theirTeam":[],"actions":[]});
        steps.extend([read("/lol-gameflow/v1/session",flow),read("/lol-champ-select/v1/session",draft),ExpectedRequest {method:"POST",path:"/lol-perks/v1/pages".into(),body:Some(json!({"name":"Open LoL Companion : Ahri","primaryStyleId":8000,"subStyleId":8200,"selectedPerkIds":[8005,9111,9104,8014,8233,8236,5005,5008,5001],"current":true})),status:204,response:Value::Null}]);
        let (client, server) = mock_client(steps).await;
        assert_eq!(client.import_draft_runes(&request()).await, Ok(()));
        server.await.unwrap();
    }
    #[test]
    fn erreurs_publiques_sans_enveloppe_ni_reponse_brute() {
        assert_eq!(
            serde_json::to_value(DraftRuneImportError::Guard(
                DraftRuneGuardError::UnsupportedMode
            ))
            .unwrap(),
            json!("unsupportedMode")
        );
        assert_eq!(
            serde_json::to_value(DraftRuneImportError::Client(
                super::super::ImportError::InvalidRunes
            ))
            .unwrap(),
            json!("invalidRunes")
        );
    }
}
