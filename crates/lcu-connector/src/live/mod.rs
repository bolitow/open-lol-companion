//! Lecture locale uniquement. Schéma et endpoint vérifiés dans la documentation Riot :
//! https://developer.riotgames.com/docs/lol#game-client-api_live-client-data-api
//! Aucun identifiant de joueur, champ adverse ou événement avec acteur ne sort de Rust.
use crate::{GameflowPhase, LcuSession};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
const POLL_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LivePlayer {
    pub champion_key: String,
    pub level: u32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub creep_score: u32,
    pub items: Vec<u32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct LiveEvent {
    pub id: u32,
    pub name: String,
    pub time: f64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveGame {
    pub game_time: f64,
    pub game_mode: String,
    pub map_number: u32,
    pub player: LivePlayer,
    pub events: Vec<LiveEvent>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveContext {
    pub champion_id: u32,
    pub role: Option<String>,
    pub platform: Option<String>,
    pub queue: Option<u32>,
    pub custom_game: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LiveStatus {
    #[default]
    Idle,
    Waiting,
    Ready,
    Unavailable,
    Invalid,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct LiveSession {
    pub revision: u32,
    pub generation: u32,
    pub status: LiveStatus,
    pub context: Option<LiveContext>,
    pub game: Option<LiveGame>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveError {
    Unavailable,
    Invalid,
}
fn number(value: &Value) -> Result<u32, LiveError> {
    value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or(LiveError::Invalid)
}
fn time(value: &Value) -> Result<f64, LiveError> {
    value
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or(LiveError::Invalid)
}
pub fn valid_role(role: &str) -> bool {
    matches!(role, "TOP" | "JUNGLE" | "MIDDLE" | "BOTTOM" | "UTILITY")
}
/// L'identité sert seulement à retrouver sans ambiguïté le joueur local.
fn project(value: Value) -> Result<LiveGame, LiveError> {
    let active = &value["activePlayer"];
    let (field, identity) = if let Some(id) = active["riotId"].as_str().filter(|v| !v.is_empty()) {
        ("riotId", id)
    } else {
        (
            "summonerName",
            active["summonerName"]
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or(LiveError::Invalid)?,
        )
    };
    let players = value["allPlayers"].as_array().ok_or(LiveError::Invalid)?;
    let mut own = players
        .iter()
        .filter(|p| p[field].as_str() == Some(identity));
    let own_player = own.next().ok_or(LiveError::Invalid)?;
    if own.next().is_some() {
        return Err(LiveError::Invalid);
    }
    let champion = own_player["rawChampionName"]
        .as_str()
        .and_then(|s| s.strip_prefix("game_character_displayname_"))
        .filter(|s| !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .ok_or(LiveError::Invalid)?;
    let scores = &own_player["scores"];
    let items = own_player["items"]
        .as_array()
        .filter(|items| items.len() <= 16)
        .ok_or(LiveError::Invalid)?
        .iter()
        .map(|item| number(&item["itemID"]))
        .collect::<Result<Vec<_>, _>>()?;
    let data = &value["gameData"];
    let mut events = Vec::new();
    for event in value["events"]["Events"]
        .as_array()
        .ok_or(LiveError::Invalid)?
    {
        let name = event["EventName"].as_str().unwrap_or_default();
        // Aucun kill, timer adverse, nom ou position n'est relayé.
        if matches!(name, "GameStart" | "MinionsSpawning" | "GameEnd") {
            events.push(LiveEvent {
                id: number(&event["EventID"])?,
                name: name.into(),
                time: time(&event["EventTime"])?,
            });
        }
    }
    events.sort_by_key(|e| e.id);
    events.dedup_by_key(|e| e.id);
    if events.len() > 32 {
        events.drain(..events.len() - 32);
    }
    Ok(LiveGame {
        game_time: time(&data["gameTime"])?,
        game_mode: data["gameMode"]
            .as_str()
            .filter(|s| s.len() <= 32)
            .ok_or(LiveError::Invalid)?
            .into(),
        map_number: number(&data["mapNumber"])?,
        player: LivePlayer {
            champion_key: champion.into(),
            level: number(&own_player["level"])?,
            kills: number(&scores["kills"])?,
            deaths: number(&scores["deaths"])?,
            assists: number(&scores["assists"])?,
            creep_score: number(&scores["creepScore"])?,
            items,
        },
        events,
    })
}

pub struct LiveClient {
    http: reqwest::Client,
}
impl LiveClient {
    pub fn new() -> Result<Self, LiveError> {
        let http = reqwest::Client::builder()
            .use_preconfigured_tls(crate::client::tls_config().map_err(|_| LiveError::Unavailable)?)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|_| LiveError::Unavailable)?;
        Ok(Self { http })
    }
    pub async fn read(&self) -> Result<LiveGame, LiveError> {
        self.read_url(&format!("{}/allgamedata", crate::LIVE_CLIENT_DATA_URL))
            .await
    }
    // URL privée : aucune commande ne peut changer la destination ou ajouter une authentification.
    async fn read_url(&self, url: &str) -> Result<LiveGame, LiveError> {
        let mut response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|_| LiveError::Unavailable)?;
        if !response.status().is_success() {
            return Err(LiveError::Unavailable);
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_BODY_BYTES as u64)
        {
            return Err(LiveError::Invalid);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| LiveError::Unavailable)? {
            if bytes.len() + chunk.len() > MAX_BODY_BYTES {
                return Err(LiveError::Invalid);
            }
            bytes.extend_from_slice(&chunk);
        }
        project(serde_json::from_slice(&bytes).map_err(|_| LiveError::Invalid)?)
    }
}
#[derive(Default)]
struct LiveTracker {
    snapshot: LiveSession,
    active: bool,
    pending: Option<LiveContext>,
    pending_draft: Option<String>,
    last_time: Option<f64>,
}
impl LiveTracker {
    fn observe(&mut self, session: &LcuSession) {
        let active = session.connected && session.phase.is_some_and(GameflowPhase::is_in_game);
        if session.connected && session.phase == Some(GameflowPhase::ChampSelect) {
            if self.pending_draft != session.draft_id {
                self.pending = None;
                self.pending_draft = session.draft_id.clone();
            }
            if session.draft.is_some() {
                self.pending = session
                    .draft
                    .as_ref()
                    .filter(|d| d.supported)
                    .and_then(|d| {
                        let mut locals = d.allies.iter().filter(|p| p.local);
                        let p = locals.next()?;
                        if locals.next().is_some() || !p.locked {
                            return None;
                        }
                        Some(LiveContext {
                            champion_id: p.champion_id?,
                            role: p
                                .position
                                .as_ref()
                                .map(|s| s.to_uppercase())
                                .filter(|s| valid_role(s)),
                            queue: d.queue_id,
                            platform: session.account.as_ref().map(|a| a.platform.clone()),
                            custom_game: d.custom_game,
                        })
                    });
            }
        } else if !active {
            self.pending = None;
            self.pending_draft = None;
        }
        if active != self.active {
            self.active = active;
            self.last_time = None;
            self.snapshot.generation = self.snapshot.generation.saturating_add(1);
            self.snapshot.game = None;
            self.snapshot.context = if active { self.pending.take() } else { None };
            self.snapshot.status = if active {
                LiveStatus::Waiting
            } else {
                LiveStatus::Idle
            };
        }
        self.snapshot.revision = self.snapshot.revision.saturating_add(1);
    }
    fn custom_role(&mut self, role: Option<&str>) {
        if let Some(context) = self
            .pending
            .as_mut()
            .filter(|c| c.custom_game && c.role.is_none())
        {
            context.role = role.filter(|r| valid_role(r)).map(str::to_owned);
        }
    }
    fn accept(&mut self, result: Result<LiveGame, LiveError>) {
        if !self.active {
            return;
        }
        self.snapshot.revision = self.snapshot.revision.saturating_add(1);
        match result {
            Ok(game) => {
                if self.last_time.is_some_and(|t| game.game_time + 1.0 < t) {
                    self.snapshot.generation = self.snapshot.generation.saturating_add(1);
                    self.snapshot.context = None;
                }
                self.last_time = Some(game.game_time);
                self.snapshot.status = LiveStatus::Ready;
                self.snapshot.game = Some(game);
            }
            Err(error) => {
                self.snapshot.game = None;
                self.snapshot.status = match error {
                    LiveError::Invalid => LiveStatus::Invalid,
                    LiveError::Unavailable => LiveStatus::Unavailable,
                };
            }
        }
    }
}
#[derive(Clone, Default)]
pub struct LiveInput {
    session: LcuSession,
    pub custom_role: Option<String>,
    last_draft: Option<LcuSession>,
    game_epoch: u32,
}
impl LiveInput {
    /// Repli synchrone du producteur : watch peut regrouper plusieurs événements.
    /// Le dernier champion et la sortie/entrée de partie ne doivent pas se perdre.
    pub fn set_session(&mut self, session: &LcuSession) {
        let active = |s: &LcuSession| s.connected && s.phase.is_some_and(GameflowPhase::is_in_game);
        if active(&self.session) != active(session) {
            self.game_epoch = self.game_epoch.saturating_add(1);
        }
        if session.connected && session.phase == Some(GameflowPhase::ChampSelect) {
            if self
                .last_draft
                .as_ref()
                .is_some_and(|s| s.draft_id != session.draft_id)
            {
                self.last_draft = None;
            }
            if session.draft.is_some() {
                self.last_draft = Some(session.clone());
            }
        } else if !active(session) {
            self.last_draft = None;
        }
        self.session = session.clone();
    }
}

/// Une seule requête à la fois. Tout changement LCU annule la lecture en attente ;
/// aucune réponse ancienne ne traverse un changement de partie. Pas de rattrapage de ticks.
pub async fn watch(input: watch::Receiver<LiveInput>, output: mpsc::Sender<LiveSession>) {
    let client = LiveClient::new();
    watch_with_reader(input, output, || async {
        match &client {
            Ok(client) => client.read().await,
            Err(error) => Err(*error),
        }
    })
    .await;
}

async fn watch_with_reader<R, F>(
    mut input: watch::Receiver<LiveInput>,
    output: mpsc::Sender<LiveSession>,
    read: R,
) where
    R: Fn() -> F,
    F: std::future::Future<Output = Result<LiveGame, LiveError>>,
{
    let mut tracker = LiveTracker::default();
    let mut deadline = tokio::time::Instant::now();
    let mut game_epoch = 0;
    loop {
        let current = input.borrow_and_update().clone();
        if game_epoch != current.game_epoch {
            tracker.observe(&LcuSession::default());
            game_epoch = current.game_epoch;
        }
        if !tracker.active {
            if let Some(draft) = &current.last_draft {
                tracker.observe(draft);
                tracker.custom_role(current.custom_role.as_deref());
            }
        }
        tracker.observe(&current.session);
        tracker.custom_role(current.custom_role.as_deref());
        if output.send(tracker.snapshot.clone()).await.is_err() {
            return;
        }
        loop {
            tokio::select! { biased;
                changed = input.changed() => {if changed.is_err() {return;} break;}
                _ = tokio::time::sleep_until(deadline), if tracker.active => {
                    let result = tokio::select! { biased;
                        changed = input.changed() => {if changed.is_err() {return;} break;}
                        result = read() => result
                    };
                    tracker.accept(result);
                    if output.send(tracker.snapshot.clone()).await.is_err() {return;}
                    deadline = tokio::time::Instant::now() + POLL_DELAY;
                }
            }
        }
    }
}
#[cfg(test)]
mod tests;
