//! Profils publics via les routes existantes de l'API #19, sans accès direct à Riot.
use crate::{BuildClient, BuildError};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashSet, time::Duration};

const MAX_PROFILE_BYTES: usize = 512 * 1024;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Identité recherchée, miroir `PlayerRequest` de @olc/shared.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerRequest {
    pub platform: String,
    pub game_name: String,
    pub tag_line: String,
}
impl PlayerRequest {
    /// Refuse les identités incomplètes et les segments normalisés par URL.
    pub fn validate(&self) -> Result<(), PlayerError> {
        if ![
            "BR1", "EUN1", "EUW1", "JP1", "KR", "LA1", "LA2", "ME1", "NA1", "OC1", "RU", "SG2",
            "TR1", "TW2", "VN2",
        ]
        .contains(&self.platform.as_str())
            || !valid_name(&self.game_name, 64)
            || !valid_name(&self.tag_line, 32)
        {
            return Err(PlayerError::InvalidRequest);
        }
        Ok(())
    }
    fn matches(&self, platform: &str, name: &str, tag: &str) -> bool {
        self.platform == platform
            && self.game_name.to_lowercase() == name.to_lowercase()
            && self.tag_line.to_lowercase() == tag.to_lowercase()
    }
}
fn valid_name(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.trim() == value
        && ![".", ".."].contains(&value)
        && !value.chars().any(|c| c.is_control() || c == '#')
}
/// Pagination explicite, miroir `PlayerMatchesRequest`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerMatchesRequest {
    pub player: PlayerRequest,
    pub start: u32,
    pub count: u32,
}

/// Codes traduits par React ; aucun détail de transport ou d'autorisation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerError {
    NotConfigured,
    InvalidConfiguration,
    InvalidRequest,
    Unauthorized,
    NotFound,
    Unavailable,
    RateLimited,
    InvalidResponse,
}
impl From<BuildError> for PlayerError {
    fn from(error: BuildError) -> Self {
        match error {
            BuildError::NotConfigured => Self::NotConfigured,
            BuildError::InvalidConfiguration => Self::InvalidConfiguration,
            _ => Self::Unavailable,
        }
    }
}
/// Miroir exact du contrat `ProfileRank` de l'API et de @olc/shared.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileRank {
    pub queue_id: i32,
    pub status: String,
    pub tier: Option<String>,
    pub division: Option<String>,
    pub league_points: Option<i32>,
}
/// Miroir `Profile` ; le PUUID reste en mémoire et n'est jamais journalisé.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub platform: String,
    pub game_name: String,
    pub tag_line: String,
    pub puuid: String,
    pub profile_icon_id: Option<u32>,
    pub summoner_level: Option<u64>,
    pub ranks: Vec<ProfileRank>,
    pub fetched_at: u64,
}
/// Miroir `PlayerMatch` : seule la participation du joueur consulté est exposée.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerMatch {
    pub match_id: String,
    pub queue_id: i32,
    pub patch: String,
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
/// Miroir `ProfileMatches` ; une page vide peut avoir une suite.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileMatches {
    pub platform: String,
    pub game_name: String,
    pub tag_line: String,
    pub fetched_at: u64,
    pub start: u32,
    pub count: u32,
    pub next_start: Option<u32>,
    pub omitted_matches: u32,
    pub matches: Vec<PlayerMatch>,
}

impl BuildClient {
    /// Profil et rangs officiels, route `GET /v1/profiles/{platform}/{name}/{tag}`.
    pub async fn player_profile(&self, request: PlayerRequest) -> Result<Profile, PlayerError> {
        request.validate()?;
        let result: Profile = self.read_player(&request, None).await?;
        let mut queues = HashSet::new();
        if !request.matches(&result.platform, &result.game_name, &result.tag_line)
            || result.puuid.is_empty()
            || result.puuid.len() > 128
            || result.puuid.chars().any(char::is_control)
            || result.fetched_at > MAX_SAFE_INTEGER / 1000
            || result.summoner_level.is_some_and(|n| n > MAX_SAFE_INTEGER)
            || result.ranks.len() > 2
            || result.ranks.iter().any(|r| {
                ![420, 440].contains(&r.queue_id)
                    || !queues.insert(r.queue_id)
                    || match r.status.as_str() {
                        "unranked" => {
                            r.tier.is_some() || r.division.is_some() || r.league_points.is_some()
                        }
                        "ranked" => {
                            !r.tier.as_deref().is_some_and(|v| {
                                [
                                    "IRON",
                                    "BRONZE",
                                    "SILVER",
                                    "GOLD",
                                    "PLATINUM",
                                    "EMERALD",
                                    "DIAMOND",
                                    "MASTER",
                                    "GRANDMASTER",
                                    "CHALLENGER",
                                ]
                                .contains(&v)
                            }) || !r
                                .division
                                .as_deref()
                                .is_some_and(|v| ["I", "II", "III", "IV"].contains(&v))
                                || r.league_points.is_none()
                        }
                        _ => true,
                    }
            })
        {
            return Err(PlayerError::InvalidResponse);
        }
        Ok(result)
    }
    /// Historique public ; l'API exclut les parties privées sans consentement RSO.
    pub async fn player_matches(
        &self,
        request: PlayerMatchesRequest,
    ) -> Result<ProfileMatches, PlayerError> {
        request.player.validate()?;
        if request.start > 10_000 || !(1..=20).contains(&request.count) {
            return Err(PlayerError::InvalidRequest);
        }
        let result: ProfileMatches = self
            .read_player(&request.player, Some((request.start, request.count)))
            .await?;
        let mut seen = HashSet::new();
        if !request
            .player
            .matches(&result.platform, &result.game_name, &result.tag_line)
            || result.fetched_at > MAX_SAFE_INTEGER / 1000
            || result.start != request.start
            || result.count != request.count
            || result.matches.len() + result.omitted_matches as usize > request.count as usize
            || result.next_start.is_some_and(|next| {
                next != request.start + request.count
                    || next > 10_000
                    || result.matches.len() + result.omitted_matches as usize
                        != request.count as usize
            })
            || result.matches.iter().any(|m| {
                !m.match_id
                    .starts_with(&format!("{}_", request.player.platform))
                    || m.match_id.len() > 100
                    || !m
                        .match_id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    || !seen.insert(&m.match_id)
                    || m.queue_id <= 0
                    || m.champion_id == 0
                    || m.patch.is_empty()
                    || m.patch.len() > 16
                    || !m.patch.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                    || m.game_start_ms < 0
                    || m.game_start_ms as u64 > 8_640_000_000_000_000
                    || m.duration_s < 0
                    || m.duration_s as u64 > MAX_SAFE_INTEGER
                    || m.items.len() > 7
                    || m.items.contains(&0)
                    || m.role.as_deref().is_some_and(|r| {
                        !["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"].contains(&r)
                    })
            })
        {
            return Err(PlayerError::InvalidResponse);
        }
        Ok(result)
    }
    async fn read_player<T: DeserializeOwned>(
        &self,
        request: &PlayerRequest,
        page: Option<(u32, u32)>,
    ) -> Result<T, PlayerError> {
        // L'historique assemble plusieurs requêtes Riot : délai borné, sans retry ni polling du front.
        tokio::time::timeout(Duration::from_secs(65), async {
            let _permit = self
                .slots
                .acquire()
                .await
                .map_err(|_| PlayerError::Unavailable)?;
            let mut url = self.base.clone();
            url.path_segments_mut()
                .map_err(|_| PlayerError::InvalidConfiguration)?
                .extend([
                    "v1",
                    "profiles",
                    &request.platform,
                    &request.game_name,
                    &request.tag_line,
                ]);
            if let Some((start, count)) = page {
                url.path_segments_mut()
                    .map_err(|_| PlayerError::InvalidConfiguration)?
                    .push("matches");
                url.query_pairs_mut()
                    .extend_pairs([("start", start.to_string()), ("count", count.to_string())]);
            }
            let mut response = self
                .http
                .get(url)
                .timeout(Duration::from_secs(60))
                .send()
                .await
                .map_err(|_| PlayerError::Unavailable)?;
            match response.status().as_u16() {
                200 => {}
                400 => return Err(PlayerError::InvalidRequest),
                401 | 403 => return Err(PlayerError::Unauthorized),
                404 => return Err(PlayerError::NotFound),
                429 => return Err(PlayerError::RateLimited),
                _ => return Err(PlayerError::Unavailable),
            }
            if response
                .content_length()
                .is_some_and(|n| n > MAX_PROFILE_BYTES as u64)
            {
                return Err(PlayerError::InvalidResponse);
            }
            let mut bytes = vec![];
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| PlayerError::Unavailable)?
            {
                if bytes.len() + chunk.len() > MAX_PROFILE_BYTES {
                    return Err(PlayerError::InvalidResponse);
                }
                bytes.extend_from_slice(&chunk);
            }
            serde_json::from_slice(&bytes).map_err(|_| PlayerError::InvalidResponse)
        })
        .await
        .map_err(|_| PlayerError::Unavailable)?
    }
}
