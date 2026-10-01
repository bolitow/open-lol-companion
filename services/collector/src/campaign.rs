//! Campagnes de collecte bornées, persistées et reprises sans déplacer leurs fenêtres.

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use sqlx::Row;
use thiserror::Error;
use tokio::time::Instant;

use crate::collector::{collection_window, now_ms, Collector, CollectorError, StopReason};
use crate::config::{RunParams, RuntimeOptions};
use crate::riot_client::Transport;
use crate::storage::{RunStatus, Storage, StorageError};

const MAX_DURATION: Duration = Duration::from_secs(24 * 3600);
const MAX_SLICE: Duration = Duration::from_secs(15 * 60);
const MAX_DRAIN: Duration = Duration::from_secs(15);

#[derive(Debug, Error)]
pub enum CampaignError {
    #[error("paramètre de campagne invalide : {0}")]
    Invalid(&'static str),
    #[error("campagne {0} introuvable")]
    NotFound(i64),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Collection(#[from] CollectorError),
    #[error("PostgreSQL : {0}")]
    Db(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CampaignStatus {
    Paused,
    Finished,
}

impl CampaignStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Paused => "paused",
            Self::Finished => "finished",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CampaignOutcome {
    pub campaign_id: i64,
    pub status: CampaignStatus,
    pub reason: String,
    pub completed_runs: usize,
    pub total_runs: usize,
}

/// Crée toutes les exécutions dans une transaction, avec le même instant de fin.
/// Les choix de volumes et de patches sont fournis par le lanceur et restent figés.
pub async fn start(
    storage: &Storage,
    platforms: &[String],
    template: &RunParams,
    now_ms: i64,
    duration: Duration,
) -> Result<i64, CampaignError> {
    if platforms.is_empty() || platforms.iter().collect::<BTreeSet<_>>().len() != platforms.len() {
        return Err(CampaignError::Invalid("plateformes absentes ou dupliquées"));
    }
    if duration.as_millis() == 0 || duration > MAX_DURATION {
        return Err(CampaignError::Invalid(
            "durée attendue : de 1 ms à 24 heures",
        ));
    }
    let deadline_ms = now_ms
        .checked_add(duration.as_millis() as i64)
        .ok_or(CampaignError::Invalid("échéance hors limites"))?;
    let mut parameters = Vec::new();
    for platform in platforms {
        let params = RunParams {
            platform_id: platform.clone(),
            ..template.clone()
        };
        params.validate().map_err(StorageError::from)?;
        parameters.push(params);
    }
    let (window_start, window_end) = collection_window(now_ms, template.window_days);
    let mut connection = storage.transaction_connection().await?;
    let mut tx = connection.begin().await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO collection_campaigns
        (created_at, deadline_at, status, params)
        VALUES (to_timestamp($1::float8 / 1000), to_timestamp($2::float8 / 1000), 'ready', $3)
        RETURNING id",
    )
    .bind(now_ms)
    .bind(deadline_ms)
    .bind(serde_json::json!({"platforms":platforms, "template":template}))
    .fetch_one(&mut *tx)
    .await?;
    for (ordinal, params) in parameters.iter().enumerate() {
        let run_id = Storage::create_run_in(&mut tx, params, window_start, window_end).await?;
        sqlx::query("INSERT INTO campaign_runs (campaign_id, run_id, ordinal) VALUES ($1, $2, $3)")
            .bind(id)
            .bind(run_id)
            .bind(ordinal as i32)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(id)
}

/// Détient le verrou global du collecteur et partage un seul gouverneur entre les runs.
/// La tranche peut être raccourcie par le lanceur ; elle ne dépasse jamais 15 minutes.
pub async fn execute<T: Transport>(
    storage: &Storage,
    transport: T,
    mut runtime: RuntimeOptions,
    campaign_id: i64,
    shutdown: impl Future<Output = ()>,
) -> Result<CampaignOutcome, CampaignError> {
    let slice = runtime.max_duration.unwrap_or(MAX_SLICE).min(MAX_SLICE);
    if slice.is_zero() {
        return Err(CampaignError::Invalid("tranche nulle"));
    }
    runtime.max_duration = Some(slice);
    let _lock = storage.lock_collector().await?;
    let row = sqlx::query(
        "SELECT status, next_ordinal,
        round(extract(epoch FROM deadline_at) * 1000)::bigint AS deadline_ms
        FROM collection_campaigns WHERE id=$1",
    )
    .bind(campaign_id)
    .fetch_optional(storage.pool())
    .await?
    .ok_or(CampaignError::NotFound(campaign_id))?;
    let runs: Vec<i64> = sqlx::query_scalar(
        "SELECT run_id FROM campaign_runs WHERE campaign_id=$1 ORDER BY ordinal",
    )
    .bind(campaign_id)
    .fetch_all(storage.pool())
    .await?;
    if runs.is_empty() {
        return Err(CampaignError::Invalid("campagne sans exécution"));
    }
    let status: String = row.try_get("status")?;
    if status == "finished" {
        return finish(
            storage,
            campaign_id,
            &runs,
            CampaignStatus::Finished,
            "finished",
        )
        .await;
    }
    let deadline_ms: i64 = row.try_get("deadline_ms")?;
    if deadline_ms <= now_ms() {
        return finish(
            storage,
            campaign_id,
            &runs,
            CampaignStatus::Paused,
            "deadline_reached",
        )
        .await;
    }
    let deadline = Instant::now() + Duration::from_millis((deadline_ms - now_ms()).max(0) as u64);
    let ordinal: i32 = row.try_get("next_ordinal")?;
    sqlx::query("UPDATE collection_campaigns SET status='running', status_reason=NULL, updated_at=now() WHERE id=$1")
        .bind(campaign_id).execute(storage.pool()).await?;
    let collector = Collector::new(storage.clone(), transport, runtime);
    let result = drive(
        storage,
        &collector,
        campaign_id,
        &runs,
        ordinal as usize,
        deadline,
        shutdown,
    )
    .await;
    if result.is_err() {
        let _ = sqlx::query("UPDATE collection_campaigns SET status='paused', status_reason='error', updated_at=now() WHERE id=$1")
            .bind(campaign_id).execute(storage.pool()).await;
    }
    result
}

async fn drive<T: Transport>(
    storage: &Storage,
    collector: &Arc<Collector<T>>,
    campaign_id: i64,
    runs: &[i64],
    mut ordinal: usize,
    deadline: Instant,
    shutdown: impl Future<Output = ()>,
) -> Result<CampaignOutcome, CampaignError> {
    tokio::pin!(shutdown);
    loop {
        let mut finished_runs = 0;
        let mut executed = false;
        // Une rotation complète évite de revenir en boucle sur un budget épuisé.
        for _ in 0..runs.len() {
            tokio::select! {
                biased;
                _ = &mut shutdown => return finish(storage, campaign_id, runs, CampaignStatus::Paused, "interrupted").await,
                _ = std::future::ready(()) => {}
            }
            if Instant::now() >= deadline {
                return finish(
                    storage,
                    campaign_id,
                    runs,
                    CampaignStatus::Paused,
                    "deadline_reached",
                )
                .await;
            }
            let index = ordinal % runs.len();
            ordinal = (index + 1) % runs.len();
            let run = storage.load_run(runs[index]).await?;
            if matches!(run.status, RunStatus::Completed | RunStatus::Incomplete) {
                finished_runs += 1;
                continue;
            }
            if u64::try_from(run.calls_made).unwrap_or(0) >= run.params.call_budget {
                continue;
            }
            executed = true;
            let mut stopped_for_deadline = false;
            let stop = async {
                tokio::select! {
                    biased;
                    _ = &mut shutdown => {},
                    _ = tokio::time::sleep_until(deadline) => { stopped_for_deadline = true; }
                }
            };
            // La fermeture du futur annule aussi son JoinSet. Les travaux dont la
            // réponse n'a pas été validée restent rejouables, comme après un crash.
            let outcome = match tokio::time::timeout_at(
                deadline + MAX_DRAIN,
                collector.execute(run.id, stop),
            )
            .await
            {
                Ok(result) => result?,
                Err(_) => {
                    storage.reset_interrupted(run.id).await?;
                    storage
                        .set_run_status(run.id, RunStatus::Paused, Some("deadline_reached"))
                        .await?;
                    return finish(
                        storage,
                        campaign_id,
                        runs,
                        CampaignStatus::Paused,
                        "deadline_reached",
                    )
                    .await;
                }
            };
            sqlx::query(
                "UPDATE collection_campaigns SET next_ordinal=$2, updated_at=now() WHERE id=$1",
            )
            .bind(campaign_id)
            .bind(ordinal as i32)
            .execute(storage.pool())
            .await?;
            match outcome.reason {
                StopReason::AuthRejected(_) => {
                    return finish(
                        storage,
                        campaign_id,
                        runs,
                        CampaignStatus::Paused,
                        "riot_auth_rejected",
                    )
                    .await
                }
                StopReason::Interrupted => {
                    return finish(
                        storage,
                        campaign_id,
                        runs,
                        CampaignStatus::Paused,
                        if stopped_for_deadline {
                            "deadline_reached"
                        } else {
                            "interrupted"
                        },
                    )
                    .await
                }
                _ => {}
            }
            if matches!(outcome.status, RunStatus::Completed | RunStatus::Incomplete) {
                finished_runs += 1;
            }
        }
        if finished_runs == runs.len() {
            return finish(
                storage,
                campaign_id,
                runs,
                CampaignStatus::Finished,
                "finished",
            )
            .await;
        }
        if !executed {
            return finish(
                storage,
                campaign_id,
                runs,
                CampaignStatus::Paused,
                "call_budgets_exhausted",
            )
            .await;
        }
    }
}

async fn finish(
    storage: &Storage,
    campaign_id: i64,
    runs: &[i64],
    status: CampaignStatus,
    reason: &str,
) -> Result<CampaignOutcome, CampaignError> {
    sqlx::query(
        "UPDATE collection_campaigns SET status=$2, status_reason=$3, updated_at=now() WHERE id=$1",
    )
    .bind(campaign_id)
    .bind(status.as_str())
    .bind(reason)
    .execute(storage.pool())
    .await?;
    let completed: i64 = sqlx::query_scalar("SELECT count(*) FROM collection_runs r
        JOIN campaign_runs c ON c.run_id=r.id WHERE c.campaign_id=$1 AND r.status IN ('completed','incomplete')")
        .bind(campaign_id).fetch_one(storage.pool()).await?;
    Ok(CampaignOutcome {
        campaign_id,
        status,
        reason: reason.into(),
        completed_runs: completed as usize,
        total_runs: runs.len(),
    })
}
