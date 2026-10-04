//! Profils résolus par Riot ID actuel et historique public projeté par participant.
use crate::error::ApiError;
use olc_collector::rate_limit::{Governor, RateLimiter};
use olc_collector::riot_client::{Endpoint, Request, RiotClient, RiotError, Route, Transport};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{Mutex, Semaphore};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileRank {
    pub queue_id: i32,
    pub status: String,
    pub tier: Option<String>,
    pub division: Option<String>,
    pub league_points: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryQuery {
    #[serde(default)]
    pub start: u32,
    #[serde(default = "default_count")]
    pub count: u32,
}
fn default_count() -> u32 {
    10
}

pub struct Profiles<T: Transport> {
    client: RiotClient<T>,
    cache: Mutex<HashMap<(String, String, String), Profile>>,
    matches: Mutex<HashMap<(String, String), PlayerMatch>>,
    requests: Semaphore,
    riot_timeout: Duration,
}
impl<T: Transport> Profiles<T> {
    /// Le transport de production doit être coordonné avec le collecteur.
    pub fn new(transport: T) -> Self {
        let governor = Arc::new(Governor::new(RateLimiter::new(vec![
            (20, Duration::from_secs(1)),
            (100, Duration::from_secs(120)),
        ])));
        Self {
            client: RiotClient::new(transport, governor, Duration::from_secs(120)),
            cache: Mutex::new(HashMap::new()),
            matches: Mutex::new(HashMap::new()),
            requests: Semaphore::new(4),
            riot_timeout: Duration::from_secs(20),
        }
    }
    /// Durée maximale d'un appel Riot, attente de quota partagé comprise.
    pub fn with_riot_timeout(mut self, timeout: Duration) -> Self {
        self.riot_timeout = timeout;
        self
    }
    async fn get(&self, request: &Request) -> Result<Vec<u8>, ApiError> {
        let _permit = self
            .requests
            .try_acquire()
            .map_err(|_| ApiError::RateLimited)?;
        // Le transport HTTP est borné à 15 s, plus court que ce délai : dépasser celui-ci
        // signifie en pratique que la réservation de quota partagé a attendu (collecte en cours).
        let body = tokio::time::timeout(self.riot_timeout, self.client.get(request))
            .await
            .map_err(|_| ApiError::RiotBusy)?
            .map_err(|error| match error {
                RiotError::NotFound => ApiError::NotFound,
                RiotError::RateLimited { .. } => ApiError::RateLimited,
                _ => ApiError::Unavailable,
            })?;
        if body.len() > 8 * 1024 * 1024 {
            return Err(ApiError::Unavailable);
        }
        Ok(body)
    }
    /// Résout le Riot ID actuel ; aucune recherche dans les identités historiques.
    pub async fn profile(
        &self,
        platform: &str,
        name: &str,
        tag: &str,
    ) -> Result<Profile, ApiError> {
        let (local, _) = Route::for_platform(platform).map_err(|_| ApiError::InvalidRequest)?;
        if !valid_text(name, 64) || !valid_text(tag, 32) {
            return Err(ApiError::InvalidRequest);
        }
        let key = (platform.to_owned(), name.to_lowercase(), tag.to_lowercase());
        let now = jsonwebtoken::get_current_timestamp();
        if let Some(profile) = self
            .cache
            .lock()
            .await
            .get(&key)
            .filter(|p| now.saturating_sub(p.fetched_at) < 300)
        {
            return Ok(profile.clone());
        }
        // account-v1 : trois clusters globaux seulement, pas de SEA. Europe pour l'hébergement UE.
        // Source : https://developer.riotgames.com/api-details/account-v1
        // account-v1 GET_getByRiotId ; summoner-v4 GET_getByPUUID ; league-v4 GET_getLeagueEntriesByPUUID.
        let account: Value = serde_json::from_slice(
            &self
                .get(&request(
                    Endpoint::AccountByRiotId,
                    Route::Europe,
                    &["riot", "account", "v1", "accounts", "by-riot-id", name, tag],
                ))
                .await?,
        )
        .map_err(|_| ApiError::Unavailable)?;
        let puuid = required_text(&account, "puuid")?;
        let current_name = required_text(&account, "gameName")?;
        let current_tag = required_text(&account, "tagLine")?;
        if current_name.to_lowercase() != name.to_lowercase()
            || current_tag.to_lowercase() != tag.to_lowercase()
        {
            return Err(ApiError::NotFound);
        }
        let summoner: Value = serde_json::from_slice(
            &self
                .get(&request(
                    Endpoint::SummonerByPuuid,
                    local,
                    &["lol", "summoner", "v4", "summoners", "by-puuid", puuid],
                ))
                .await?,
        )
        .map_err(|_| ApiError::Unavailable)?;
        if summoner
            .get("puuid")
            .and_then(Value::as_str)
            .is_some_and(|p| p != puuid)
        {
            return Err(ApiError::Unavailable);
        }
        let ranks_request =
            Request::participant_ranks(platform, puuid).map_err(|_| ApiError::InvalidRequest)?;
        let observed =
            olc_collector::model::parse_participant_ranks(&self.get(&ranks_request).await?)
                .map_err(|_| ApiError::Unavailable)?;
        let ranks = [420, 440]
            .into_iter()
            .map(
                |queue_id| match observed.iter().find(|r| r.queue_id == queue_id) {
                    Some(rank) => ProfileRank {
                        queue_id,
                        status: "ranked".into(),
                        tier: Some(rank.tier.clone()),
                        division: Some(rank.division.clone()),
                        league_points: Some(rank.league_points),
                    },
                    None => ProfileRank {
                        queue_id,
                        status: "unranked".into(),
                        tier: None,
                        division: None,
                        league_points: None,
                    },
                },
            )
            .collect();
        let profile = Profile {
            platform: platform.into(),
            game_name: current_name.into(),
            tag_line: current_tag.into(),
            puuid: puuid.into(),
            profile_icon_id: number(&summoner, "profileIconId"),
            summoner_level: summoner.get("summonerLevel").and_then(Value::as_u64),
            ranks,
            fetched_at: jsonwebtoken::get_current_timestamp(),
        };
        let mut cache = self.cache.lock().await;
        cache.retain(|_, p| now.saturating_sub(p.fetched_at) < 300);
        if cache.len() >= 1024 {
            cache.clear();
        }
        cache.insert(key, profile.clone());
        Ok(profile)
    }
    /// Lit une page Riot courante ; le curseur progresse aussi sur les parties exclues.
    pub async fn history(
        &self,
        platform: &str,
        name: &str,
        tag: &str,
        query: HistoryQuery,
    ) -> Result<ProfileMatches, ApiError> {
        if query.start > 10_000 || !(1..=20).contains(&query.count) {
            return Err(ApiError::InvalidRequest);
        }
        let profile = self.profile(platform, name, tag).await?;
        let regional = Route::for_platform(platform)
            .map_err(|_| ApiError::InvalidRequest)?
            .1;
        // Source : https://developer.riotgames.com/apis#match-v5/GET_getMatchIdsByPUUID
        let mut ids_request = request(
            Endpoint::MatchIdsByPuuid,
            regional,
            &[
                "lol",
                "match",
                "v5",
                "matches",
                "by-puuid",
                &profile.puuid,
                "ids",
            ],
        );
        ids_request.query = vec![
            ("start", query.start.to_string()),
            ("count", query.count.to_string()),
        ];
        let ids: Vec<String> = serde_json::from_slice(&self.get(&ids_request).await?)
            .map_err(|_| ApiError::Unavailable)?;
        if ids.len() > query.count as usize {
            return Err(ApiError::Unavailable);
        }
        let next_start = (ids.len() == query.count as usize && query.start + query.count <= 10_000)
            .then_some(query.start + query.count);
        let mut result = ProfileMatches {
            platform: platform.into(),
            game_name: profile.game_name,
            tag_line: profile.tag_line,
            fetched_at: jsonwebtoken::get_current_timestamp(),
            start: query.start,
            count: query.count,
            next_start,
            omitted_matches: 0,
            matches: vec![],
        };
        for id in ids {
            // Après un transfert, match-v5 peut encore retourner une autre plateforme.
            if !id.starts_with(&format!("{platform}_")) {
                result.omitted_matches += 1;
                continue;
            }
            if id.len() > 100 || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return Err(ApiError::Unavailable);
            }
            let cache_key = (id.clone(), profile.puuid.clone());
            if let Some(game) = self.matches.lock().await.get(&cache_key).cloned() {
                result.matches.push(game);
                continue;
            }
            let req =
                Request::match_detail_for(platform, &id).map_err(|_| ApiError::InvalidRequest)?;
            let raw: Value = serde_json::from_slice(&self.get(&req).await?)
                .map_err(|_| ApiError::Unavailable)?;
            if let Some(game) = project_match(&raw, &id, platform, &profile.puuid)? {
                let mut cache = self.matches.lock().await;
                if cache.len() >= 4096 {
                    cache.clear();
                }
                cache.insert(cache_key, game.clone());
                result.matches.push(game);
            } else {
                result.omitted_matches += 1;
            }
        }
        result.fetched_at = jsonwebtoken::get_current_timestamp();
        Ok(result)
    }
}
fn request(endpoint: Endpoint, route: Route, segments: &[&str]) -> Request {
    Request {
        endpoint,
        route,
        segments: segments.iter().map(|s| (*s).into()).collect(),
        query: vec![],
    }
}
fn valid_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}
fn required_text<'a>(value: &'a Value, field: &str) -> Result<&'a str, ApiError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|s| valid_text(s, 128))
        .ok_or(ApiError::Unavailable)
}
fn number(value: &Value, field: &str) -> Option<u32> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|n| n.try_into().ok())
}

fn project_match(
    value: &Value,
    id: &str,
    platform: &str,
    puuid: &str,
) -> Result<Option<PlayerMatch>, ApiError> {
    let info = &value["info"];
    if value["metadata"]["matchId"].as_str() != Some(id)
        || info["platformId"].as_str() != Some(platform)
    {
        return Err(ApiError::Unavailable);
    }
    let queue = info["queueId"]
        .as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .filter(|n| *n >= 0)
        .ok_or(ApiError::Unavailable)?;
    // Politique Riot : aucune partie personnalisée publique sans consentement RSO.
    // https://developer.riotgames.com/docs/lol#game-policy
    if queue == 0 || info["gameType"] == "CUSTOM_GAME" {
        return Ok(None);
    }
    let participants = info["participants"]
        .as_array()
        .ok_or(ApiError::Unavailable)?;
    let selected: Vec<_> = participants
        .iter()
        .filter(|p| p["puuid"].as_str() == Some(puuid))
        .collect();
    if selected.len() != 1 {
        return Err(ApiError::Unavailable);
    }
    let player = selected[0];
    let patch = olc_collector::model::patch_from_version(required_text(info, "gameVersion")?)
        .ok_or(ApiError::Unavailable)?;
    let start = info["gameStartTimestamp"]
        .as_i64()
        .or_else(|| info["gameCreation"].as_i64())
        .filter(|n| *n >= 0)
        .ok_or(ApiError::Unavailable)?;
    let mut duration = info["gameDuration"]
        .as_i64()
        .filter(|n| *n >= 0)
        .ok_or(ApiError::Unavailable)?;
    let version: Vec<u32> = patch.split('.').filter_map(|s| s.parse().ok()).collect();
    if version.len() == 2 && (version[0], version[1]) < (11, 20) {
        duration /= 1000;
    }
    Ok(Some(PlayerMatch {
        match_id: id.into(),
        queue_id: queue,
        patch,
        game_start_ms: start,
        duration_s: duration,
        champion_id: number(player, "championId")
            .filter(|n| *n > 0)
            .ok_or(ApiError::Unavailable)?,
        win: player["win"].as_bool().ok_or(ApiError::Unavailable)?,
        kills: number(player, "kills"),
        deaths: number(player, "deaths"),
        assists: number(player, "assists"),
        items: (0..=6)
            .filter_map(|i| number(player, &format!("item{i}")).filter(|n| *n > 0))
            .collect(),
        role: player["teamPosition"]
            .as_str()
            .filter(|s| ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"].contains(s))
            .map(str::to_owned),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use olc_collector::riot_client::{RawResponse, Request, TransportError};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Fake {
        requests: Arc<Mutex<Vec<Request>>>,
    }
    impl Transport for Fake {
        async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
            self.requests.lock().unwrap().push(request.clone());
            let path = request.segments.join("/");
            let body = if path.contains("accounts/by-riot-id") {
                json!({"puuid":"synthetic-player","gameName":"Current Name","tagLine":"TEST"})
            } else if path.contains("summoners/by-puuid") {
                json!({"puuid":"synthetic-player","profileIconId":1,"summonerLevel":42})
            } else if path.contains("entries/by-puuid") {
                json!([{ "queueType":"RANKED_SOLO_5x5", "tier":"GOLD", "rank":"II", "leaguePoints":55 }])
            } else if path.ends_with("/ids") {
                json!(["EUW1_1", "EUW1_2"])
            } else {
                let mut v = game();
                if path.ends_with("EUW1_2") {
                    v["metadata"]["matchId"] = json!("EUW1_2");
                    v["info"]["queueId"] = json!(0);
                }
                v
            };
            Ok(RawResponse {
                status: 200,
                headers: vec![],
                body: serde_json::to_vec(&body).unwrap(),
            })
        }
    }
    fn game() -> Value {
        json!({"metadata":{"matchId":"EUW1_1"},"info":{"platformId":"EUW1","queueId":420,"gameVersion":"16.19.1","gameStartTimestamp":1000,"gameDuration":1200,"gameType":"MATCHED_GAME","participants":[{"puuid":"synthetic-player","riotIdGameName":"Historic Name","championId":1,"win":true,"kills":2,"deaths":3,"assists":4,"item0":1001,"teamPosition":"MIDDLE"},{"puuid":"synthetic-opponent","riotIdGameName":"Hidden Name","championId":2,"win":false}]}})
    }
    #[tokio::test]
    async fn les_profils_sea_separent_le_routage_des_comptes_et_des_matchs() {
        let fake = Fake::default();
        let service = Profiles::new(fake.clone());
        service
            .history(
                "OC1",
                "Current Name",
                "TEST",
                HistoryQuery { start: 0, count: 2 },
            )
            .await
            .unwrap();
        let requests = fake.requests.lock().unwrap();
        assert_eq!(requests[0].route.host(), "europe.api.riotgames.com");
        assert_eq!(requests[1].route.host(), "oc1.api.riotgames.com");
        assert_eq!(requests[3].route.host(), "sea.api.riotgames.com");
    }

    #[tokio::test]
    async fn recherche_actuelle_routage_et_cache_ne_recourent_pas_aux_anciens_pseudos() {
        let fake = Fake::default();
        let service = Profiles::new(fake.clone());
        let p = service
            .profile("EUW1", "Current Name", "TEST")
            .await
            .unwrap();
        assert_eq!(p.game_name, "Current Name");
        assert_eq!(p.ranks[0].tier.as_deref(), Some("GOLD"));
        assert_eq!(p.ranks[1].status, "unranked");
        let requests = fake.requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].route.host(), "europe.api.riotgames.com");
        assert_eq!(requests[0].segments.last().unwrap(), "TEST");
        service
            .profile("EUW1", "Current Name", "TEST")
            .await
            .unwrap();
        assert_eq!(fake.requests.lock().unwrap().len(), 3);
        assert!(matches!(
            service.profile("EUW1", "Historic Name", "TEST").await,
            Err(ApiError::NotFound)
        ));
        assert!(matches!(
            service.profile("INVALID", "Current Name", "TEST").await,
            Err(ApiError::InvalidRequest)
        ));
    }
    #[test]
    fn projette_uniquement_le_joueur_et_refuse_parties_privees_ou_incoherentes() {
        let raw = game();
        let game = project_match(&raw, "EUW1_1", "EUW1", "synthetic-player")
            .unwrap()
            .unwrap();
        assert_eq!(game.kills, Some(2));
        let text = serde_json::to_string(&game).unwrap();
        assert!(!text.contains("Name") && !text.contains("puuid"));
        let mut custom = raw.clone();
        custom["info"]["queueId"] = json!(0);
        assert!(project_match(&custom, "EUW1_1", "EUW1", "synthetic-player")
            .unwrap()
            .is_none());
        assert!(project_match(&raw, "KR_1", "KR", "synthetic-player").is_err());
        assert!(project_match(&raw, "EUW1_1", "EUW1", "absent").is_err());
    }
    #[tokio::test]
    async fn pagination_avance_meme_si_une_partie_est_non_publiable() {
        let fake = Fake::default();
        let service = Profiles::new(fake.clone());
        let result = service
            .history(
                "EUW1",
                "Current Name",
                "TEST",
                HistoryQuery {
                    start: 10,
                    count: 2,
                },
            )
            .await
            .unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.omitted_matches, 1);
        assert_eq!(result.next_start, Some(12));
        let req = fake
            .requests
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.segments.last().is_some_and(|p| p == "ids"))
            .unwrap()
            .clone();
        assert_eq!(
            req.query,
            vec![("start", "10".into()), ("count", "2".into())]
        );
        assert!(matches!(
            service
                .history(
                    "EUW1",
                    "Current Name",
                    "TEST",
                    HistoryQuery {
                        start: 0,
                        count: 21
                    }
                )
                .await,
            Err(ApiError::InvalidRequest)
        ));
    }

    /// Transport qui n'aboutit jamais : simule une attente de quota prolongée.
    struct Stuck;
    impl Transport for Stuck {
        async fn send(&self, _: &Request) -> Result<RawResponse, TransportError> {
            std::future::pending().await
        }
    }
    #[tokio::test]
    async fn une_attente_de_quota_trop_longue_est_distincte_d_une_panne() {
        let service = Profiles::new(Stuck).with_riot_timeout(Duration::from_millis(30));
        assert_eq!(
            service
                .profile("EUW1", "Current Name", "TEST")
                .await
                .unwrap_err(),
            ApiError::RiotBusy
        );
    }
    #[tokio::test]
    async fn une_panne_du_transport_reste_unavailable() {
        struct Broken;
        impl Transport for Broken {
            async fn send(&self, _: &Request) -> Result<RawResponse, TransportError> {
                Err(TransportError::Timeout)
            }
        }
        let service = Profiles::new(Broken);
        assert_eq!(
            service
                .profile("EUW1", "Current Name", "TEST")
                .await
                .unwrap_err(),
            ApiError::Unavailable
        );
    }
}
