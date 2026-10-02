//! Page de runes équipée : projection en lecture seule de la ressource courante.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const RUNES_ENDPOINT: &str = "/lol-perks/v1/currentpage";

/// DTO explicitement limité : aucun nom, identifiant de page ou de compte.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunePage {
    pub primary_style_id: u32,
    pub sub_style_id: u32,
    pub selected_perk_ids: Vec<u32>,
    pub is_valid: bool,
    pub is_temporary: bool,
    #[serde(default)]
    pub auto_modified_selections: Vec<u32>,
}
impl RunePage {
    pub fn parse(value: Value) -> Option<Self> {
        let page: Self = serde_json::from_value(value).ok()?;
        (page.selected_perk_ids.len() <= 9 && page.auto_modified_selections.len() <= 9)
            .then_some(page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn page() -> serde_json::Value {
        json!({"id":12,"name":"Ne pas transmettre", "primaryStyleId":8000,
            "subStyleId":8200,"selectedPerkIds":[8005,8009,9104,8014,8210,8236,5008,5008,5011],
            "isValid":true,"isTemporary":false,"autoModifiedSelections":[],"isActive":false})
    }
    #[test]
    fn conserve_ordre_et_fragments_repetes_sans_identifiants_prives() {
        let p = RunePage::parse(page()).unwrap();
        assert_eq!(
            p.selected_perk_ids,
            vec![8005, 8009, 9104, 8014, 8210, 8236, 5008, 5008, 5011]
        );
        assert_eq!(
            serde_json::to_value(p).unwrap(),
            json!({"primaryStyleId":8000,"subStyleId":8200,
            "selectedPerkIds":[8005,8009,9104,8014,8210,8236,5008,5008,5011],
            "isValid":true,"isTemporary":false,"autoModifiedSelections":[]})
        );
    }
    #[test]
    fn accepte_une_page_incomplete_sans_fabriquer_de_choix() {
        let mut p = page();
        p["isValid"] = json!(false);
        p["selectedPerkIds"] = json!([0, 0]);
        assert!(!RunePage::parse(p).unwrap().is_valid);
    }
    #[test]
    fn refuse_absence_types_incorrects_et_tableaux_demesures() {
        for p in [
            json!(null),
            json!({}),
            json!({"primaryStyleId":"8000"}),
            json!([]),
        ] {
            assert!(RunePage::parse(p).is_none());
        }
        for ids in [
            json!([1, 2, -1]),
            json!(["8005"]),
            json!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
        ] {
            let mut p = page();
            p["selectedPerkIds"] = ids;
            assert!(RunePage::parse(p).is_none());
        }
    }
}
