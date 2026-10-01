//! Orchestration d'une collecte : réservation des travaux, appels Riot, décisions
//! en cas d'erreur et arrêt propre.

use std::collections::hash_map::RandomState;
use std::future::Future;
use std::hash::{BuildHasher, Hasher};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use thiserror::Error;
use tokio::task::JoinSet;
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::config::{RetryPolicy, RunParams, RuntimeOptions};
use crate::model::{self, MatchCheck};
use crate::rate_limit::{Governor, RateLimiter};
use crate::riot_client::{Request, RiotClient, RiotError, Transport};
use crate::storage::{
    Job, JobKind, MatchIdsPayload, MatchPayload, ParticipantRankPayload, RetryKind, RunRecord,
    RunStatus, SeedPagePayload, Storage, StorageError, TimelinePayload,
};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
    #[error("tâche de collecte interrompue : {0}")]
    Task(String),
}

/// Pourquoi une exécution s'est arrêtée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Plus aucun travail : cible atteinte ou sources épuisées.
    Finished,
    /// Arrêt demandé (Ctrl+C).
    Interrupted,
    /// Riot refuse la clé (401/403) : collecte suspendue, progression conservée.
    AuthRejected(u16),
    CallBudget,
    MaxDuration,
}

impl StopReason {
    /// Valeur enregistrée dans `collection_runs.status_reason`.
    pub fn as_str(self) -> &'static str {
        match self {
            StopReason::Finished => "finished",
            StopReason::Interrupted => "interrupted",
            StopReason::AuthRejected(_) => "riot_auth_rejected",
            StopReason::CallBudget => "call_budget_exhausted",
            StopReason::MaxDuration => "max_duration_reached",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunOutcome {
    pub run_id: i64,
    pub status: RunStatus,
    pub reason: StopReason,
    pub retained: i64,
    pub target: i64,
}

/// Décision prise après une erreur Riot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Decision {
    Retry { delay: Duration, kind: RetryKind },
    Fail,
    TimelineUnavailable,
    Suspend(u16),
}

/// Politique d'erreur (voir le tableau du ticket #17).
pub fn decide(
    kind: JobKind,
    err: &RiotError,
    attempts: i32,
    not_found_count: i32,
    policy: &RetryPolicy,
    jitter: f64,
) -> Decision {
    match err {
        RiotError::RateLimited { pause, .. } => Decision::Retry {
            delay: *pause,
            kind: RetryKind::RateLimited,
        },
        RiotError::Unauthorized(status) => Decision::Suspend(*status),
        RiotError::NotFound => match kind {
            JobKind::Match | JobKind::Timeline => {
                let nf = not_found_count.max(0) as u32;
                if nf < policy.not_found_retries {
                    Decision::Retry {
                        delay: policy.not_found_delay.saturating_mul(nf + 1),
                        kind: RetryKind::NotFound,
                    }
                } else if kind == JobKind::Timeline {
                    Decision::TimelineUnavailable
                } else {
                    Decision::Fail
                }
            }
            JobKind::SeedPage | JobKind::MatchIds | JobKind::ParticipantRank => Decision::Fail,
        },
        RiotError::BadRequest => Decision::Fail,
        RiotError::Server(_)
        | RiotError::Unexpected(_)
        | RiotError::Transport(_)
        | RiotError::InvalidBody(_) => {
            let attempts = attempts.max(0) as u32;
            if attempts + 1 >= policy.max_attempts {
                Decision::Fail
            } else {
                Decision::Retry {
                    delay: policy.backoff(attempts, jitter),
                    kind: RetryKind::Attempt,
                }
            }
        }
    }
}

enum JobEffect {
    Continue,
    Suspend(u16),
}

fn jitter() -> f64 {
    let mut h = RandomState::new().build_hasher();
    h.write_u64(0);
    (h.finish() % 10_000) as f64 / 10_000.0
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Fenêtre `[début, fin)` en millisecondes, fin arrondie à la seconde (les
/// historiques Riot prennent des secondes).
pub fn collection_window(now_ms: i64, days: u32) -> (i64, i64) {
    let end = now_ms - now_ms.rem_euclid(1000);
    (end - i64::from(days) * DAY_MS, end)
}

pub struct Collector<T: Transport> {
    storage: Storage,
    client: RiotClient<T>,
    options: RuntimeOptions,
    /// Sérialise les réservations pour que la cible ne soit jamais dépassée.
    claim_lock: tokio::sync::Mutex<()>,
}

impl<T: Transport> Collector<T> {
    pub fn new(storage: Storage, transport: T, options: RuntimeOptions) -> Arc<Self> {
        let governor = Arc::new(Governor::new(RateLimiter::new(
            options.initial_app_limits.clone(),
        )));
        let client = RiotClient::new(transport, governor, options.retry.default_rate_limit_pause);
        Arc::new(Self {
            storage,
            client,
            options,
            claim_lock: tokio::sync::Mutex::new(()),
        })
    }

    /// Appels Riot envoyés par ce processus.
    pub fn calls(&self) -> u64 {
        self.client.calls()
    }

    /// Crée une exécution dont la fenêtre se termine à `now_ms`.
    pub async fn start_run(&self, params: &RunParams, now_ms: i64) -> Result<i64, CollectorError> {
        params.validate()?;
        let (start, end) = collection_window(now_ms, params.window_days);
        let run_id = self.storage.create_run(params, start, end).await?;
        info!(run_id, "exécution créée");
        Ok(run_id)
    }

    /// Exécute (ou reprend) une exécution jusqu'à la fin, un arrêt demandé via
    /// `shutdown`, une limite atteinte ou un refus de la clé.
    pub async fn execute(
        self: &Arc<Self>,
        run_id: i64,
        shutdown: impl Future<Output = ()>,
    ) -> Result<RunOutcome, CollectorError> {
        let run = Arc::new(self.storage.load_run(run_id).await?);
        let target = i64::from(run.params.target_matches);
        if matches!(run.status, RunStatus::Completed | RunStatus::Incomplete) {
            info!(run_id, "exécution déjà terminée : rien à faire");
            return Ok(RunOutcome {
                run_id,
                status: run.status,
                reason: StopReason::Finished,
                retained: self.storage.retained_count(run_id).await?,
                target,
            });
        }
        let reset = self.storage.reset_interrupted(run_id).await?;
        if reset > 0 {
            info!(run_id, reset, "travaux interrompus remis en attente");
        }
        self.storage
            .set_run_status(run_id, RunStatus::Running, None)
            .await?;

        let result = self.drive(&run, target, shutdown).await;
        let reason = match result {
            Ok(reason) => reason,
            Err(e) => {
                let _ = self
                    .storage
                    .set_run_status(run_id, RunStatus::Paused, Some("error"))
                    .await;
                return Err(e);
            }
        };
        if matches!(reason, StopReason::AuthRejected(_)) {
            // Tous les travailleurs sont drainés : les refus peuvent redevenir rejouables.
            self.storage.reset_interrupted(run_id).await?;
        }
        let retained = self.storage.retained_count(run_id).await?;
        let status = match reason {
            StopReason::Finished if retained >= target => RunStatus::Completed,
            StopReason::Finished => RunStatus::Incomplete,
            _ => RunStatus::Paused,
        };
        self.storage
            .set_run_status(run_id, status, Some(reason.as_str()))
            .await?;
        Ok(RunOutcome {
            run_id,
            status,
            reason,
            retained,
            target,
        })
    }

    async fn drive(
        self: &Arc<Self>,
        run: &Arc<RunRecord>,
        target: i64,
        shutdown: impl Future<Output = ()>,
    ) -> Result<StopReason, CollectorError> {
        let deadline = self.options.max_duration.map(|d| Instant::now() + d);
        let budget = i64::try_from(run.params.call_budget).unwrap_or(i64::MAX);
        let concurrency = self.options.concurrency.max(1);
        let mut tasks: JoinSet<Result<JobEffect, StorageError>> = JoinSet::new();
        let mut stop: Option<StopReason> = None;
        let mut failure: Option<CollectorError> = None;
        tokio::pin!(shutdown);

        loop {
            while stop.is_none() && tasks.len() < concurrency {
                if deadline.is_some_and(|d| Instant::now() >= d) {
                    stop = Some(StopReason::MaxDuration);
                    break;
                }
                let calls = self.storage.calls_made(run.id).await? + tasks.len() as i64;
                if calls >= budget {
                    stop = Some(StopReason::CallBudget);
                    break;
                }
                let job = {
                    let _guard = self.claim_lock.lock().await;
                    self.storage.claim_next(run.id, target).await?
                };
                let Some(job) = job else { break };
                let this = Arc::clone(self);
                let run = Arc::clone(run);
                tasks.spawn(async move { this.run_job(job, &run).await });
            }

            if tasks.is_empty() {
                if stop.is_some() {
                    break;
                }
                match self.storage.next_due_in(run.id, target).await? {
                    None => {
                        stop = Some(StopReason::Finished);
                        break;
                    }
                    Some(secs) => {
                        let wait = Duration::from_secs_f64(secs.clamp(0.05, 30.0));
                        tokio::select! {
                            _ = tokio::time::sleep(wait) => {}
                            _ = &mut shutdown => stop = Some(StopReason::Interrupted),
                        }
                    }
                }
                continue;
            }

            tokio::select! {
                res = tasks.join_next() => match res {
                    Some(Ok(Ok(JobEffect::Continue))) | None => {}
                    Some(Ok(Ok(JobEffect::Suspend(status)))) => {
                        // Un refus de clé prime sur une borne détectée pendant la requête.
                        stop = Some(StopReason::AuthRejected(status));
                    }
                    Some(Ok(Err(e))) => {
                        stop.get_or_insert(StopReason::Interrupted);
                        failure.get_or_insert(e.into());
                    }
                    Some(Err(e)) => {
                        stop.get_or_insert(StopReason::Interrupted);
                        failure.get_or_insert(CollectorError::Task(e.to_string()));
                    }
                },
                _ = &mut shutdown, if stop.is_none() => stop = Some(StopReason::Interrupted),
                // Réveil régulier : une nouvelle tentative peut être devenue due.
                _ = tokio::time::sleep(Duration::from_secs(1)) => {}
            }
        }

        match failure {
            Some(e) => Err(e),
            // Les dernières tâches peuvent avoir terminé toute la collecte en
            // consommant exactement le budget. Après leur drainage, il n'y a
            // alors aucune raison de demander une reprise ou un budget supérieur.
            None if stop == Some(StopReason::CallBudget)
                && self.storage.next_due_in(run.id, target).await?.is_none() =>
            {
                Ok(StopReason::Finished)
            }
            None => Ok(stop.unwrap_or(StopReason::Finished)),
        }
    }

    async fn run_job(&self, job: Job, run: &RunRecord) -> Result<JobEffect, StorageError> {
        debug!(job = job.id, kind = job.kind.as_str(), "travail réservé");
        match job.kind {
            JobKind::SeedPage => {
                let p: SeedPagePayload = job.payload()?;
                let mut req = Request::league_entries_for(
                    &run.scope.platform_id,
                    p.tier,
                    p.division,
                    p.page,
                )?;
                // La Flex utilise ses propres seeds ; les autres modes partent du classement Solo.
                if run.scope.queue_id == 440 {
                    for segment in &mut req.segments {
                        if segment == "RANKED_SOLO_5x5" {
                            *segment = "RANKED_FLEX_SR".into();
                        }
                    }
                }
                match self.client.get(&req).await {
                    Ok(body) => match model::parse_league_entries(&body) {
                        Ok(entries) => {
                            let n = self
                                .storage
                                .complete_seed_page(&job, &p, &entries, &run.params)
                                .await?;
                            info!(
                                tier = p.tier.as_str(),
                                division = p.division.as_str(),
                                page = p.page,
                                seeds = n,
                                "joueurs de départ enregistrés"
                            );
                            Ok(JobEffect::Continue)
                        }
                        Err(msg) => self.handle_error(&job, RiotError::InvalidBody(msg)).await,
                    },
                    Err(e) => self.handle_error(&job, e).await,
                }
            }
            JobKind::MatchIds => {
                let p: MatchIdsPayload = job.payload()?;
                let requested = run
                    .params
                    .max_matches_per_seed
                    .saturating_sub(p.start)
                    .clamp(1, 100);
                let req = Request::match_ids_for(
                    &run.scope.platform_id,
                    run.scope.queue_id,
                    &p.puuid,
                    p.start,
                    requested,
                    run.scope.window_start_ms / 1000,
                    run.scope.window_end_ms / 1000,
                )?;
                match self.client.get(&req).await {
                    Ok(body) => match model::parse_match_ids(&body) {
                        Ok(ids) => {
                            self.storage
                                .complete_match_ids(
                                    &job,
                                    &p,
                                    requested,
                                    &ids,
                                    &run.scope,
                                    &run.params,
                                )
                                .await?;
                            Ok(JobEffect::Continue)
                        }
                        Err(msg) => self.handle_error(&job, RiotError::InvalidBody(msg)).await,
                    },
                    Err(e) => self.handle_error(&job, e).await,
                }
            }
            JobKind::Match => {
                let p: MatchPayload = job.payload()?;
                // Déjà en base (autre exécution) : pas de nouveau téléchargement.
                if let Some(facts) = self.storage.find_match(&p.match_id).await? {
                    match run.scope.check_stored(&facts) {
                        Ok(()) => self.storage.link_existing_match(&job, &p).await?,
                        Err(ex) => self.storage.exclude_match(&job, ex, 0).await?,
                    }
                    return Ok(JobEffect::Continue);
                }
                match self
                    .client
                    .get(&Request::match_detail_for(
                        &run.scope.platform_id,
                        &p.match_id,
                    )?)
                    .await
                {
                    Ok(body) => match model::check_match(&body, &p.match_id, &run.scope) {
                        Ok(MatchCheck::Accepted(facts, raw)) => {
                            self.storage.store_match(&job, &p, &facts, &raw).await?;
                            info!(match_id = %p.match_id, "partie enregistrée");
                            Ok(JobEffect::Continue)
                        }
                        Ok(MatchCheck::Excluded(ex)) => {
                            self.storage.exclude_match(&job, ex, 1).await?;
                            info!(match_id = %p.match_id, reason = ex.outcome(), "partie exclue");
                            Ok(JobEffect::Continue)
                        }
                        Err(msg) => self.handle_error(&job, RiotError::InvalidBody(msg)).await,
                    },
                    Err(e) => self.handle_error(&job, e).await,
                }
            }
            JobKind::ParticipantRank => {
                let p: ParticipantRankPayload = job.payload()?;
                if self.storage.ranks_fresh(&p).await? {
                    self.storage.finish_job(&job, "cached", 0).await?;
                    return Ok(JobEffect::Continue);
                }
                let request = Request::participant_ranks(&p.platform_id, &p.puuid)?;
                match self.client.get(&request).await {
                    Ok(body) => match model::parse_participant_ranks(&body) {
                        Ok(ranks) => {
                            self.storage
                                .store_participant_ranks(&job, &p, &ranks)
                                .await?;
                            Ok(JobEffect::Continue)
                        }
                        Err(message) => {
                            self.handle_error(&job, RiotError::InvalidBody(message))
                                .await
                        }
                    },
                    Err(error) => self.handle_error(&job, error).await,
                }
            }
            JobKind::Timeline => {
                let p: TimelinePayload = job.payload()?;
                if self.storage.timeline_available(&p.match_id).await? {
                    self.storage.finish_job(&job, "already_present", 0).await?;
                    return Ok(JobEffect::Continue);
                }
                match self
                    .client
                    .get(&Request::timeline_for(&run.scope.platform_id, &p.match_id)?)
                    .await
                {
                    Ok(body) => match model::check_timeline(&body, &p.match_id) {
                        Ok(raw) => {
                            self.storage.store_timeline(&job, &p.match_id, &raw).await?;
                            Ok(JobEffect::Continue)
                        }
                        Err(msg) => self.handle_error(&job, RiotError::InvalidBody(msg)).await,
                    },
                    Err(e) => self.handle_error(&job, e).await,
                }
            }
        }
    }

    /// Applique la politique d'erreur. Tous les chemins comptent l'appel envoyé.
    async fn handle_error(&self, job: &Job, err: RiotError) -> Result<JobEffect, StorageError> {
        let message = err.to_string();
        let decision = decide(
            job.kind,
            &err,
            job.attempts,
            job.not_found_count,
            &self.options.retry,
            jitter(),
        );
        match decision {
            Decision::Retry { delay, kind } => {
                warn!(job = job.id, kind = job.kind.as_str(), error = %message, ?delay, "nouvelle tentative programmée");
                self.storage
                    .retry_job(job, delay, kind, &message, 1)
                    .await?;
                Ok(JobEffect::Continue)
            }
            Decision::Fail => {
                warn!(job = job.id, kind = job.kind.as_str(), error = %message, "travail en échec");
                self.storage.fail_job(job, &message, 1).await?;
                Ok(JobEffect::Continue)
            }
            Decision::TimelineUnavailable => {
                let p: TimelinePayload = job.payload()?;
                warn!(match_id = %p.match_id, "timeline indisponible après revalidation");
                self.storage
                    .mark_timeline_unavailable(job, &p.match_id)
                    .await?;
                Ok(JobEffect::Continue)
            }
            Decision::Suspend(status) => {
                warn!(status, "clé Riot refusée : collecte suspendue");
                self.storage.suspend_job(job, &message, 1).await?;
                Ok(JobEffect::Suspend(status))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_limit::LimitScope;
    use crate::riot_client::TransportError;

    fn policy() -> RetryPolicy {
        RetryPolicy::default()
    }

    #[test]
    fn un_429_reprend_apres_la_pause_sans_compter_de_tentative() {
        let err = RiotError::RateLimited {
            scope: LimitScope::Application,
            pause: Duration::from_secs(7),
        };
        assert_eq!(
            decide(JobKind::Match, &err, 4, 0, &policy(), 0.5),
            Decision::Retry {
                delay: Duration::from_secs(7),
                kind: RetryKind::RateLimited
            }
        );
    }

    #[test]
    fn un_401_ou_403_suspend_la_collecte() {
        for status in [401, 403] {
            assert_eq!(
                decide(
                    JobKind::Timeline,
                    &RiotError::Unauthorized(status),
                    0,
                    0,
                    &policy(),
                    0.5
                ),
                Decision::Suspend(status)
            );
        }
    }

    #[test]
    fn une_timeline_404_est_revalidee_puis_declaree_indisponible() {
        let p = policy();
        assert_eq!(
            decide(JobKind::Timeline, &RiotError::NotFound, 0, 0, &p, 0.5),
            Decision::Retry {
                delay: p.not_found_delay,
                kind: RetryKind::NotFound
            }
        );
        assert_eq!(
            decide(JobKind::Timeline, &RiotError::NotFound, 0, 1, &p, 0.5),
            Decision::Retry {
                delay: p.not_found_delay * 2,
                kind: RetryKind::NotFound
            }
        );
        assert_eq!(
            decide(JobKind::Timeline, &RiotError::NotFound, 0, 2, &p, 0.5),
            Decision::TimelineUnavailable
        );
        // Une partie introuvable n'est jamais déclarée « indisponible » : elle échoue.
        assert_eq!(
            decide(JobKind::Match, &RiotError::NotFound, 0, 2, &p, 0.5),
            Decision::Fail
        );
        assert_eq!(
            decide(JobKind::MatchIds, &RiotError::NotFound, 0, 0, &p, 0.5),
            Decision::Fail
        );
    }

    #[test]
    fn erreurs_serveur_reseau_et_json_sont_reessayees_puis_echouent() {
        let p = policy();
        for err in [
            RiotError::Server(503),
            RiotError::Transport(TransportError::Timeout),
            RiotError::InvalidBody("x".into()),
        ] {
            assert!(matches!(
                decide(JobKind::Timeline, &err, 0, 0, &p, 1.0),
                Decision::Retry {
                    kind: RetryKind::Attempt,
                    ..
                }
            ));
            // Jamais « indisponible » : une erreur serveur reste une erreur.
            assert_eq!(
                decide(
                    JobKind::Timeline,
                    &err,
                    p.max_attempts as i32 - 1,
                    0,
                    &p,
                    1.0
                ),
                Decision::Fail
            );
        }
    }

    #[test]
    fn un_400_echoue_immediatement() {
        assert_eq!(
            decide(
                JobKind::MatchIds,
                &RiotError::BadRequest,
                0,
                0,
                &policy(),
                0.5
            ),
            Decision::Fail
        );
    }

    #[test]
    fn la_fenetre_est_figee_a_la_seconde() {
        let (start, end) = collection_window(1_700_000_000_123, 14);
        assert_eq!(end, 1_700_000_000_000);
        assert_eq!(end - start, 14 * DAY_MS);
    }
}
