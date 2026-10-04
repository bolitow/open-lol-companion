//! Version du jeu installée, lue dans le client LoL pour choisir le patch des données.

use serde::Serialize;
use serde_json::Value;

// Contrat consulté le 4 octobre 2026 (réponse 200 : chaîne JSON) :
// https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json
pub const GAME_VERSION_ENDPOINT: &str = "/lol-patch/v1/game-version";

/// Version technique du jeu et patch `majeur.mineur` qui en est tiré.
/// Miroir exact de `ClientPatch` dans @olc/shared. Aucun libellé public ni
/// version Data Dragon n'est déduit : ils viennent d'autres sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPatch {
    pub game_version: String,
    pub patch: String,
}

/// Codes traduits par l'interface ; aucun détail de transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientPatchError {
    Unavailable,
    InvalidResponse,
}

impl ClientPatch {
    /// Le format exact au-delà de `majeur.mineur` n'est pas documenté : la suite
    /// reste conservée telle quelle, mais bornée et sans espace ni caractère de contrôle.
    pub fn parse(value: &Value) -> Option<Self> {
        let raw = value.as_str()?;
        if raw.is_empty() || raw.len() > 256 || !raw.bytes().all(|b| b.is_ascii_graphic()) {
            return None;
        }
        let mut parts = raw.split('.');
        let mut number = || {
            let part = parts.next()?;
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            part.parse::<u32>().ok()
        };
        let (major, minor) = (number()?, number()?);
        if parts.any(str::is_empty) {
            return None;
        }
        Some(Self {
            game_version: raw.into(),
            patch: format!("{major}.{minor}"),
        })
    }
}

/// Lecture unique, bornée dans le temps comme la lecture du compte actif.
pub async fn read_client_patch(client: &crate::LcuClient) -> Result<ClientPatch, ClientPatchError> {
    let value = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        client.get_json::<Value>(GAME_VERSION_ENDPOINT),
    )
    .await
    .map_err(|_| ClientPatchError::Unavailable)?
    .map_err(|_| ClientPatchError::Unavailable)?;
    // Une chaîne vide (client en cours de mise à jour) n'invente aucun patch.
    ClientPatch::parse(&value).ok_or(ClientPatchError::InvalidResponse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{mock_client, ExpectedRequest};
    use serde_json::json;

    fn get(status: u16, response: Value) -> ExpectedRequest {
        ExpectedRequest {
            method: "GET",
            path: GAME_VERSION_ENDPOINT.into(),
            body: None,
            status,
            response,
        }
    }

    #[test]
    fn tire_le_patch_majeur_mineur_et_conserve_la_version_brute() {
        for (raw, patch) in [
            ("16.19.715.1234", "16.19"),
            ("16.20.1.1", "16.20"),
            ("16.1", "16.1"),
            ("16.19.8230722+branch.releases-16-19.code.public.content.release.anticheat.vanguard", "16.19"),
        ] {
            assert_eq!(
                ClientPatch::parse(&json!(raw)),
                Some(ClientPatch {
                    game_version: raw.into(),
                    patch: patch.into(),
                })
            );
        }
    }

    #[test]
    fn serialise_le_contrat_public_en_camel_case() {
        let patch = ClientPatch::parse(&json!("16.19.715.1234")).unwrap();
        assert_eq!(
            serde_json::to_value(patch).unwrap(),
            json!({"gameVersion":"16.19.715.1234","patch":"16.19"})
        );
        assert_eq!(
            serde_json::to_value(ClientPatchError::InvalidResponse).unwrap(),
            json!("invalid_response")
        );
    }

    #[test]
    fn refuse_une_version_vide_ou_incoherente_sans_patch_invente() {
        for value in [
            json!(null),
            json!({}),
            json!(16.19),
            json!(""),
            json!("16"),
            json!("abc"),
            json!(".19.1"),
            json!("16..1"),
            json!("16.19."),
            json!("16.x.1"),
            json!("+16.19"),
            json!(" 16.19.1"),
            json!("16.19.1\n"),
            json!("99999999999.1"),
            json!(format!("16.19.{}", "1".repeat(256))),
        ] {
            assert!(ClientPatch::parse(&value).is_none(), "accepté : {value}");
        }
    }

    #[tokio::test]
    async fn lit_la_version_du_client_local() {
        let (client, server) = mock_client(vec![get(200, json!("16.19.715.1234"))]).await;
        assert_eq!(
            read_client_patch(&client).await,
            Ok(ClientPatch {
                game_version: "16.19.715.1234".into(),
                patch: "16.19".into(),
            })
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn distingue_client_indisponible_et_reponse_invalide() {
        let (client, server) = mock_client(vec![get(404, json!({"message":"absent"}))]).await;
        assert_eq!(
            read_client_patch(&client).await,
            Err(ClientPatchError::Unavailable)
        );
        server.await.unwrap();
        let (client, server) = mock_client(vec![get(200, json!(""))]).await;
        assert_eq!(
            read_client_patch(&client).await,
            Err(ClientPatchError::InvalidResponse)
        );
        server.await.unwrap();
    }
}
