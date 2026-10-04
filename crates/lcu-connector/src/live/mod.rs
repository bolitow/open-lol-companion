//! Lecture locale uniquement. Schéma et endpoint vérifiés dans la documentation Riot :
//! https://developer.riotgames.com/docs/lol#game-client-api_live-client-data-api
//! Liste blanche explicite (#102) : seuls les champs ci-dessous sortent de Rust. Aucun
//! identifiant de joueur, aucune donnée adverse cachée (or, sorts, positions) et aucun
//! événement avec acteur nommé ne sont projetés ; les noms servent uniquement à décider
//! d'un booléen ou d'un camp, puis sont jetés.
use crate::{GameflowPhase, LcuSession};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
const POLL_DELAY: Duration = Duration::from_secs(1);
/// Journal public borné : une longue partie compte quelques dizaines de kills et d'objectifs.
const MAX_EVENTS: usize = 256;

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
    // Champs ajoutés (#102) : optionnels, une valeur absente ou invalide reste `None`.
    pub current_gold: Option<f64>,
    pub ward_score: Option<f64>,
    pub is_dead: Option<bool>,
    pub respawn_timer: Option<f64>,
    pub ability_levels: Option<LiveAbilityLevels>,
    pub team: Option<LiveSide>,
    pub position: Option<String>,
}
/// Niveaux appris des quatre compétences du joueur local (hors passif).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LiveAbilityLevels {
    pub q: u32,
    pub w: u32,
    pub e: u32,
    pub r: u32,
}
/// Côté de la carte du joueur local (`ORDER` : bleu, `CHAOS` : rouge).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LiveSide {
    Order,
    Chaos,
}
/// Totaux visibles au tableau des scores. Ni or, ni sorts, ni vision adverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveTeamTotals {
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub creep_score: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LiveTeams {
    pub allies: LiveTeamTotals,
    pub enemies: LiveTeamTotals,
}
/// Événement public annoncé à tous. `ally` : camp de l'auteur relatif au joueur local,
/// `None` si l'auteur n'est pas un joueur identifiable (sbire) ou si le nom est ambigu.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveEvent {
    pub id: u32,
    pub name: String,
    pub time: f64,
    pub ally: Option<bool>,
    pub involves_local_player: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveGame {
    pub game_time: f64,
    pub game_mode: String,
    pub map_number: u32,
    pub player: LivePlayer,
    pub teams: Option<LiveTeams>,
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
    /// Dernière lecture valide de la partie terminée, conservée en mémoire jusqu'au bilan
    /// puis purgée (nouvelle partie ou sortie des écrans d'après-partie).
    pub postgame: Option<LiveGame>,
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
/// Nombre fini et positif, sinon `None` : les champs optionnels ne sont jamais devinés.
fn optional_time(value: &Value) -> Option<f64> {
    time(value).ok()
}
fn side(value: &Value) -> Option<LiveSide> {
    match value.as_str()? {
        "ORDER" => Some(LiveSide::Order),
        "CHAOS" => Some(LiveSide::Chaos),
        _ => None,
    }
}
fn ability_levels(abilities: &Value) -> Option<LiveAbilityLevels> {
    let level = |key: &str| number(&abilities[key]["abilityLevel"]).ok();
    Some(LiveAbilityLevels {
        q: level("Q")?,
        w: level("W")?,
        e: level("E")?,
        r: level("R")?,
    })
}
/// Un nom d'événement désigne un joueur seulement s'il est unique parmi les identifiants
/// (`riotId`, nom court, ancien pseudo). Le nom ne sort jamais de cette fonction.
fn resolve<'a>(players: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = players.iter().filter(|p| {
        ["riotId", "riotIdGameName", "summonerName"]
            .iter()
            .any(|field| p[*field].as_str() == Some(name))
    });
    let found = matches.next()?;
    matches.next().is_none().then_some(found)
}
fn totals(players: &[Value], wanted: LiveSide) -> Option<LiveTeamTotals> {
    let mut total = LiveTeamTotals {
        kills: 0,
        deaths: 0,
        assists: 0,
        creep_score: 0,
    };
    for player in players.iter().filter(|p| side(&p["team"]) == Some(wanted)) {
        let scores = &player["scores"];
        total.kills = total.kills.saturating_add(number(&scores["kills"]).ok()?);
        total.deaths = total.deaths.saturating_add(number(&scores["deaths"]).ok()?);
        total.assists = total
            .assists
            .saturating_add(number(&scores["assists"]).ok()?);
        total.creep_score = total
            .creep_score
            .saturating_add(number(&scores["creepScore"]).ok()?);
    }
    Some(total)
}
fn teams(players: &[Value], own: Option<LiveSide>) -> Option<LiveTeams> {
    let own = own?;
    let other = match own {
        LiveSide::Order => LiveSide::Chaos,
        LiveSide::Chaos => LiveSide::Order,
    };
    Some(LiveTeams {
        allies: totals(players, own)?,
        enemies: totals(players, other)?,
    })
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
    let own_side = side(&own_player["team"]);
    let mut events = Vec::new();
    for event in value["events"]["Events"]
        .as_array()
        .ok_or(LiveError::Invalid)?
    {
        let name = event["EventName"].as_str().unwrap_or_default();
        // Liste blanche : événements annoncés à tous. Noms issus de la documentation Riot et
        // du ticket #102 ; tout autre événement (Multikill, FirstBrick…) est ignoré.
        let author = match name {
            "FirstBlood" => "Recipient",
            "Ace" => "Acer",
            "ChampionKill" | "DragonKill" | "HeraldKill" | "BaronKill" | "TurretKilled"
            | "InhibKilled" => "KillerName",
            "GameStart" | "MinionsSpawning" | "GameEnd" => "",
            _ => continue,
        };
        let is_local = |who: &Value| {
            who.as_str()
                .and_then(|n| resolve(players, n))
                .is_some_and(|p| std::ptr::eq(p, own_player))
        };
        let mut involves_local_player = is_local(&event[author])
            || event["Assisters"]
                .as_array()
                .is_some_and(|names| names.iter().any(is_local));
        if name == "ChampionKill" {
            involves_local_player |= is_local(&event["VictimName"]);
        }
        let team = if name == "Ace" {
            side(&event["AcingTeam"])
        } else {
            event[author]
                .as_str()
                .and_then(|n| resolve(players, n))
                .and_then(|p| side(&p["team"]))
        };
        events.push(LiveEvent {
            id: number(&event["EventID"])?,
            name: name.into(),
            time: time(&event["EventTime"])?,
            ally: own_side.zip(team).map(|(own, other)| own == other),
            involves_local_player,
        });
    }
    events.sort_by_key(|e| e.id);
    events.dedup_by_key(|e| e.id);
    if events.len() > MAX_EVENTS {
        events.drain(..events.len() - MAX_EVENTS);
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
            current_gold: optional_time(&active["currentGold"]),
            ward_score: optional_time(&scores["wardScore"]),
            is_dead: own_player["isDead"].as_bool(),
            respawn_timer: optional_time(&own_player["respawnTimer"]),
            ability_levels: ability_levels(&active["abilities"]),
            team: own_side,
            position: own_player["position"]
                .as_str()
                .filter(|r| valid_role(r))
                .map(str::to_owned),
        },
        teams: teams(players, own_side),
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
    /// Dernière lecture valide de la partie en cours : le jeu s'arrête souvent avant le
    /// changement de phase LCU, la lecture suivante échoue et vide `snapshot.game`.
    last_ok: Option<LiveGame>,
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
            // Début de partie : l'ancien bilan est purgé ; fin : la dernière lecture devient le bilan.
            self.snapshot.postgame = if active { None } else { self.last_ok.take() };
            self.last_ok = None;
            self.snapshot.generation = self.snapshot.generation.saturating_add(1);
            self.snapshot.game = None;
            self.snapshot.context = if active { self.pending.take() } else { None };
            self.snapshot.status = if active {
                LiveStatus::Waiting
            } else {
                LiveStatus::Idle
            };
        }
        // Purge dès que le joueur quitte les écrans d'après-partie (salon, file, sélection…).
        // Une session déconnectée (passage synthétique à vide) ne purge rien.
        if session.connected
            && session
                .phase
                .is_some_and(|p| !p.is_in_game() && !p.is_post_game())
        {
            self.snapshot.postgame = None;
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
                self.last_ok = Some(game.clone());
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
