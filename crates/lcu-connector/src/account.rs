use serde::Serialize;
use serde_json::Value;

/// Identité publique du compte connecté ; aucun identifiant technique ni secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LcuAccount {
    pub platform: String,
    pub game_name: String,
    pub tag_line: String,
    pub profile_icon_id: Option<u32>,
}

impl LcuAccount {
    pub(crate) fn parse(summoner: &Value, region: &Value) -> Option<Self> {
        if summoner.get("unnamed").and_then(Value::as_bool) == Some(true) {
            return None;
        }
        let game_name = summoner.get("gameName")?.as_str()?;
        let tag_line = summoner.get("tagLine")?.as_str()?;
        let valid = |text: &str, max| {
            !text.is_empty()
                && text.len() <= max
                && text.trim() == text
                && text != "."
                && text != ".."
                && !text.chars().any(|c| c.is_control() || c == '#')
        };
        if !valid(game_name, 64) || !valid(tag_line, 32) {
            return None;
        }
        // La région du client est indépendante du tag personnel du Riot ID.
        let platform = match region.get("region")?.as_str()? {
            "EUW" | "EUW1" => "EUW1",
            "EUNE" | "EUN1" => "EUN1",
            "NA" | "NA1" => "NA1",
            "BR" | "BR1" => "BR1",
            "JP" | "JP1" => "JP1",
            "LA1" => "LA1",
            "LA2" => "LA2",
            "OCE" | "OC1" => "OC1",
            "TR" | "TR1" => "TR1",
            "RU" => "RU",
            "KR" => "KR",
            "ME" | "ME1" => "ME1",
            "SG" | "SG2" => "SG2",
            "TW" | "TW2" => "TW2",
            "VN" | "VN2" => "VN2",
            _ => return None,
        };
        Some(Self {
            platform: platform.into(),
            game_name: game_name.into(),
            tag_line: tag_line.into(),
            profile_icon_id: summoner
                .get("profileIconId")
                .and_then(Value::as_u64)
                .and_then(|id| u32::try_from(id).ok()),
        })
    }
}

// Contrats consultés le 2 octobre 2026 :
// https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json
pub(crate) const ACCOUNT_ENDPOINT: &str = "/lol-summoner/v1/current-summoner";
pub(crate) const REGION_ENDPOINT: &str = "/riotclient/region-locale";

pub(crate) async fn read_account(client: &crate::LcuClient) -> Option<LcuAccount> {
    // La relecture détecte un Riot ID qui change entre les GET ; la LCU n'offre pas de snapshot atomique.
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let first: Value = client.get_json(ACCOUNT_ENDPOINT).await.ok()?;
        let region: Value = client.get_json(REGION_ENDPOINT).await.ok()?;
        let before = LcuAccount::parse(&first, &region)?;
        let last: Value = client.get_json(ACCOUNT_ENDPOINT).await.ok()?;
        let after = LcuAccount::parse(&last, &region)?;
        (before == after).then_some(after)
    })
    .await
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{mock_client, ExpectedRequest};
    use serde_json::json;
    fn get(path: &str, response: Value) -> ExpectedRequest {
        ExpectedRequest {
            method: "GET",
            path: path.into(),
            body: None,
            status: 200,
            response,
        }
    }
    #[tokio::test]
    async fn lit_un_compte_coherent_et_refuse_un_changement_pendant_la_lecture() {
        let a = json!({"gameName":"Alpha","tagLine":"TAG"});
        let b = json!({"gameName":"Beta","tagLine":"TAG"});
        let region = json!({"region":"EUW"});
        for last in [a.clone(), b] {
            let (client, server) = mock_client(vec![
                get(ACCOUNT_ENDPOINT, a.clone()),
                get(REGION_ENDPOINT, region.clone()),
                get(ACCOUNT_ENDPOINT, last.clone()),
            ])
            .await;
            let actual = read_account(&client).await;
            assert_eq!(
                actual,
                LcuAccount::parse(&last, &region).filter(|_| last == a)
            );
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn une_erreur_client_ne_produit_pas_de_compte_fictif() {
        let mut request = get(ACCOUNT_ENDPOINT, json!({"message":"unavailable"}));
        request.status = 404;
        let (client, server) = mock_client(vec![request]).await;
        assert!(read_account(&client).await.is_none());
        server.await.unwrap();
    }

    #[test]
    fn projette_uniquement_le_riot_id_et_la_plateforme_sans_inferer_le_tag() {
        let value = json!({"gameName":"First Player", "tagLine":"NA1", "puuid":"fixture-private", "accountId":123});
        let account =
            LcuAccount::parse(&value, &json!({"region":"EUW", "locale":"en_US"})).unwrap();
        assert_eq!(
            serde_json::to_value(account).unwrap(),
            json!({"platform":"EUW1","game_name":"First Player","tag_line":"NA1","profile_icon_id":null})
        );
    }
    #[test]
    fn conserve_uniquement_une_icone_publique_valide() {
        for (value, expected) in [
            (json!(0), Some(0)),
            (json!(42), Some(42)),
            (json!(-1), None),
            (json!(4294967296u64), None),
            (json!("42"), None),
            (Value::Null, None),
        ] {
            let account = LcuAccount::parse(
                &json!({"gameName":"Name","tagLine":"TAG","profileIconId":value}),
                &json!({"region":"EUW"}),
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(account).unwrap()["profile_icon_id"],
                json!(expected)
            );
        }
    }

    #[test]
    fn refuse_les_donnees_incompletes_et_regions_non_prises_en_charge() {
        for value in [
            json!({}),
            json!({"gameName":"Name","tagLine":""}),
            json!({"gameName":"..","tagLine":"TAG"}),
            json!({"gameName":"A\nB","tagLine":"TAG"}),
            json!({"gameName":"é".repeat(33),"tagLine":"TAG"}),
            json!({"gameName":"Name","tagLine":"TAG","unnamed":true}),
        ] {
            assert!(LcuAccount::parse(&value, &json!({"region":"EUW"})).is_none());
        }
        let value = json!({"gameName":"Name","tagLine":"TAG"});
        for region in ["PBE", "UNKNOWN", ""] {
            assert!(LcuAccount::parse(&value, &json!({"region":region})).is_none());
        }
        for (region, platform) in [
            ("EUNE", "EUN1"),
            ("NA", "NA1"),
            ("KR", "KR"),
            ("EUW1", "EUW1"),
            ("OC1", "OC1"),
            ("TR", "TR1"),
        ] {
            assert_eq!(
                LcuAccount::parse(&value, &json!({"region":region}))
                    .unwrap()
                    .platform,
                platform
            );
        }
    }
}
