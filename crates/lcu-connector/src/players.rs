//! Profil et historique du compte actif, lus dans la LCU et projetés sans PUUID.
use crate::{LcuAccount, LcuClient};
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashSet, time::Duration};
use thiserror::Error;

// Schémas et paramètres vérifiés le 2 octobre 2026, puis GET sur le client local :
// https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json
const RANKS_ENDPOINT: &str = "/lol-ranked/v1/current-ranked-stats";
const HISTORY_ENDPOINT: &str = "/lol-match-history/v1/products/lol/current-summoner/matches";
const READ_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerSource {
    Lcu,
    Api,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileRank {
    pub queue_id: i32,
    pub status: String,
    pub tier: Option<String>,
    pub division: Option<String>,
    pub league_points: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerProfile {
    pub platform: String,
    pub game_name: String,
    pub tag_line: String,
    pub profile_icon_id: Option<u32>,
    pub summoner_level: Option<u64>,
    pub ranks: Vec<ProfileRank>,
    pub fetched_at: u64,
    pub source: PlayerSource,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerMatch {
    pub match_id: String,
    pub queue_id: i32,
    pub patch: Option<String>,
    pub game_start_ms: i64,
    pub duration_s: i64,
    pub champion_id: u32,
    pub win: bool,
    pub kills: Option<u32>,
    pub deaths: Option<u32>,
    pub assists: Option<u32>,
    pub items: Vec<u32>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerHistory {
    pub platform: String,
    pub game_name: String,
    pub tag_line: String,
    pub fetched_at: u64,
    pub start: u32,
    pub count: u32,
    pub next_start: Option<u32>,
    pub omitted_matches: u32,
    pub matches: Vec<PlayerMatch>,
    pub source: PlayerSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LocalPlayerError {
    #[error("données du compte local indisponibles")]
    Unavailable,
    #[error("réponse du compte local incohérente")]
    InvalidResponse,
    #[error("le compte actif a changé")]
    AccountChanged,
    #[error("pagination locale invalide")]
    InvalidRequest,
}

/// Lecture du compte actif uniquement ; le PUUID reste privé au connecteur.
pub async fn read_profile(
    client: &LcuClient,
    account: &LcuAccount,
) -> Result<PlayerProfile, LocalPlayerError> {
    tokio::time::timeout(READ_TIMEOUT, async {
        let before = read_identity(client).await?;
        before.validate_account(account)?;
        // L'absence d'un classement ne rend pas indisponibles l'icône et le niveau.
        let ranks = client.get_json::<Value>(RANKS_ENDPOINT).await.ok();
        before.validate_after(&read_identity(client).await?)?;
        Ok(PlayerProfile {
            platform: before.account.platform,
            game_name: before.account.game_name,
            tag_line: before.account.tag_line,
            profile_icon_id: optional_u32(&before.summoner, "profileIconId")?,
            summoner_level: before.summoner.get("summonerLevel").and_then(Value::as_u64),
            ranks: ranks.as_ref().map(parse_ranks).unwrap_or_default(),
            fetched_at: timestamp()?,
            source: PlayerSource::Lcu,
        })
    })
    .await
    .map_err(|_| LocalPlayerError::Unavailable)?
}

/// Retourne seulement la page réellement disponible dans le client, sans repli réseau public.
pub async fn read_matches(
    client: &LcuClient,
    account: &LcuAccount,
    start: u32,
    count: u32,
) -> Result<PlayerHistory, LocalPlayerError> {
    if start > 10_000 || !(1..=20).contains(&count) {
        return Err(LocalPlayerError::InvalidRequest);
    }
    tokio::time::timeout(READ_TIMEOUT, async {
        let before = read_identity(client).await?;
        before.validate_account(account)?;
        let path = format!(
            "{HISTORY_ENDPOINT}?begIndex={start}&endIndex={}",
            start + count - 1
        );
        let value = client
            .get_json::<Value>(&path)
            .await
            .map_err(|_| LocalPlayerError::Unavailable)?;
        before.validate_after(&read_identity(client).await?)?;
        parse_history(&value, &before, start, count)
    })
    .await
    .map_err(|_| LocalPlayerError::Unavailable)?
}

// Ne pas dériver Debug/Serialize : ces données servent uniquement au rapprochement local.
struct Identity {
    account: LcuAccount,
    puuid: String,
    summoner: Value,
}

impl Identity {
    fn validate_account(&self, expected: &LcuAccount) -> Result<(), LocalPlayerError> {
        if !self.account.same_identity(expected) {
            return Err(LocalPlayerError::AccountChanged);
        }
        Ok(())
    }

    fn validate_after(&self, after: &Self) -> Result<(), LocalPlayerError> {
        after.validate_account(&self.account)?;
        if self.puuid != after.puuid {
            return Err(LocalPlayerError::AccountChanged);
        }
        Ok(())
    }
}

async fn read_identity(client: &LcuClient) -> Result<Identity, LocalPlayerError> {
    let summoner: Value = client
        .get_json(crate::account::ACCOUNT_ENDPOINT)
        .await
        .map_err(|_| LocalPlayerError::Unavailable)?;
    let region: Value = client
        .get_json(crate::account::REGION_ENDPOINT)
        .await
        .map_err(|_| LocalPlayerError::Unavailable)?;
    let account = LcuAccount::parse(&summoner, &region).ok_or(LocalPlayerError::InvalidResponse)?;
    let puuid = summoner
        .get("puuid")
        .and_then(Value::as_str)
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 128
                && value.trim() == *value
                && !value.chars().any(char::is_control)
        })
        .ok_or(LocalPlayerError::InvalidResponse)?
        .to_owned();
    Ok(Identity {
        account,
        puuid,
        summoner,
    })
}

fn parse_ranks(value: &Value) -> Vec<ProfileRank> {
    [("RANKED_SOLO_5x5", 420), ("RANKED_FLEX_SR", 440)]
        .into_iter()
        .filter_map(|(queue, queue_id)| {
            let rank = value.get("queueMap")?.get(queue)?;
            if rank.get("queueType")?.as_str()? != queue {
                return None;
            }
            let tier = rank.get("tier")?.as_str()?;
            if tier == "NONE" {
                return Some(ProfileRank {
                    queue_id,
                    status: "unranked".into(),
                    tier: None,
                    division: None,
                    league_points: None,
                });
            }
            if !matches!(
                tier,
                "IRON"
                    | "BRONZE"
                    | "SILVER"
                    | "GOLD"
                    | "PLATINUM"
                    | "EMERALD"
                    | "DIAMOND"
                    | "MASTER"
                    | "GRANDMASTER"
                    | "CHALLENGER"
            ) {
                return None;
            }
            let division = rank.get("division")?.as_str()?;
            if !matches!(division, "I" | "II" | "III" | "IV") {
                return None;
            }
            Some(ProfileRank {
                queue_id,
                status: "ranked".into(),
                tier: Some(tier.into()),
                division: Some(division.into()),
                league_points: Some(i32::try_from(rank.get("leaguePoints")?.as_i64()?).ok()?),
            })
        })
        .collect()
}

fn parse_history(
    value: &Value,
    identity: &Identity,
    start: u32,
    count: u32,
) -> Result<PlayerHistory, LocalPlayerError> {
    let invalid = LocalPlayerError::InvalidResponse;
    if value.get("platformId").and_then(Value::as_str) != Some(identity.account.platform.as_str()) {
        return Err(invalid);
    }
    let page = value.get("games").ok_or(invalid)?;
    let games = page.get("games").and_then(Value::as_array).ok_or(invalid)?;
    let length = u32::try_from(games.len()).map_err(|_| invalid)?;
    // gameCount ne prouve ni la longueur de page ni l'exhaustivité d'une saison.
    let begin = page
        .get("gameIndexBegin")
        .and_then(Value::as_u64)
        .ok_or(invalid)?;
    let repeated = start > 0 && begin == 0;
    if length > count
        || (begin != u64::from(start) && !repeated)
        || page.get("gameIndexEnd").and_then(Value::as_u64)
            != begin.checked_add(u64::from(length.saturating_sub(1)))
    {
        return Err(invalid);
    }
    let mut matches = Vec::with_capacity(games.len());
    let mut omitted_matches = 0;
    let mut ids = HashSet::new();
    for game in games {
        // Identité et unicité restent fatales, même pour une partie illisible
        // ou la page initiale répétée par certains clients.
        let (game_id, participant) = match_identity(game, identity)?;
        if !ids.insert(game_id) {
            return Err(invalid);
        }
        if !repeated {
            match parse_match(game, identity, game_id, participant) {
                Ok(game) => matches.push(game),
                Err(_) => omitted_matches += 1,
            }
        }
    }
    Ok(PlayerHistory {
        platform: identity.account.platform.clone(),
        game_name: identity.account.game_name.clone(),
        tag_line: identity.account.tag_line.clone(),
        fetched_at: timestamp()?,
        start,
        count,
        next_start: (!repeated && length == count && start + count <= 10_000)
            .then_some(start + count),
        omitted_matches,
        matches,
        source: PlayerSource::Lcu,
    })
}

fn match_identity<'a>(
    game: &'a Value,
    identity: &Identity,
) -> Result<(u64, &'a Value), LocalPlayerError> {
    let invalid = LocalPlayerError::InvalidResponse;
    if game.get("platformId").and_then(Value::as_str) != Some(identity.account.platform.as_str()) {
        return Err(invalid);
    }
    let identities = game
        .get("participantIdentities")
        .and_then(Value::as_array)
        .ok_or(invalid)?;
    let mut local = identities.iter().filter(|row| {
        row.get("player")
            .and_then(|p| p.get("puuid"))
            .and_then(Value::as_str)
            == Some(identity.puuid.as_str())
    });
    let participant_id = local
        .next()
        .and_then(|row| row.get("participantId"))
        .and_then(Value::as_u64)
        .filter(|id| *id > 0)
        .ok_or(invalid)?;
    if local.next().is_some()
        || identities
            .iter()
            .filter(|row| row.get("participantId").and_then(Value::as_u64) == Some(participant_id))
            .count()
            != 1
    {
        return Err(invalid);
    }
    let participants = game
        .get("participants")
        .and_then(Value::as_array)
        .ok_or(invalid)?;
    let mut matching = participants
        .iter()
        .filter(|row| row.get("participantId").and_then(Value::as_u64) == Some(participant_id));
    let participant = matching.next().ok_or(invalid)?;
    if matching.next().is_some() {
        return Err(invalid);
    }
    let game_id = game
        .get("gameId")
        .and_then(Value::as_u64)
        .filter(|id| *id > 0)
        .ok_or(invalid)?;
    Ok((game_id, participant))
}

fn parse_match(
    game: &Value,
    identity: &Identity,
    game_id: u64,
    participant: &Value,
) -> Result<PlayerMatch, LocalPlayerError> {
    let invalid = LocalPlayerError::InvalidResponse;
    let stats = participant.get("stats").ok_or(invalid)?;
    let game_start_ms = game
        .get("gameCreation")
        .and_then(Value::as_i64)
        .filter(|date| (0..=8_640_000_000_000_000).contains(date))
        .ok_or(invalid)?;
    let duration_s = game
        .get("gameDuration")
        .and_then(Value::as_i64)
        .filter(|duration| (0..=i64::from(i32::MAX)).contains(duration))
        .ok_or(invalid)?;
    let queue_id = game
        .get("queueId")
        .and_then(Value::as_u64)
        .and_then(|queue| i32::try_from(queue).ok())
        .ok_or(invalid)?;
    let champion_id = optional_u32(participant, "championId")?
        .filter(|id| *id > 0)
        .ok_or(invalid)?;
    let items = (0..=6)
        .map(|slot| optional_u32(stats, &format!("item{slot}")))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .filter(|item| *item > 0)
        .collect();
    Ok(PlayerMatch {
        match_id: format!("{}_{game_id}", identity.account.platform),
        queue_id,
        patch: game
            .get("gameVersion")
            .and_then(Value::as_str)
            .and_then(parse_patch),
        game_start_ms,
        duration_s,
        champion_id,
        win: stats.get("win").and_then(Value::as_bool).ok_or(invalid)?,
        kills: optional_u32(stats, "kills")?,
        deaths: optional_u32(stats, "deaths")?,
        assists: optional_u32(stats, "assists")?,
        items,
        role: participant
            .get("timeline")
            .and_then(parse_role)
            .map(str::to_owned),
    })
}

fn optional_u32(value: &Value, key: &str) -> Result<Option<u32>, LocalPlayerError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(number) => number
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .map(Some)
            .ok_or(LocalPlayerError::InvalidResponse),
    }
}

fn parse_patch(version: &str) -> Option<String> {
    let mut parts = version.split('.');
    let major = parts.next()?;
    let minor = parts.next()?;
    let valid = |part: &str| {
        !part.is_empty() && part.len() <= 3 && part.bytes().all(|b| b.is_ascii_digit())
    };
    (valid(major) && valid(minor)).then(|| format!("{major}.{minor}"))
}

fn parse_role(timeline: &Value) -> Option<&'static str> {
    match (
        timeline.get("lane")?.as_str()?,
        timeline.get("role").and_then(Value::as_str),
    ) {
        ("TOP", _) => Some("TOP"),
        ("JUNGLE", _) => Some("JUNGLE"),
        ("MIDDLE", _) => Some("MIDDLE"),
        ("BOTTOM", Some("DUO_CARRY")) => Some("BOTTOM"),
        ("BOTTOM", Some("DUO_SUPPORT")) => Some("UTILITY"),
        _ => None,
    }
}

fn timestamp() -> Result<u64, LocalPlayerError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|time| time.as_secs())
        .map_err(|_| LocalPlayerError::Unavailable)
}

#[cfg(test)]
mod tests;
