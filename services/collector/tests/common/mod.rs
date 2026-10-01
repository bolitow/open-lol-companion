//! Outils des tests d'intégration : base PostgreSQL jetable et faux serveur Riot.
//!
//! Les tests PostgreSQL ne tournent que si `OLC_TEST_DATABASE_URL` est définie
//! (ex. `postgres://postgres:postgres@localhost:5432/postgres`) ; sinon ils sont
//! ignorés avec un message. Aucun appel à Riot : tout passe par `FakeRiot`.

#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use olc_collector::config::{RetryPolicy, RuntimeOptions};
use olc_collector::model::fixtures;
use olc_collector::riot_client::{Endpoint, RawResponse, Request, Transport, TransportError};
use olc_collector::storage::Storage;
use serde_json::{json, Value};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, Executor};
use tokio::sync::Notify;

pub const DAY_MS: i64 = 24 * 60 * 60 * 1000;

pub struct TestDb {
    pub storage: Storage,
    admin: PgConnectOptions,
    name: String,
}

impl TestDb {
    /// Base jetable migrée, ou `None` si `OLC_TEST_DATABASE_URL` est absente.
    pub async fn create() -> Option<Self> {
        let Ok(url) = std::env::var("OLC_TEST_DATABASE_URL") else {
            eprintln!("OLC_TEST_DATABASE_URL absente : test PostgreSQL ignoré");
            return None;
        };
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "olc_test_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let admin = PgConnectOptions::from_str(&url).expect("OLC_TEST_DATABASE_URL invalide");
        let mut conn = admin.connect().await.expect("connexion PostgreSQL");
        conn.execute(format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)").as_str())
            .await
            .unwrap();
        conn.execute(format!("CREATE DATABASE {name}").as_str())
            .await
            .unwrap();
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(admin.clone().database(&name))
            .await
            .unwrap();
        let storage = Storage::from_pool(pool);
        storage.migrate().await.unwrap();
        Some(Self {
            storage,
            admin,
            name,
        })
    }

    /// Autre pool sur la même base : simule un second processus, qui a ses propres
    /// connexions et les perd toutes s'il est tué.
    pub async fn separate_storage(&self) -> Storage {
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect_with(self.admin.clone().database(&self.name))
            .await
            .unwrap();
        Storage::from_pool(pool)
    }

    /// URL de la base jetable pour lancer le vrai binaire en test.
    pub fn database_url(&self) -> String {
        self.admin
            .clone()
            .database(&self.name)
            .to_url_lossy()
            .to_string()
    }

    pub async fn cleanup(self) {
        self.storage.pool().close().await;
        if let Ok(mut conn) = self.admin.connect().await {
            let _ = conn
                .execute(format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", self.name).as_str())
                .await;
        }
    }

    pub async fn scalar(&self, sql: &str) -> i64 {
        sqlx::query_scalar(sql)
            .fetch_one(self.storage.pool())
            .await
            .unwrap()
    }
}

/// Options rapides : délais de quelques millisecondes, quotas larges.
pub fn fast_options() -> RuntimeOptions {
    RuntimeOptions {
        concurrency: 2,
        max_duration: None,
        initial_app_limits: vec![(10_000, Duration::from_secs(1))],
        retry: RetryPolicy {
            max_attempts: 3,
            base_delay: Duration::from_millis(5),
            max_delay: Duration::from_millis(20),
            not_found_retries: 2,
            not_found_delay: Duration::from_millis(5),
            default_rate_limit_pause: Duration::from_millis(5),
        },
    }
}

type Scripted = Result<RawResponse, TransportError>;

#[derive(Default)]
struct State {
    league: HashMap<String, Vec<Value>>,
    histories: HashMap<String, Vec<String>>,
    matches: HashMap<String, Value>,
    timelines: HashMap<String, Value>,
    /// Réponses imposées, consommées avant les données ci-dessus (clé : chemin).
    scripted: HashMap<String, VecDeque<Scripted>>,
    calls: Vec<(Endpoint, String)>,
}

/// Faux serveur Riot en mémoire, conforme aux schémas utilisés.
#[derive(Clone, Default)]
pub struct FakeRiot {
    state: Arc<Mutex<State>>,
    /// Si défini : la première requête de timeline le signale puis ne répond jamais.
    hang_timeline: Option<Arc<Notify>>,
}

pub fn ok_json(v: &Value) -> RawResponse {
    RawResponse {
        status: 200,
        headers: vec![],
        body: serde_json::to_vec(v).unwrap(),
    }
}

pub fn status(code: u16, headers: &[(&str, &str)]) -> RawResponse {
    RawResponse {
        status: code,
        headers: headers
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        body: b"{\"status\":{}}".to_vec(),
    }
}

impl FakeRiot {
    pub fn with_hang(notify: Arc<Notify>) -> Self {
        Self {
            hang_timeline: Some(notify),
            ..Self::default()
        }
    }

    /// Page de classement : PUUID factices.
    pub fn league_page(&self, tier: &str, division: &str, page: u32, puuids: &[&str]) {
        let entries = puuids
            .iter()
            .map(|p| json!({ "puuid": p, "leaguePoints": 50, "tier": tier, "rank": division }))
            .collect();
        self.state
            .lock()
            .unwrap()
            .league
            .insert(format!("{tier}/{division}/{page}"), entries);
    }

    pub fn history(&self, puuid: &str, ids: &[&str]) {
        self.state.lock().unwrap().histories.insert(
            puuid.to_owned(),
            ids.iter().map(|s| (*s).to_owned()).collect(),
        );
    }

    /// Partie EUW1 / 420 commencée à `start_ms`, avec sa timeline.
    pub fn game(&self, match_id: &str, start_ms: i64) {
        self.game_with(match_id, "EUW1", 420, start_ms, true);
    }

    pub fn game_with(
        &self,
        match_id: &str,
        platform: &str,
        queue: i32,
        start_ms: i64,
        timeline: bool,
    ) {
        let mut s = self.state.lock().unwrap();
        s.matches.insert(
            match_id.to_owned(),
            fixtures::match_detail(match_id, platform, queue, start_ms),
        );
        if timeline {
            s.timelines
                .insert(match_id.to_owned(), fixtures::timeline(match_id));
        }
    }

    /// Impose les prochaines réponses d'un chemin (ex. `matches/EUW1_1`).
    pub fn script(&self, path: &str, responses: Vec<Scripted>) {
        self.state
            .lock()
            .unwrap()
            .scripted
            .entry(path.to_owned())
            .or_default()
            .extend(responses);
    }

    pub fn calls(&self, endpoint: Endpoint) -> usize {
        self.state
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|(e, _)| *e == endpoint)
            .count()
    }

    pub fn calls_for(&self, endpoint: Endpoint, id: &str) -> usize {
        self.state
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|(e, k)| *e == endpoint && k == id)
            .count()
    }

    fn respond(&self, request: &Request) -> Scripted {
        let seg = &request.segments;
        let q = |name: &str| {
            request
                .query
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        let mut s = self.state.lock().unwrap();
        let (key, path) = match request.endpoint {
            Endpoint::AccountByRiotId | Endpoint::SummonerByPuuid => return Ok(status(404, &[])),
            Endpoint::LeagueEntries => {
                let key = format!("{}/{}/{}", seg[5], seg[6], q("page"));
                (key.clone(), format!("league/{key}"))
            }
            Endpoint::ApexLeague => (seg[3].clone(), format!("apex/{}", seg[3])),
            Endpoint::ParticipantRanks => (seg[5].clone(), format!("ranks/{}", seg[5])),
            Endpoint::MatchIdsByPuuid => (seg[5].clone(), format!("ids/{}", seg[5])),
            Endpoint::Match => (seg[4].clone(), format!("matches/{}", seg[4])),
            Endpoint::Timeline => (seg[4].clone(), format!("matches/{}/timeline", seg[4])),
        };
        s.calls.push((request.endpoint, key.clone()));
        if let Some(next) = s.scripted.get_mut(&path).and_then(VecDeque::pop_front) {
            return next;
        }
        let not_found = || Ok(status(404, &[]));
        match request.endpoint {
            Endpoint::AccountByRiotId | Endpoint::SummonerByPuuid => not_found(),
            Endpoint::LeagueEntries => Ok(ok_json(&Value::Array(
                s.league.get(&key).cloned().unwrap_or_default(),
            ))),
            Endpoint::ApexLeague => Ok(ok_json(&json!({"entries": []}))),
            Endpoint::ParticipantRanks => Ok(ok_json(&json!([]))),
            Endpoint::MatchIdsByPuuid => {
                let all = s.histories.get(&key).cloned().unwrap_or_default();
                let start: usize = q("start").parse().unwrap();
                let count: usize = q("count").parse().unwrap();
                let page: Vec<String> = all.into_iter().skip(start).take(count).collect();
                Ok(ok_json(&json!(page)))
            }
            Endpoint::Match => s.matches.get(&key).map(ok_json).map_or_else(not_found, Ok),
            Endpoint::Timeline => s
                .timelines
                .get(&key)
                .map(ok_json)
                .map_or_else(not_found, Ok),
        }
    }
}

impl Transport for FakeRiot {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        if request.endpoint == Endpoint::Timeline {
            if let Some(notify) = &self.hang_timeline {
                self.state
                    .lock()
                    .unwrap()
                    .calls
                    .push((request.endpoint, request.segments[4].clone()));
                notify.notify_one();
                std::future::pending::<()>().await;
            }
        }
        self.respond(request)
    }
}
