//! Quotas persistants communs aux processus API et collecteur d'une même base.
use crate::rate_limit::{parse_limits, parse_retry_after, LimitScope};
use crate::riot_client::{RawResponse, Request, Transport, TransportError};
use crate::storage::Storage;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
struct Window {
    limit: u32,
    period_ms: i64,
    sent: Vec<i64>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Bucket {
    windows: Vec<Window>,
    blocked_until: i64,
}
#[derive(Clone, Serialize, Deserialize)]
struct QuotaState {
    app: Bucket,
    methods: BTreeMap<String, Bucket>,
    #[serde(default)]
    next_id: u64,
    #[serde(default)]
    inflight: BTreeMap<u64, (String, i64)>,
}
impl Default for QuotaState {
    fn default() -> Self {
        Self {
            app: Bucket {
                windows: vec![
                    Window {
                        limit: 20,
                        period_ms: 1000,
                        sent: vec![],
                    },
                    Window {
                        limit: 100,
                        period_ms: 120_000,
                        sent: vec![],
                    },
                ],
                blocked_until: 0,
            },
            methods: BTreeMap::new(),
            next_id: 0,
            inflight: BTreeMap::new(),
        }
    }
}
impl QuotaState {
    fn reserve(&mut self, method: &str, now: i64) -> Option<i64> {
        // Le transport HTTP est borné à 15 s ; les abandons restent prudents pendant 60 s.
        self.inflight
            .retain(|_, (_, at)| at.saturating_add(60_000) > now);
        let bucket = self.methods.entry(method.into()).or_default();
        let until = self.app.available_at(now).max(bucket.available_at(now));
        if until > now {
            return Some(until);
        }
        for bucket in [&mut self.app, bucket] {
            for window in &mut bucket.windows {
                window.sent.push(now);
            }
        }
        self.next_id = self.next_id.saturating_add(1);
        self.inflight.insert(self.next_id, (method.into(), now));
        None
    }

    fn observe(&mut self, method: &str, now: i64, response: &RawResponse, reservation: u64) {
        self.inflight.remove(&reservation);
        self.inflight
            .retain(|_, (_, at)| at.saturating_add(60_000) > now);
        let method_pending = self.inflight.values().filter(|(m, _)| m == method).count();
        self.app.observe(
            now,
            response.header("x-app-rate-limit"),
            response.header("x-app-rate-limit-count"),
            self.inflight.len(),
        );
        self.methods.entry(method.into()).or_default().observe(
            now,
            response.header("x-method-rate-limit"),
            response.header("x-method-rate-limit-count"),
            method_pending,
        );
        if response.status == 429 {
            let delay = parse_retry_after(response.header("retry-after"))
                .unwrap_or(Duration::from_secs(120));
            let until = now.saturating_add(i64::try_from(delay.as_millis()).unwrap_or(i64::MAX));
            let bucket = if LimitScope::from_header(response.header("x-rate-limit-type"))
                == LimitScope::Method
            {
                self.methods.entry(method.into()).or_default()
            } else {
                &mut self.app
            };
            bucket.blocked_until = bucket.blocked_until.max(until);
        }
    }
}

impl Bucket {
    fn available_at(&mut self, now: i64) -> i64 {
        let mut until = self.blocked_until;
        for window in &mut self.windows {
            window
                .sent
                .retain(|sent| sent.saturating_add(window.period_ms) > now);
            if window.limit == 0 {
                until = until.max(now.saturating_add(window.period_ms));
            } else if window.sent.len() >= window.limit as usize {
                until = until.max(
                    window.sent[window.sent.len() - window.limit as usize]
                        .saturating_add(window.period_ms),
                );
            }
        }
        until
    }

    fn observe(&mut self, now: i64, limits: Option<&str>, counts: Option<&str>, pending: usize) {
        self.available_at(now);
        if let Some(limits) = limits.and_then(parse_limits) {
            let mut previous = std::mem::take(&mut self.windows);
            let history = previous
                .iter()
                .max_by_key(|w| w.period_ms)
                .map(|w| w.sent.clone())
                .unwrap_or_default();
            for (limit, duration) in limits {
                let period_ms = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
                let old = previous
                    .iter()
                    .position(|w| w.period_ms == period_ms)
                    .map(|i| previous.remove(i));
                self.windows.push(Window {
                    limit,
                    period_ms,
                    sent: old.map(|w| w.sent).unwrap_or_else(|| history.clone()),
                });
            }
        }
        if let Some(counts) = counts.and_then(parse_limits) {
            for (count, duration) in counts {
                let period_ms = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
                if let Some(window) = self.windows.iter_mut().find(|w| w.period_ms == period_ms) {
                    // Le compteur externe peut inclure des réservations déjà connues.
                    // Conserver le maximum est prudent ; ne jamais décrémenter sur une réponse tardive.
                    let count = (count as usize)
                        .saturating_add(pending)
                        .min(window.limit as usize);
                    if count > window.sent.len() {
                        window.sent.resize(count, now);
                    }
                }
            }
        }
    }
}

/// Transport réel coordonné entre tous les processus utilisant la même base PostgreSQL.
/// Les clés ne sont jamais enregistrées ; une base correspond à un produit Riot.
pub struct CoordinatedTransport<T> {
    inner: T,
    storage: Storage,
}
impl<T> CoordinatedTransport<T> {
    pub fn new(inner: T, storage: Storage) -> Self {
        Self { inner, storage }
    }

    async fn change<R>(
        &self,
        request: &Request,
        change: impl FnOnce(&mut QuotaState, i64) -> R,
    ) -> Result<R, TransportError> {
        self.change_inner(request, change)
            .await
            .map_err(|_| TransportError::Network("quotas partagés indisponibles".into()))
    }

    async fn change_inner<R>(
        &self,
        request: &Request,
        change: impl FnOnce(&mut QuotaState, i64) -> R,
    ) -> Result<R, sqlx::Error> {
        let mut connection = self.storage.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        let host = request.route.host();
        sqlx::query("INSERT INTO riot_shared_quota (host,state) VALUES ($1,$2) ON CONFLICT (host) DO NOTHING")
            .bind(&host).bind(sqlx::types::Json(QuotaState::default())).execute(&mut *tx).await?;
        let row = sqlx::query("SELECT state FROM riot_shared_quota WHERE host=$1 FOR UPDATE")
            .bind(&host)
            .fetch_one(&mut *tx)
            .await?;
        // Horloge lue après acquisition du verrou, pas au début d'une attente SQL.
        let now: i64 =
            sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut *tx)
                .await?;
        let mut state: sqlx::types::Json<QuotaState> = row.try_get("state")?;
        let result = change(&mut state, now);
        sqlx::query("UPDATE riot_shared_quota SET state=$2 WHERE host=$1")
            .bind(&host)
            .bind(state)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(result)
    }
}

impl<T: Transport> Transport for CoordinatedTransport<T> {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        let reservation = loop {
            let (delay, reservation) = self
                .change(request, |state, now| {
                    (
                        state
                            .reserve(request.endpoint.name(), now)
                            .map(|until| until.saturating_sub(now)),
                        state.next_id,
                    )
                })
                .await?;
            match delay {
                Some(ms) => tokio::time::sleep(Duration::from_millis(ms.max(1) as u64)).await,
                None => break reservation,
            }
        };
        // La réservation est déjà validée : annulation/timeout conserve sa consommation.
        let response = self.inner.send(request).await?;
        self.change(request, |state, now| {
            state.observe(request.endpoint.name(), now, &response, reservation)
        })
        .await?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn apprendre_une_nouvelle_fenetre_ne_perd_pas_les_reservations_en_vol() {
        let mut state = QuotaState::default();
        state.reserve("method", 1000);
        state.reserve("method", 1001);
        state.reserve("method", 1002);
        state.observe(
            "method",
            1003,
            &RawResponse {
                status: 200,
                headers: vec![
                    ("x-app-rate-limit".into(), "10:2".into()),
                    ("x-app-rate-limit-count".into(), "5:2".into()),
                ],
                body: vec![],
            },
            1,
        );
        // Cinq appels déjà vus par Riot, plus deux en vol non encore confirmés.
        assert_eq!(state.app.windows[0].sent.len(), 7);
    }
    #[test]
    fn une_reservation_persiste_et_la_fenetre_glisse_sans_rafale() {
        let mut state = QuotaState::default();
        state.app.windows = vec![Window {
            limit: 1,
            period_ms: 100,
            sent: vec![],
        }];
        assert_eq!(state.reserve("method", 1000), None);
        let mut reloaded: QuotaState =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert_eq!(reloaded.reserve("other", 1001), Some(1100));
        assert_eq!(reloaded.reserve("other", 1100), None);
        assert_eq!(reloaded.reserve("method", 1101), Some(1200));
    }
    #[test]
    fn les_methodes_et_blocages_restent_distincts() {
        let mut state = QuotaState::default();
        state.methods.insert(
            "blocked".into(),
            Bucket {
                windows: vec![],
                blocked_until: 2000,
            },
        );
        assert_eq!(state.reserve("blocked", 1000), Some(2000));
        assert_eq!(state.reserve("free", 1000), None);
        assert_eq!(state.app.windows[0].sent.len(), 1);
    }
}
