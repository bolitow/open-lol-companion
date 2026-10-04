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
use crate::config::{queue_needs_ranks, RunParams, RuntimeOptions};
use crate::queues;
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

/// Files visées par défaut par une campagne par file : ARAM, Swiftplay et Arena.
/// Arena est demandée sous son identifiant 1700 ; les variantes 1740 et 1750 observées
/// en recette sont des files distinctes, à ajouter explicitement.
pub const DEFAULT_QUEUES: [i32; 3] = [450, 480, 1700];

/// Une campagne par file ne vise que des files identifiées (voir `crate::queues`) :
/// un identifiant inconnu serait collecté puis exclu des agrégats, au prix de quotas Riot.
pub fn validate_queues(queues: &[i32]) -> Result<(), CampaignError> {
    if queues.is_empty() {
        return Err(CampaignError::Invalid("aucune file demandée"));
    }
    if queues.iter().collect::<BTreeSet<_>>().len() != queues.len() {
        return Err(CampaignError::Invalid("files dupliquées"));
    }
    if queues.iter().any(|q| !queues::is_identified(*q)) {
        return Err(CampaignError::Invalid("file inconnue ou nulle"));
    }
    Ok(())
}

/// Les observations de rang n'alimentent que les files classées (420, 440) : les autres
/// files sont agrégées en `UNRANKED_MODE`. Les éviter économise un appel par joueur.
pub fn needs_rank_observations(queues: &[i32]) -> bool {
    queues.iter().any(|q| [420, 440].contains(q))
}

/// Crée toutes les exécutions dans une transaction, avec le même instant de fin.
/// Les choix de volumes et de patches sont fournis par le lanceur et restent figés.
/// Une exécution par plateforme, dans la file du modèle.
pub async fn start(
    storage: &Storage,
    platforms: &[String],
    template: &RunParams,
    now_ms: i64,
    duration: Duration,
) -> Result<i64, CampaignError> {
    check_platforms_and_duration(platforms, duration)?;
    let mut parameters = Vec::new();
    for platform in platforms {
        parameters.push(RunParams {
            platform_id: platform.clone(),
            ..template.clone()
        });
    }
    create(
        storage,
        platforms,
        &[template.queue_id],
        template,
        parameters,
        now_ms,
        duration,
    )
    .await
}

/// Comme [`start`], avec une exécution par plateforme **et** par file de `queues`
/// (voir [`run_params_for_queues`]). Les files par défaut sont
/// [`crate::config::DEFAULT_CAMPAIGN_QUEUES`] (décision du 4 octobre 2026, #90).
pub async fn start_for_queues(
    storage: &Storage,
    platforms: &[String],
    queues: &[i32],
    template: &RunParams,
    now_ms: i64,
    duration: Duration,
) -> Result<i64, CampaignError> {
    check_platforms_and_duration(platforms, duration)?;
    let parameters = run_params_for_queues(platforms, queues, template)?;
    // Le gabarit est conservé tel quel dans `params->'template'` à titre de trace : ses
    // champs `queue_id` et `collect_ranks` y sont ignorés, les files réelles sont dans
    // `params->'queues'` et dans les paramètres de chaque exécution.
    create(
        storage, platforms, queues, template, parameters, now_ms, duration,
    )
    .await
}

/// Comme [`start`], avec une exécution par couple (plateforme, file) parmi les files
/// identifiées ([`validate_queues`]) : la cible du modèle et son budget d'appels
/// s'appliquent à **chaque** exécution, sans répartition (#97, commande `campaign-queues`).
/// L'ordre de rotation est plateforme puis file, pour qu'une campagne écourtée couvre
/// toutes les files des premières plateformes. Pour la répartition du budget d'une
/// plateforme entre ses files, voir [`start_for_queues`].
pub async fn start_per_queue(
    storage: &Storage,
    platforms: &[String],
    queues: &[i32],
    template: &RunParams,
    now_ms: i64,
    duration: Duration,
) -> Result<i64, CampaignError> {
    validate_queues(queues)?;
    check_platforms_and_duration(platforms, duration)?;
    let mut parameters = Vec::with_capacity(platforms.len() * queues.len());
    for platform in platforms {
        for &queue_id in queues {
            parameters.push(RunParams {
                platform_id: platform.clone(),
                queue_id,
                ..template.clone()
            });
        }
    }
    create(
        storage, platforms, queues, template, parameters, now_ms, duration,
    )
    .await
}

fn check_platforms_and_duration(
    platforms: &[String],
    duration: Duration,
) -> Result<(), CampaignError> {
    if platforms.is_empty() || platforms.iter().collect::<BTreeSet<_>>().len() != platforms.len() {
        return Err(CampaignError::Invalid("plateformes absentes ou dupliquées"));
    }
    if duration.as_millis() == 0 || duration > MAX_DURATION {
        return Err(CampaignError::Invalid(
            "durée attendue : de 1 ms à 24 heures",
        ));
    }
    Ok(())
}

/// Paramètres de chaque exécution : plateforme par plateforme, puis file par file.
///
/// Riot filtre l'historique sur une seule file : plusieurs files demandent donc plusieurs
/// exécutions. Le budget d'appels et la cible de parties de `template` valent pour une
/// plateforme ; ils sont répartis également entre ses files (budget arrondi à l'inférieur,
/// cible à la supérieure), pour que le plafond par plateforme reste celui annoncé. Les
/// rangs ne sont observés que pour les files qui les exploitent. `0` (toutes les files)
/// ne se combine avec aucune autre file.
pub fn run_params_for_queues(
    platforms: &[String],
    queues: &[i32],
    template: &RunParams,
) -> Result<Vec<RunParams>, CampaignError> {
    if queues.is_empty() || queues.iter().collect::<BTreeSet<_>>().len() != queues.len() {
        return Err(CampaignError::Invalid("files absentes ou dupliquées"));
    }
    if queues.len() > 1 && queues.contains(&0) {
        return Err(CampaignError::Invalid(
            "la file 0 (toutes les files) ne se combine pas avec d'autres files",
        ));
    }
    let count = queues.len() as u64;
    if template.call_budget < count {
        return Err(CampaignError::Invalid(
            "budget d'appels par plateforme inférieur au nombre de files",
        ));
    }
    let call_budget = template.call_budget / count;
    let target_matches = u32::try_from(u64::from(template.target_matches).div_ceil(count))
        .unwrap_or(u32::MAX)
        .max(1);
    let mut runs = Vec::with_capacity(platforms.len() * queues.len());
    for platform in platforms {
        for &queue_id in queues {
            let params = RunParams {
                platform_id: platform.clone(),
                queue_id,
                collect_ranks: queue_needs_ranks(queue_id),
                call_budget,
                target_matches,
                ..template.clone()
            };
            params.validate().map_err(StorageError::from)?;
            runs.push(params);
        }
    }
    Ok(runs)
}

async fn create(
    storage: &Storage,
    platforms: &[String],
    queues: &[i32],
    template: &RunParams,
    parameters: Vec<RunParams>,
    now_ms: i64,
    duration: Duration,
) -> Result<i64, CampaignError> {
    let deadline_ms = now_ms
        .checked_add(duration.as_millis() as i64)
        .ok_or(CampaignError::Invalid("échéance hors limites"))?;
    for params in &parameters {
        params.validate().map_err(StorageError::from)?;
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
    .bind(serde_json::json!({"platforms":platforms, "queues":queues, "template":template}))
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

/// Avancement d'une exécution (une plateforme et une file) face à sa cible.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QueueCoverage {
    pub run_id: i64,
    pub platform_id: String,
    /// 0 : toutes les files (campagne historique).
    pub queue_id: i32,
    pub target: i64,
    /// Parties retenues, déjà présentes en base comprises.
    pub retained: i64,
    pub calls_made: i64,
    pub status: String,
    pub status_reason: Option<String>,
    /// `true` seulement si la cible est atteinte.
    pub complete: bool,
}

/// Bilan d'une campagne : une ligne par plateforme et par file, dans l'ordre de rotation.
/// Une file absente ou sous sa cible signifie données non acquises, pas absence d'activité.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CampaignCoverage {
    pub campaign_id: i64,
    pub status: String,
    pub status_reason: Option<String>,
    pub runs: Vec<QueueCoverage>,
}

impl CampaignCoverage {
    /// Tableau lisible : cible et parties retenues par plateforme et par file.
    pub fn render(&self) -> String {
        let mut out = format!(
            "Campagne {} : {}{}\n",
            self.campaign_id,
            self.status,
            self.status_reason
                .as_deref()
                .map(|r| format!(" ({r})"))
                .unwrap_or_default()
        );
        for run in &self.runs {
            let queue = if run.queue_id == 0 {
                "toutes".to_owned()
            } else {
                run.queue_id.to_string()
            };
            out.push_str(&format!(
                "  {:<5} file {:<6} {:>6} / {:<6} parties, {} appels, {}{}\n",
                run.platform_id,
                queue,
                run.retained,
                run.target,
                run.calls_made,
                run.status,
                if run.complete { ", cible atteinte" } else { "" }
            ));
        }
        out
    }
}

/// Lit l'avancement de chaque exécution depuis la base (exact après une reprise).
pub async fn coverage(
    storage: &Storage,
    campaign_id: i64,
) -> Result<CampaignCoverage, CampaignError> {
    let head = sqlx::query("SELECT status, status_reason FROM collection_campaigns WHERE id=$1")
        .bind(campaign_id)
        .fetch_optional(storage.pool())
        .await?
        .ok_or(CampaignError::NotFound(campaign_id))?;
    let rows = sqlx::query(
        "SELECT r.id, r.platform_id, r.queue_id, r.target_matches, r.calls_made, r.status,
                r.status_reason,
                (SELECT count(*) FROM run_matches m WHERE m.run_id = r.id) AS retained
         FROM campaign_runs c JOIN collection_runs r ON r.id = c.run_id
         WHERE c.campaign_id = $1 ORDER BY c.ordinal",
    )
    .bind(campaign_id)
    .fetch_all(storage.pool())
    .await?;
    let mut runs = Vec::with_capacity(rows.len());
    for row in rows {
        let target = i64::from(row.try_get::<i32, _>("target_matches")?);
        let retained: i64 = row.try_get("retained")?;
        runs.push(QueueCoverage {
            run_id: row.try_get("id")?,
            platform_id: row.try_get("platform_id")?,
            queue_id: row.try_get("queue_id")?,
            target,
            retained,
            calls_made: row.try_get("calls_made")?,
            status: row.try_get("status")?,
            status_reason: row.try_get("status_reason")?,
            complete: retained >= target,
        });
    }
    Ok(CampaignCoverage {
        campaign_id,
        status: head.try_get("status")?,
        status_reason: head.try_get("status_reason")?,
        runs,
    })
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
    let lock = storage.lock_collector().await?;
    // Une fermeture de socket seule ne garantit pas que PostgreSQL a déjà libéré
    // le verrou avant une reprise immédiate. Attendre l’unlock sur chaque retour,
    // y compris les échéances dépassées et les erreurs de lecture.
    let outcome = async {
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
    .await;
    lock.release().await;
    outcome
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_CAMPAIGN_QUEUES;

    fn platforms() -> Vec<String> {
        vec!["EUW1".into(), "NA1".into()]
    }

    fn template() -> RunParams {
        RunParams {
            target_matches: 1001,
            call_budget: 1000,
            ..RunParams::default()
        }
    }

    #[test]
    fn les_files_par_defaut_sont_solo_et_flex() {
        assert_eq!(DEFAULT_CAMPAIGN_QUEUES, [420, 440]);
    }

    #[test]
    fn une_execution_par_plateforme_et_par_file_avec_le_budget_reparti() {
        let runs =
            run_params_for_queues(&platforms(), &DEFAULT_CAMPAIGN_QUEUES, &template()).unwrap();
        let scopes: Vec<_> = runs
            .iter()
            .map(|r| (r.platform_id.as_str(), r.queue_id))
            .collect();
        assert_eq!(
            scopes,
            [("EUW1", 420), ("EUW1", 440), ("NA1", 420), ("NA1", 440)]
        );
        // Le budget et la cible de chaque plateforme sont répartis entre ses files.
        assert!(runs.iter().all(|r| r.call_budget == 500));
        assert!(runs.iter().all(|r| r.target_matches == 501));
        assert!(runs.iter().all(|r| r.collect_ranks));
    }

    #[test]
    fn une_file_explicite_hors_solo_et_flex_est_acceptee_sans_rangs() {
        let runs = run_params_for_queues(&platforms(), &[450], &template()).unwrap();
        assert_eq!(runs.len(), 2);
        assert!(runs.iter().all(|r| r.queue_id == 450 && !r.collect_ranks));
        // Une seule file : budget et cible de la plateforme restent entiers.
        assert!(runs
            .iter()
            .all(|r| r.call_budget == 1000 && r.target_matches == 1001));
        // Plusieurs files explicites, classées ou non : les rangs suivent chaque file.
        let mixed = run_params_for_queues(&platforms(), &[420, 450, 1700], &template()).unwrap();
        let ranks: Vec<_> = mixed.iter().take(3).map(|r| r.collect_ranks).collect();
        assert_eq!(ranks, [true, false, false]);
    }

    #[test]
    fn les_listes_de_files_incoherentes_sont_refusees() {
        for queues in [&[][..], &[420, 420], &[0, 420], &[-1]] {
            assert!(
                run_params_for_queues(&platforms(), queues, &template()).is_err(),
                "{queues:?}"
            );
        }
        // 0 seul = toutes les files, comportement historique conservé.
        let all = run_params_for_queues(&platforms(), &[0], &template()).unwrap();
        assert!(all.iter().all(|r| r.queue_id == 0 && r.collect_ranks));
    }

    #[test]
    fn un_budget_inferieur_au_nombre_de_files_est_refuse() {
        // Sinon chaque file recevrait au moins 1 appel et la plateforme dépasserait son plafond.
        let small = RunParams {
            call_budget: 1,
            ..template()
        };
        assert!(run_params_for_queues(&platforms(), &DEFAULT_CAMPAIGN_QUEUES, &small).is_err());
        let exact = RunParams {
            call_budget: 2,
            ..template()
        };
        let runs = run_params_for_queues(&platforms(), &DEFAULT_CAMPAIGN_QUEUES, &exact).unwrap();
        assert!(runs.iter().all(|r| r.call_budget == 1));
    }

    #[test]
    fn les_files_par_defaut_sont_aram_swiftplay_et_arena_et_sont_valides() {
        assert_eq!(DEFAULT_QUEUES, [450, 480, 1700]);
        assert!(validate_queues(&DEFAULT_QUEUES).is_ok());
        assert!(validate_queues(&[420, 440, 1740, 1750]).is_ok());
    }

    #[test]
    fn une_liste_de_files_vide_dupliquee_nulle_ou_inconnue_est_refusee() {
        for queues in [&[][..], &[450, 450], &[0], &[-1], &[450, 710], &[3130]] {
            assert!(validate_queues(queues).is_err(), "{queues:?}");
        }
    }

    #[test]
    fn seules_les_files_classees_demandent_les_observations_de_rang() {
        assert!(!needs_rank_observations(&DEFAULT_QUEUES));
        assert!(needs_rank_observations(&[450, 440]));
        assert!(needs_rank_observations(&[420]));
        assert!(!needs_rank_observations(&[]));
    }
}
