//! Messages WAMP 1.0 du WebSocket de la League Client API.
//!
//! Le client n'implémente qu'une petite partie du protocole : on s'abonne avec
//! `[5, "OnJsonApiEvent_<uri>"]` et on reçoit `[8, topic, {data, eventType, uri}]`.

use serde::Deserialize;
use serde_json::Value;

const SUBSCRIBE: u8 = 5;
const EVENT: u8 = 8;

/// Événement de l'API locale : une ressource `uri` a été créée, modifiée ou supprimée.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct JsonApiEvent {
    pub uri: String,
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub data: Value,
}

/// Message d'abonnement aux événements d'un endpoint, ex. `/lol-gameflow/v1/gameflow-phase`
/// → `[5,"OnJsonApiEvent_lol-gameflow_v1_gameflow-phase"]`.
pub fn subscribe_message(uri: &str) -> String {
    let topic = format!("OnJsonApiEvent{}", uri.replace('/', "_"));
    serde_json::json!([SUBSCRIBE, topic]).to_string()
}

/// Extrait l'événement d'un message reçu ; `None` pour tout autre message (bienvenue, erreurs…).
pub fn parse_event(text: &str) -> Option<JsonApiEvent> {
    let (kind, _topic, payload): (u8, String, JsonApiEvent) = serde_json::from_str(text).ok()?;
    (kind == EVENT).then_some(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construit_le_topic_d_abonnement() {
        assert_eq!(
            subscribe_message("/lol-gameflow/v1/gameflow-phase"),
            r#"[5,"OnJsonApiEvent_lol-gameflow_v1_gameflow-phase"]"#
        );
    }

    #[test]
    fn lit_un_evenement_du_client() {
        let msg = r#"[8,"OnJsonApiEvent_lol-gameflow_v1_gameflow-phase",{"data":"ChampSelect","eventType":"Update","uri":"/lol-gameflow/v1/gameflow-phase"}]"#;
        let e = parse_event(msg).unwrap();
        assert_eq!(e.uri, "/lol-gameflow/v1/gameflow-phase");
        assert_eq!(e.event_type, "Update");
        assert_eq!(e.data, Value::from("ChampSelect"));
    }

    #[test]
    fn ignore_les_autres_messages() {
        assert_eq!(parse_event(r#"[0,"session",1,"server"]"#), None);
        assert_eq!(
            parse_event(r#"[4,"OnJsonApiEvent",{"uri":"/x","eventType":"Update","data":1}]"#),
            None
        );
        assert_eq!(parse_event(""), None);
        assert_eq!(parse_event("pas du json"), None);
    }
}
