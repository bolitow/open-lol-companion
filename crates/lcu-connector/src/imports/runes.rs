use std::collections::HashSet;

use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::{ImportError, APP_NAME};
use crate::LcuClient;

// Endpoints et schémas vérifiés le 01/10/2026 dans la capture du client :
// https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json
const STYLES_ENDPOINT: &str = "/lol-perks/v1/styles";
const PAGES_ENDPOINT: &str = "/lol-perks/v1/pages";

/// Runes choisies par l'utilisateur, dans l'ordre primaire, secondaire, fragments.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRunesRequest {
    pub champion_name: String,
    pub primary_style_id: u32,
    pub sub_style_id: u32,
    pub selected_perk_ids: Vec<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuneStyle {
    id: u32,
    allowed_sub_styles: Vec<u32>,
    slots: Vec<RuneSlot>,
}

#[derive(Deserialize)]
struct RuneSlot {
    #[serde(rename = "type")]
    kind: String,
    perks: Vec<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunePage {
    id: i32,
    name: String,
    is_editable: bool,
}

fn validate_runes(request: &ImportRunesRequest, styles: &[RuneStyle]) -> Result<(), ImportError> {
    if request.champion_name.trim().is_empty()
        || request.champion_name.chars().any(char::is_control)
        || request.selected_perk_ids.len() != 9
        || request.primary_style_id == request.sub_style_id
    {
        return Err(ImportError::InvalidRunes);
    }
    let mut ids = HashSet::new();
    if styles.iter().any(|style| !ids.insert(style.id)) {
        return Err(ImportError::InvalidClientData);
    }
    let primary = styles
        .iter()
        .find(|style| style.id == request.primary_style_id)
        .ok_or(ImportError::InvalidRunes)?;
    let secondary = styles
        .iter()
        .find(|style| style.id == request.sub_style_id)
        .ok_or(ImportError::InvalidRunes)?;
    if !primary.allowed_sub_styles.contains(&secondary.id) {
        return Err(ImportError::InvalidRunes);
    }
    let expected_slots = [
        "kKeyStone",
        "kMixedRegularSplashable",
        "kMixedRegularSplashable",
        "kMixedRegularSplashable",
        "kStatMod",
        "kStatMod",
        "kStatMod",
    ];
    for style in [primary, secondary] {
        if style.slots.len() != expected_slots.len()
            || style
                .slots
                .iter()
                .zip(expected_slots)
                .any(|(slot, kind)| slot.kind != kind || slot.perks.is_empty())
        {
            return Err(ImportError::InvalidClientData);
        }
    }

    let perks = &request.selected_perk_ids;
    if primary.slots[..4]
        .iter()
        .zip(&perks[..4])
        .chain(primary.slots[4..].iter().zip(&perks[6..]))
        .any(|(slot, perk)| !slot.perks.contains(perk))
    {
        return Err(ImportError::InvalidRunes);
    }
    // Deux secondaires doivent provenir de deux lignes distinctes ; les fragments
    // peuvent au contraire répéter un identifiant présent dans plusieurs lignes.
    let first_row = secondary.slots[1..4]
        .iter()
        .position(|slot| slot.perks.contains(&perks[4]))
        .ok_or(ImportError::InvalidRunes)?;
    let second_row = secondary.slots[1..4]
        .iter()
        .position(|slot| slot.perks.contains(&perks[5]))
        .ok_or(ImportError::InvalidRunes)?;
    if first_row == second_row {
        return Err(ImportError::InvalidRunes);
    }
    Ok(())
}

fn owned_page(pages: &[RunePage]) -> Result<Option<i32>, ImportError> {
    let prefix = format!("{APP_NAME} : ");
    let mut id = None;
    for page in pages.iter().filter(|page| page.name.starts_with(&prefix)) {
        if id.is_some() || !page.is_editable || page.id <= 0 {
            return Err(ImportError::RunePageUnavailable);
        }
        id = Some(page.id);
    }
    Ok(id)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunePageWrite<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<i32>,
    name: String,
    primary_style_id: u32,
    sub_style_id: u32,
    selected_perk_ids: &'a [u32],
    current: bool,
}

pub(super) struct PreparedRunes<'a> {
    method: Method,
    path: String,
    page: RunePageWrite<'a>,
}
impl PreparedRunes<'_> {
    pub(super) async fn apply(self, client: &LcuClient) -> Result<(), ImportError> {
        client
            .write_json(self.method, &self.path, &self.page)
            .await?;
        Ok(())
    }
}

impl LcuClient {
    /// Valide les runes avec le catalogue du client puis importe la page réservée à l'app.
    /// Une page personnelle n'est jamais supprimée pour libérer un emplacement.
    pub async fn import_runes(&self, request: &ImportRunesRequest) -> Result<(), ImportError> {
        self.prepare_runes(request).await?.apply(self).await
    }

    pub(super) async fn prepare_runes<'a>(
        &self,
        request: &'a ImportRunesRequest,
    ) -> Result<PreparedRunes<'a>, ImportError> {
        let styles: Vec<RuneStyle> = self.get_json(STYLES_ENDPOINT).await?;
        validate_runes(request, &styles)?;
        let pages: Vec<RunePage> = self.get_json(PAGES_ENDPOINT).await?;
        let id = owned_page(&pages)?;
        let page = RunePageWrite {
            id,
            name: format!("{APP_NAME} : {}", request.champion_name.trim()),
            primary_style_id: request.primary_style_id,
            sub_style_id: request.sub_style_id,
            selected_perk_ids: &request.selected_perk_ids,
            current: true,
        };
        let (method, path) = match id {
            Some(id) => (Method::PUT, format!("{PAGES_ENDPOINT}/{id}")),
            None => (Method::POST, PAGES_ENDPOINT.into()),
        };
        Ok(PreparedRunes { method, path, page })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::test_support::{mock_client, ExpectedRequest};

    fn styles_json() -> Value {
        serde_json::from_str(include_str!("../../tests/fixtures/rune-styles.json")).unwrap()
    }

    fn request() -> ImportRunesRequest {
        ImportRunesRequest {
            champion_name: "Jinx".into(),
            primary_style_id: 8000,
            sub_style_id: 8200,
            selected_perk_ids: vec![8005, 9111, 9104, 8014, 8233, 8236, 5005, 5008, 5001],
        }
    }

    fn validate(request: &ImportRunesRequest) -> Result<(), ImportError> {
        validate_runes(
            request,
            &serde_json::from_value::<Vec<RuneStyle>>(styles_json()).unwrap(),
        )
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

    fn body() -> Value {
        json!({
            "name":"Open LoL Companion : Jinx", "primaryStyleId":8000,
            "subStyleId":8200, "selectedPerkIds":[8005,9111,9104,8014,8233,8236,5005,5008,5001],
            "current":true
        })
    }

    #[test]
    fn accepte_une_selection_complete_du_catalogue() {
        assert!(validate(&request()).is_ok());
    }

    #[test]
    fn refuse_deux_secondaires_de_la_meme_ligne() {
        let mut input = request();
        input.selected_perk_ids[5] = 8210;
        assert!(matches!(validate(&input), Err(ImportError::InvalidRunes)));
    }

    #[test]
    fn refuse_une_cle_de_voute_en_secondaire_et_un_identifiant_inconnu() {
        for invalid in [8214, 999999, 5008, 9111] {
            let mut input = request();
            input.selected_perk_ids[4] = invalid;
            assert!(matches!(validate(&input), Err(ImportError::InvalidRunes)));
        }
    }

    #[test]
    fn refuse_une_primaire_ou_un_fragment_dans_la_mauvaise_ligne() {
        for (index, invalid) in [(0, 9111), (1, 9104), (6, 5001), (7, 5005), (8, 5008)] {
            let mut input = request();
            input.selected_perk_ids[index] = invalid;
            assert!(matches!(validate(&input), Err(ImportError::InvalidRunes)));
        }
    }

    #[test]
    fn autorise_les_fragments_repetes_sur_des_lignes_compatibles() {
        let mut input = request();
        input.selected_perk_ids[6] = 5008;
        assert!(validate(&input).is_ok());
        input.selected_perk_ids[7] = 5001;
        assert!(validate(&input).is_ok());
    }

    #[test]
    fn refuse_les_styles_inconnus_identiques_ou_incompatibles() {
        for (primary, secondary) in [(0, 8200), (8000, 0), (8000, 8000)] {
            let mut input = request();
            input.primary_style_id = primary;
            input.sub_style_id = secondary;
            assert!(matches!(validate(&input), Err(ImportError::InvalidRunes)));
        }
        let mut styles: Vec<RuneStyle> = serde_json::from_value(styles_json()).unwrap();
        styles[0].allowed_sub_styles.clear();
        assert!(matches!(
            validate_runes(&request(), &styles),
            Err(ImportError::InvalidRunes)
        ));
    }

    #[test]
    fn refuse_les_selections_incompletes_ou_excedentaires() {
        for count in [0, 8, 10] {
            let mut input = request();
            input.selected_perk_ids.resize(count, 5001);
            assert!(matches!(validate(&input), Err(ImportError::InvalidRunes)));
        }
    }

    #[test]
    fn refuse_un_nom_de_champion_vide_ou_de_controle() {
        for name in ["", "   ", "Jinx\n"] {
            let mut input = request();
            input.champion_name = name.into();
            assert!(matches!(validate(&input), Err(ImportError::InvalidRunes)));
        }
    }

    #[test]
    fn refuse_un_catalogue_ambigu_ou_une_structure_de_lignes_inconnue() {
        let mut duplicate = styles_json();
        duplicate
            .as_array_mut()
            .unwrap()
            .push(styles_json()[0].clone());
        let mut unknown = styles_json();
        unknown[0]["slots"][0]["type"] = json!("unknown");
        let mut incomplete = styles_json();
        incomplete[0]["slots"].as_array_mut().unwrap().pop();
        for catalog in [duplicate, unknown, incomplete] {
            let styles: Vec<RuneStyle> = serde_json::from_value(catalog).unwrap();
            assert!(matches!(
                validate_runes(&request(), &styles),
                Err(ImportError::InvalidClientData)
            ));
        }
    }

    #[test]
    fn remplace_seulement_la_page_au_prefixe_reserve_exact() {
        let pages: Vec<RunePage> = serde_json::from_value(json!([
            {"id":1,"name":"Personnel","isEditable":true},
            {"id":2,"name":"Copie Open LoL Companion : Jinx","isEditable":true},
            {"id":3,"name":"Open LoL Companion : Ahri","isEditable":true}
        ]))
        .unwrap();
        assert_eq!(owned_page(&pages).unwrap(), Some(3));
    }

    #[test]
    fn refuse_les_pages_reservees_ambigues_verrouillees_ou_sans_identifiant_valide() {
        for pages in [
            json!([
                {"id":1,"name":"Open LoL Companion : Ahri","isEditable":true},
                {"id":2,"name":"Open LoL Companion : Jinx","isEditable":true}
            ]),
            json!([{"id":1,"name":"Open LoL Companion : Ahri","isEditable":false}]),
            json!([{"id":0,"name":"Open LoL Companion : Ahri","isEditable":true}]),
        ] {
            let pages: Vec<RunePage> = serde_json::from_value(pages).unwrap();
            assert!(matches!(
                owned_page(&pages),
                Err(ImportError::RunePageUnavailable)
            ));
        }
    }

    #[tokio::test]
    async fn cree_une_page_active_sans_toucher_aux_pages_personnelles() {
        let (client, server) = mock_client(vec![
            get("/lol-perks/v1/styles", styles_json()),
            get(
                "/lol-perks/v1/pages",
                json!([
                    {"id":1,"name":"Personnel","isEditable":true},
                    {"id":2,"name":"Open LoL CompanionX : Jinx","isEditable":true},
                    {"id":-1,"name":"Préréglage","isEditable":false}
                ]),
            ),
            ExpectedRequest {
                method: "POST",
                path: "/lol-perks/v1/pages".into(),
                body: Some(body()),
                status: 200,
                response: json!({"id":3}),
            },
        ])
        .await;
        assert!(client.import_runes(&request()).await.is_ok());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn met_a_jour_la_page_reservee_sans_la_supprimer() {
        let mut expected = body();
        expected["id"] = json!(7);
        let (client, server) = mock_client(vec![
            get("/lol-perks/v1/styles", styles_json()),
            get(
                "/lol-perks/v1/pages",
                json!([
                    {"id":2,"name":"Personnel","isEditable":true},
                    {"id":7,"name":"Open LoL Companion : Ahri","isEditable":true}
                ]),
            ),
            ExpectedRequest {
                method: "PUT",
                path: "/lol-perks/v1/pages/7".into(),
                body: Some(expected),
                status: 204,
                response: Value::Null,
            },
        ])
        .await;
        assert!(client.import_runes(&request()).await.is_ok());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn refuse_la_selection_invalide_avant_la_lecture_des_pages() {
        let (client, server) = mock_client(vec![get("/lol-perks/v1/styles", styles_json())]).await;
        let mut input = request();
        input.selected_perk_ids[5] = 8210;
        assert!(matches!(
            client.import_runes(&input).await,
            Err(ImportError::InvalidRunes)
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn propage_le_refus_de_creation_sans_supprimer_une_page_personnelle() {
        let (client, server) = mock_client(vec![
            get("/lol-perks/v1/styles", styles_json()),
            get(
                "/lol-perks/v1/pages",
                json!([{"id":2,"name":"Personnel","isEditable":true}]),
            ),
            ExpectedRequest {
                method: "POST",
                path: "/lol-perks/v1/pages".into(),
                body: Some(body()),
                status: 400,
                response: json!({"message":"inventaire plein"}),
            },
        ])
        .await;
        assert!(matches!(
            client.import_runes(&request()).await,
            Err(ImportError::ClientRejected)
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn refuse_un_catalogue_malforme_sans_ecriture() {
        let (client, server) =
            mock_client(vec![get("/lol-perks/v1/styles", json!([{"id":8000}]))]).await;
        assert!(matches!(
            client.import_runes(&request()).await,
            Err(ImportError::InvalidClientData)
        ));
        server.await.unwrap();
    }
}
