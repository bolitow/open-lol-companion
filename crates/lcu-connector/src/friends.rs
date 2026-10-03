//! Projection minimale des amis du client, sans identifiant technique ni persistance.

use serde::Serialize;
use serde_json::Value;

use crate::{account::read_account, LcuAccount, LcuClient};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FriendsStatus {
    #[default]
    Disconnected,
    Loading,
    Ready,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FriendPresence {
    Online,
    Away,
    Busy,
    Offline,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Friend {
    pub name: String,
    pub game_name: Option<String>,
    pub tag_line: Option<String>,
    pub platform: Option<String>,
    pub icon_id: Option<u32>,
    pub presence: FriendPresence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FriendsSnapshot {
    pub status: FriendsStatus,
    pub items: Vec<Friend>,
}

// Contrat LolChatFriendResource / LolChatSessionState consulté le 2 octobre 2026 :
// https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json
const FRIENDS_ENDPOINT: &str = "/lol-chat/v1/friends";
const SESSION_ENDPOINT: &str = "/lol-chat/v1/session";

fn text(value: Option<&Value>, limit: usize) -> Option<&str> {
    value.and_then(Value::as_str).filter(|text| {
        !text.is_empty()
            && text.len() <= limit
            && text.trim() == *text
            && !text.chars().any(char::is_control)
    })
}

fn riot_id_part(value: Option<&Value>, limit: usize) -> Option<&str> {
    text(value, limit).filter(|part| *part != "." && *part != ".." && !part.contains('#'))
}

fn parse_friend(value: &Value) -> Option<Friend> {
    let game_name = riot_id_part(value.get("gameName"), 64);
    let tag_line = riot_id_part(value.get("gameTag"), 32);
    let name = game_name.or_else(|| text(value.get("name"), 128))?;
    let riot_id = game_name.zip(tag_line);
    // Une plateforme absente n'est jamais remplacée par celle du compte connecté.
    let platform = value
        .get("platformId")
        .and_then(Value::as_str)
        .filter(|platform| {
            matches!(
                *platform,
                "EUW1"
                    | "EUN1"
                    | "NA1"
                    | "BR1"
                    | "JP1"
                    | "LA1"
                    | "LA2"
                    | "OC1"
                    | "TR1"
                    | "RU"
                    | "KR"
                    | "ME1"
                    | "SG2"
                    | "TW2"
                    | "VN2"
            )
        })
        .map(str::to_owned);
    // Valeurs vérifiées par GET local le 2 octobre 2026, sans conserver de réponse nominative.
    // Le schéma déclare une chaîne libre : toute nouvelle valeur reste inconnue.
    let presence = match value.get("availability").and_then(Value::as_str) {
        Some("chat") => FriendPresence::Online,
        Some("away") => FriendPresence::Away,
        Some("dnd") => FriendPresence::Busy,
        Some("offline") => FriendPresence::Offline,
        _ => FriendPresence::Unknown,
    };
    Some(Friend {
        name: name.into(),
        game_name: riot_id.map(|(name, _)| name.into()),
        tag_line: riot_id.map(|(_, tag)| tag.into()),
        platform,
        icon_id: value
            .get("icon")
            .and_then(Value::as_u64)
            .and_then(|icon| u32::try_from(icon).ok()),
        presence,
    })
}

fn parse_friends(value: &Value) -> Option<Vec<Friend>> {
    let entries = value.as_array()?;
    let items: Vec<_> = entries.iter().filter_map(parse_friend).collect();
    // Une réponse non vide entièrement illisible n'est pas une vraie liste vide.
    (entries.is_empty() || !items.is_empty()).then_some(items)
}

fn social_status(value: &Value) -> FriendsStatus {
    match value.get("sessionState").and_then(Value::as_str) {
        Some("loaded") => FriendsStatus::Ready,
        Some("initializing" | "connected") => FriendsStatus::Loading,
        _ => FriendsStatus::Unavailable,
    }
}

fn empty(status: FriendsStatus) -> FriendsSnapshot {
    FriendsSnapshot {
        status,
        items: Vec::new(),
    }
}

/// Lecture locale seule, bornée par les délais HTTP existants. Le producteur doit annuler
/// la lecture sur déconnexion ou changement de compte et ne pas persister cette liste.
/// Les relectures réduisent les courses ; la LCU n'offre pas de snapshot atomique.
pub async fn read_friends(client: &LcuClient, expected: &LcuAccount) -> FriendsSnapshot {
    read_checked(client, expected)
        .await
        .unwrap_or_else(|| empty(FriendsStatus::Unavailable))
}

async fn read_checked(client: &LcuClient, expected: &LcuAccount) -> Option<FriendsSnapshot> {
    if read_account(client).await.as_ref() != Some(expected) {
        return None;
    }
    let session: Value = client.get_json(SESSION_ENDPOINT).await.ok()?;
    let status = social_status(&session);
    if status != FriendsStatus::Ready {
        return Some(empty(status));
    }
    let response: Value = client.get_json(FRIENDS_ENDPOINT).await.ok()?;
    let items = parse_friends(&response)?;
    if read_account(client).await.as_ref() != Some(expected) {
        return None;
    }
    let session: Value = client.get_json(SESSION_ENDPOINT).await.ok()?;
    let status = social_status(&session);
    Some(if status == FriendsStatus::Ready {
        FriendsSnapshot { status, items }
    } else {
        empty(status)
    })
}

#[cfg(test)]
mod tests;
