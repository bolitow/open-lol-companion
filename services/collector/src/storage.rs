//! Persistance PostgreSQL : migrations, verrou du collecteur, file de travaux
//! reprenable et écriture transactionnelle des résultats.
//!
//! Chaque résultat est enregistré dans la même transaction que l'avancement de son
//! travail et que le compteur d'appels de l'exécution.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::{PgConnection, PgPool, PgPoolOptions};
use sqlx::{Connection, Postgres, Row, Transaction};
use thiserror::Error;

use crate::config::{ConfigError, Division, RunParams, Tier};
use crate::model::{Exclusion, LeagueEntry, MatchFacts, ParticipantRank, Scope, RANKED_QUEUE_IDS};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Clé du verrou consultatif : un seul collecteur actif par base.
const COLLECTOR_LOCK_KEY: i64 = 0x0017_C011_EC70;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("PostgreSQL : {0}")]
    Db(#[from] sqlx::Error),
    #[error("migrations PostgreSQL : {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("un autre collecteur utilise déjà cette base")]
    Busy,
    #[error("exécution {0} introuvable")]
    RunNotFound(i64),
    #[error("donnée enregistrée illisible : {0}")]
    Corrupt(String),
}

/// Étape d'une collecte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobKind {
    SeedPage,
    MatchIds,
    Match,
    Timeline,
    ParticipantRank,
}

impl JobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            JobKind::SeedPage => "seed_page",
            JobKind::MatchIds => "match_ids",
            JobKind::Match => "match",
            JobKind::Timeline => "timeline",
            JobKind::ParticipantRank => "participant_rank",
        }
    }

    fn parse(s: &str) -> Result<Self, StorageError> {
        match s {
            "seed_page" => Ok(JobKind::SeedPage),
            "match_ids" => Ok(JobKind::MatchIds),
            "match" => Ok(JobKind::Match),
            "timeline" => Ok(JobKind::Timeline),
            "participant_rank" => Ok(JobKind::ParticipantRank),
            other => Err(StorageError::Corrupt(format!("type de travail {other}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedPagePayload {
    pub tier: Tier,
    pub division: Division,
    pub page: u32,
    /// Position de la strate (rang, division) dans l'ordre de parcours.
    pub stratum: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchIdsPayload {
    pub puuid: String,
    pub start: u32,
    /// Ordre du joueur : alterne les strates pour équilibrer l'échantillon.
    pub seed_sort: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchPayload {
    pub match_id: String,
    pub seed_puuid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelinePayload {
    pub match_id: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantRankPayload {
    pub platform_id: String,
    pub puuid: String,
}

/// Travail réservé par le collecteur (état `running`).
#[derive(Debug, Clone)]
pub struct Job {
    pub id: i64,
    pub run_id: i64,
    pub kind: JobKind,
    pub payload: Value,
    pub attempts: i32,
    pub not_found_count: i32,
    pub sort_key: i64,
}

impl Job {
    pub fn payload<T: for<'de> Deserialize<'de>>(&self) -> Result<T, StorageError> {
        serde_json::from_value(self.payload.clone())
            .map_err(|e| StorageError::Corrupt(format!("travail {} : {e}", self.id)))
    }
}

/// État d'une exécution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Running,
    Paused,
    Completed,
    Incomplete,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Paused => "paused",
            RunStatus::Completed => "completed",
            RunStatus::Incomplete => "incomplete",
        }
    }

    fn parse(s: &str) -> Result<Self, StorageError> {
        match s {
            "running" => Ok(RunStatus::Running),
            "paused" => Ok(RunStatus::Paused),
            "completed" => Ok(RunStatus::Completed),
            "incomplete" => Ok(RunStatus::Incomplete),
            other => Err(StorageError::Corrupt(format!("statut d'exécution {other}"))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunRecord {
    pub id: i64,
    pub status: RunStatus,
    pub params: RunParams,
    pub scope: Scope,
    pub calls_made: i64,
}

/// Verrou consultatif détenu sur une connexion dédiée ; relâché à la fermeture de
/// celle-ci (fin du processus comprise).
pub struct CollectorLock {
    conn: PgConnection,
}

impl CollectorLock {
    /// Relâche le verrou immédiatement puis ferme la connexion.
    pub async fn release(mut self) {
        let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(COLLECTOR_LOCK_KEY)
            .execute(&mut self.conn)
            .await;
        let _ = self.conn.close().await;
    }
}

#[derive(Clone)]
pub struct Storage {
    pool: PgPool,
}

impl Storage {
    pub async fn connect(url: &str, max_connections: u32) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(10))
            .connect(url)
            .await?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub(crate) async fn transaction_connection(
        &self,
    ) -> Result<crate::transaction::TransactionConnection, sqlx::Error> {
        crate::transaction::TransactionConnection::acquire(&self.pool).await
    }

    pub async fn migrate(&self) -> Result<(), StorageError> {
        MIGRATOR.run(&self.pool).await?;
        Ok(())
    }

    /// Garantit qu'un seul collecteur travaille sur la base.
    pub async fn lock_collector(&self) -> Result<CollectorLock, StorageError> {
        let mut conn = self.pool.acquire().await?.detach();
        let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
            .bind(COLLECTOR_LOCK_KEY)
            .fetch_one(&mut conn)
            .await?;
        if locked {
            Ok(CollectorLock { conn })
        } else {
            let _ = conn.close().await;
            Err(StorageError::Busy)
        }
    }

    /// Crée une exécution et ses premiers travaux (page 1 de chaque strate).
    pub async fn create_run(
        &self,
        params: &RunParams,
        window_start_ms: i64,
        window_end_ms: i64,
    ) -> Result<i64, StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        let run_id = Self::create_run_in(&mut tx, params, window_start_ms, window_end_ms).await?;
        tx.commit().await?;
        Ok(run_id)
    }

    /// Partage la transaction de création d'une campagne pour éviter les runs orphelins.
    pub(crate) async fn create_run_in(
        tx: &mut Transaction<'_, Postgres>,
        params: &RunParams,
        window_start_ms: i64,
        window_end_ms: i64,
    ) -> Result<i64, StorageError> {
        let params_json =
            serde_json::to_value(params).map_err(|e| StorageError::Corrupt(e.to_string()))?;
        let run_id: i64 = sqlx::query_scalar(
            "INSERT INTO collection_runs
                (status, platform_id, queue_id, window_start, window_end, target_matches, params)
             VALUES ('running', $1, $2, to_timestamp($3::float8 / 1000), to_timestamp($4::float8 / 1000), $5, $6)
             RETURNING id",
        )
        .bind(&params.platform_id)
        .bind(params.queue_id)
        .bind(window_start_ms)
        .bind(window_end_ms)
        .bind(params.target_matches as i32)
        .bind(params_json)
        .fetch_one(&mut **tx)
        .await?;
        let strata_count = params.strata().len() as i64;
        for (stratum, (tier, division)) in params.strata().into_iter().enumerate() {
            let payload = SeedPagePayload {
                tier,
                division,
                page: 1,
                stratum: stratum as u32,
            };
            insert_job(
                tx,
                run_id,
                JobKind::SeedPage,
                &seed_page_key(&payload),
                &payload,
                seed_page_sort(1, stratum as i64, strata_count),
            )
            .await?;
        }
        Ok(run_id)
    }

    pub async fn load_run(&self, run_id: i64) -> Result<RunRecord, StorageError> {
        let row = sqlx::query(
            "SELECT id, status, params, platform_id, queue_id, calls_made,
                    round(extract(epoch FROM window_start) * 1000)::bigint AS start_ms,
                    round(extract(epoch FROM window_end) * 1000)::bigint AS end_ms
             FROM collection_runs WHERE id = $1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::RunNotFound(run_id))?;
        let params: RunParams = serde_json::from_value(row.try_get("params")?)
            .map_err(|e| StorageError::Corrupt(format!("paramètres : {e}")))?;
        Ok(RunRecord {
            id: row.try_get("id")?,
            status: RunStatus::parse(row.try_get("status")?)?,
            params: params.clone(),
            scope: Scope {
                platform_id: row.try_get("platform_id")?,
                queue_id: row.try_get("queue_id")?,
                patches: params.patches.clone(),
                window_start_ms: row.try_get("start_ms")?,
                window_end_ms: row.try_get("end_ms")?,
            },
            calls_made: row.try_get("calls_made")?,
        })
    }

    /// Relève le budget d'appels d'une exécution (reprise après épuisement).
    pub async fn set_call_budget(&self, run_id: i64, budget: u64) -> Result<(), StorageError> {
        sqlx::query(
            "UPDATE collection_runs
             SET params = jsonb_set(params, '{call_budget}', to_jsonb($2::bigint)), updated_at = now()
             WHERE id = $1",
        )
        .bind(run_id)
        .bind(budget as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_run_status(
        &self,
        run_id: i64,
        status: RunStatus,
        reason: Option<&str>,
    ) -> Result<(), StorageError> {
        let finished = matches!(status, RunStatus::Completed | RunStatus::Incomplete);
        sqlx::query(
            "UPDATE collection_runs
             SET status = $2, status_reason = $3, updated_at = now(),
                 finished_at = CASE WHEN $4 THEN now() ELSE NULL END
             WHERE id = $1",
        )
        .bind(run_id)
        .bind(status.as_str())
        .bind(reason)
        .bind(finished)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_report(&self, run_id: i64, report: &Value) -> Result<(), StorageError> {
        sqlx::query("UPDATE collection_runs SET report = $2, updated_at = now() WHERE id = $1")
            .bind(run_id)
            .bind(report)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Remet en attente les travaux restés `running` après un arrêt brutal. Sûr car
    /// le verrou garantit qu'aucun autre collecteur ne les traite.
    pub async fn reset_interrupted(&self, run_id: i64) -> Result<u64, StorageError> {
        let res = sqlx::query(
            "UPDATE collection_jobs SET state = 'pending', updated_at = now()
             WHERE run_id = $1 AND state = 'running'",
        )
        .bind(run_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }

    /// Remet en attente les travaux en échec (après une panne réseau, par exemple)
    /// et rouvre l'exécution. Renvoie le nombre de travaux relancés.
    pub async fn requeue_failed(&self, run_id: i64) -> Result<u64, StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        let res = sqlx::query(
            "UPDATE collection_jobs
             SET state = 'pending', attempts = 0, not_found_count = 0, outcome = NULL,
                 next_attempt_at = now(), updated_at = now()
             WHERE run_id = $1 AND state = 'failed'",
        )
        .bind(run_id)
        .execute(&mut *tx)
        .await?;
        if res.rows_affected() > 0 {
            sqlx::query(
                "UPDATE collection_runs
                 SET status = 'paused', status_reason = 'failed_jobs_requeued', finished_at = NULL,
                     updated_at = now()
                 WHERE id = $1",
            )
            .bind(run_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(res.rows_affected())
    }

    pub async fn calls_made(&self, run_id: i64) -> Result<i64, StorageError> {
        Ok(
            sqlx::query_scalar("SELECT calls_made FROM collection_runs WHERE id = $1")
                .bind(run_id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// Réserve le prochain travail exécutable. Les détails de parties passent avant les
    /// rangs : un rang n'a d'intérêt que pour une partie déjà retenue, et il ne doit pas
    /// consommer le budget d'appels qui servirait à télécharger davantage de parties (#90).
    /// Les détails ne sont plus réservés une fois la cible atteinte (parties retenues +
    /// détails en cours), ni tant qu'une découverte est en attente ou en cours : l'ordre
    /// d'alternance entre joueurs ne dépend ainsi pas du moment où chaque historique
    /// arrive. Une découverte en attente de nouvelle tentative ne bloque pas les détails.
    pub async fn claim_next(&self, run_id: i64, target: i64) -> Result<Option<Job>, StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        let row = sqlx::query(
            "WITH counts AS (
                 SELECT (SELECT count(*) FROM run_matches WHERE run_id = $1)
                      + (SELECT count(*) FROM collection_jobs
                         WHERE run_id = $1 AND kind = 'match' AND state = 'running') AS engaged
             )
             SELECT j.id, j.run_id, j.kind, j.payload, j.attempts, j.not_found_count, j.sort_key
             FROM collection_jobs j, counts
             WHERE j.run_id = $1
               AND j.state IN ('pending', 'retry_wait')
               AND j.next_attempt_at <= now()
               AND (j.kind <> 'match' OR (
                        counts.engaged < $2
                        AND NOT EXISTS (
                            SELECT 1 FROM collection_jobs d
                            WHERE d.run_id = $1 AND d.kind IN ('seed_page', 'match_ids')
                              AND d.state IN ('pending', 'running'))))
             ORDER BY CASE j.kind WHEN 'seed_page' THEN 0 WHEN 'match_ids' THEN 1
                                  WHEN 'timeline' THEN 2 WHEN 'participant_rank' THEN 4 ELSE 3 END,
                      j.sort_key, j.id
             LIMIT 1
             FOR UPDATE OF j SKIP LOCKED",
        )
        .bind(run_id)
        .bind(target)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        let job = Job {
            id: row.try_get("id")?,
            run_id: row.try_get("run_id")?,
            kind: JobKind::parse(row.try_get("kind")?)?,
            payload: row.try_get("payload")?,
            attempts: row.try_get("attempts")?,
            not_found_count: row.try_get("not_found_count")?,
            sort_key: row.try_get("sort_key")?,
        };
        sqlx::query(
            "UPDATE collection_jobs SET state = 'running', updated_at = now() WHERE id = $1",
        )
        .bind(job.id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(job))
    }

    /// Délai avant le prochain travail exécutable (négatif = déjà dû) ; `None` s'il
    /// ne reste rien à faire pour cette exécution.
    pub async fn next_due_in(&self, run_id: i64, target: i64) -> Result<Option<f64>, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT extract(epoch FROM (min(next_attempt_at) - now()))::float8
             FROM collection_jobs
             WHERE run_id = $1 AND state IN ('pending', 'retry_wait')
               AND (kind <> 'match' OR (SELECT count(*) FROM run_matches WHERE run_id = $1) < $2)",
        )
        .bind(run_id)
        .bind(target)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn retained_count(&self, run_id: i64) -> Result<i64, StorageError> {
        Ok(
            sqlx::query_scalar("SELECT count(*) FROM run_matches WHERE run_id = $1")
                .bind(run_id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// Enregistre les joueurs d'une page de classement et crée leurs travaux
    /// d'historique ; ajoute la page suivante si la strate n'est pas complète.
    pub async fn complete_seed_page(
        &self,
        job: &Job,
        payload: &SeedPagePayload,
        entries: &[LeagueEntry],
        params: &RunParams,
    ) -> Result<u32, StorageError> {
        let strata_count = params.strata().len() as i64;
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        let existing: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM seed_players WHERE run_id = $1 AND tier = $2 AND division = $3",
        )
        .bind(job.run_id)
        .bind(payload.tier.as_str())
        .bind(payload.division.as_str())
        .fetch_one(&mut *tx)
        .await?;
        let capacity = (i64::from(params.seeds_per_division) - existing).max(0);
        let mut inserted = 0i64;
        for entry in entries {
            if inserted >= capacity {
                break;
            }
            let Some(puuid) = entry.puuid.as_deref().filter(|p| !p.is_empty()) else {
                continue;
            };
            let seed_index = existing + inserted;
            let res = sqlx::query(
                "INSERT INTO seed_players
                    (run_id, puuid, platform_id, tier, division, league_points, source_page, seed_index)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 ON CONFLICT DO NOTHING",
            )
            .bind(job.run_id)
            .bind(puuid)
            .bind(&params.platform_id)
            .bind(payload.tier.as_str())
            .bind(payload.division.as_str())
            .bind(entry.league_points)
            .bind(payload.page as i32)
            .bind(seed_index as i32)
            .execute(&mut *tx)
            .await?;
            if res.rows_affected() == 0 {
                continue;
            }
            let seed_sort = seed_index * strata_count + i64::from(payload.stratum);
            let ids = MatchIdsPayload {
                puuid: puuid.to_owned(),
                start: 0,
                seed_sort,
            };
            insert_job(
                &mut tx,
                job.run_id,
                JobKind::MatchIds,
                &match_ids_key(&ids),
                &ids,
                seed_sort,
            )
            .await?;
            inserted += 1;
        }
        if !payload.tier.is_apex() && inserted < capacity && !entries.is_empty() {
            let next = SeedPagePayload {
                page: payload.page + 1,
                ..payload.clone()
            };
            insert_job(
                &mut tx,
                job.run_id,
                JobKind::SeedPage,
                &seed_page_key(&next),
                &next,
                seed_page_sort(next.page, i64::from(next.stratum), strata_count),
            )
            .await?;
        }
        finish(&mut tx, job, &format!("seeds:{inserted}"), 1).await?;
        tx.commit().await?;
        Ok(inserted as u32)
    }

    /// Enregistre les parties découvertes dans un historique. La contrainte unique
    /// `(run_id, kind, job_key)` garantit un seul travail de détail par partie.
    pub async fn complete_match_ids(
        &self,
        job: &Job,
        payload: &MatchIdsPayload,
        requested: u32,
        ids: &[String],
        scope: &Scope,
        params: &RunParams,
    ) -> Result<(), StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        for (i, match_id) in ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO run_discoveries (run_id, match_id, seed_puuid)
                 VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            )
            .bind(job.run_id)
            .bind(match_id)
            .bind(&payload.puuid)
            .execute(&mut *tx)
            .await?;
            // Alterne les joueurs : 1ʳᵉ partie de chacun, puis 2ᵉ, etc.
            let position = i64::from(payload.start) + i as i64;
            let sort_key = position * 1_000_000 + payload.seed_sort;
            let m = MatchPayload {
                match_id: match_id.clone(),
                seed_puuid: payload.puuid.clone(),
            };
            if scope.has_platform_prefix(match_id) {
                insert_job(&mut tx, job.run_id, JobKind::Match, match_id, &m, sort_key).await?;
            } else {
                // Partie d'une autre plateforme (transfert de compte) : exclue sans appel.
                sqlx::query(
                    "INSERT INTO collection_jobs
                        (run_id, kind, job_key, payload, sort_key, state, outcome)
                     VALUES ($1, 'match', $2, $3, $4, 'done', $5)
                     ON CONFLICT DO NOTHING",
                )
                .bind(job.run_id)
                .bind(match_id)
                .bind(to_json(&m)?)
                .bind(sort_key)
                .bind(Exclusion::WrongPlatform.outcome())
                .execute(&mut *tx)
                .await?;
            }
        }
        let next_start = payload.start + requested;
        if ids.len() as u32 >= requested && next_start < params.max_matches_per_seed {
            let next = MatchIdsPayload {
                start: next_start,
                ..payload.clone()
            };
            insert_job(
                &mut tx,
                job.run_id,
                JobKind::MatchIds,
                &match_ids_key(&next),
                &next,
                payload.seed_sort,
            )
            .await?;
        }
        finish(&mut tx, job, &format!("ids:{}", ids.len()), 1).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Colonnes d'une partie déjà en base.
    pub async fn find_match(&self, match_id: &str) -> Result<Option<MatchFacts>, StorageError> {
        self.find_facts("matches", match_id).await
    }

    /// Colonnes d'une partie déjà téléchargée puis exclue d'un périmètre (cache négatif).
    pub async fn find_excluded_match(
        &self,
        match_id: &str,
    ) -> Result<Option<MatchFacts>, StorageError> {
        self.find_facts("excluded_matches", match_id).await
    }

    /// `table` est une constante interne (`matches` ou `excluded_matches`), jamais une
    /// valeur externe : même structure de colonnes dans les deux tables.
    async fn find_facts(
        &self,
        table: &'static str,
        match_id: &str,
    ) -> Result<Option<MatchFacts>, StorageError> {
        let row = sqlx::query(&format!(
            "SELECT match_id, platform_id, queue_id, game_version, patch, game_duration_s,
                    is_remake, data_version,
                    round(extract(epoch FROM game_start) * 1000)::bigint AS start_ms
             FROM {table} WHERE match_id = $1"
        ))
        .bind(match_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| {
            Ok(MatchFacts {
                match_id: r.try_get("match_id")?,
                platform_id: r.try_get("platform_id")?,
                queue_id: r.try_get("queue_id")?,
                game_version: r.try_get("game_version")?,
                patch: r.try_get("patch")?,
                game_start_ms: r.try_get("start_ms")?,
                game_duration_s: r.try_get("game_duration_s")?,
                is_remake: r.try_get("is_remake")?,
                data_version: r.try_get("data_version")?,
            })
        })
        .transpose()
    }

    /// Retient une partie déjà en base, sans la retélécharger.
    pub async fn link_existing_match(
        &self,
        job: &Job,
        payload: &MatchPayload,
    ) -> Result<(), StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        link_match(&mut tx, job, payload, true).await?;
        finish(&mut tx, job, "already_present", 0).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Enregistre le détail d'une partie, la retient et crée son travail de timeline.
    pub async fn store_match(
        &self,
        job: &Job,
        payload: &MatchPayload,
        facts: &MatchFacts,
        detail: &Value,
    ) -> Result<(), StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        let inserted = sqlx::query(
            "INSERT INTO matches
                (match_id, platform_id, queue_id, game_version, patch, game_start,
                 game_duration_s, is_remake, data_version, detail, first_run_id)
             VALUES ($1, $2, $3, $4, $5, to_timestamp($6::float8 / 1000), $7, $8, $9, $10, $11)
             ON CONFLICT (match_id) DO NOTHING",
        )
        .bind(&facts.match_id)
        .bind(&facts.platform_id)
        .bind(facts.queue_id)
        .bind(&facts.game_version)
        .bind(&facts.patch)
        .bind(facts.game_start_ms)
        .bind(facts.game_duration_s)
        .bind(facts.is_remake)
        .bind(&facts.data_version)
        .bind(detail)
        .bind(job.run_id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            == 1;
        link_match(&mut tx, job, payload, !inserted).await?;
        finish(&mut tx, job, "stored", 1).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn exclude_match(
        &self,
        job: &Job,
        exclusion: Exclusion,
        calls: i64,
    ) -> Result<(), StorageError> {
        self.finish_job(job, exclusion.outcome(), calls).await
    }

    /// Exclut une partie qui vient d'être téléchargée (un appel) et mémorise ses faits :
    /// une autre exécution au même périmètre ne la retéléchargera pas (#90).
    pub async fn exclude_downloaded_match(
        &self,
        job: &Job,
        exclusion: Exclusion,
        facts: &MatchFacts,
    ) -> Result<(), StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        sqlx::query(
            "INSERT INTO excluded_matches
                (match_id, platform_id, queue_id, game_version, patch, game_start,
                 game_duration_s, is_remake, data_version)
             VALUES ($1, $2, $3, $4, $5, to_timestamp($6::float8 / 1000), $7, $8, $9)
             ON CONFLICT (match_id) DO NOTHING",
        )
        .bind(&facts.match_id)
        .bind(&facts.platform_id)
        .bind(facts.queue_id)
        .bind(&facts.game_version)
        .bind(&facts.patch)
        .bind(facts.game_start_ms)
        .bind(facts.game_duration_s)
        .bind(facts.is_remake)
        .bind(&facts.data_version)
        .execute(&mut *tx)
        .await?;
        finish(&mut tx, job, exclusion.outcome(), 1).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Termine un travail sans autre écriture.
    pub async fn finish_job(
        &self,
        job: &Job,
        outcome: &str,
        calls: i64,
    ) -> Result<(), StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        finish(&mut tx, job, outcome, calls).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Maintenance (#90) : ferme les demandes de rang encore en attente dont aucune partie
    /// retenue n'est classée (files 420/440 hors remake). Celles créées avant le filtrage
    /// par file consomment sinon du budget d'appels pour un rang jamais lu. Un joueur lié à
    /// aucune partie lisible (détail purgé ou caviardé) est conservé : dans le doute, la
    /// demande reste. `apply = false` compte sans rien modifier. Les travaux déjà terminés
    /// ne sont pas touchés ; ceux fermés gardent leur trace (`skipped:unserved_queue`).
    pub async fn close_unserved_rank_jobs(
        &self,
        run_id: Option<i64>,
        apply: bool,
    ) -> Result<u64, StorageError> {
        const ELIGIBLE: &str = "j.kind = 'participant_rank'
             AND j.state IN ('pending', 'retry_wait')
             AND ($1::bigint IS NULL OR j.run_id = $1)
             AND EXISTS (
                 SELECT 1 FROM run_matches rm JOIN matches m ON m.match_id = rm.match_id
                 WHERE rm.run_id = j.run_id
                   AND m.detail -> 'metadata' -> 'participants' @> to_jsonb(j.job_key))
             AND NOT EXISTS (
                 SELECT 1 FROM run_matches rm JOIN matches m ON m.match_id = rm.match_id
                 WHERE rm.run_id = j.run_id
                   AND m.queue_id = ANY($2) AND NOT m.is_remake
                   AND m.detail -> 'metadata' -> 'participants' @> to_jsonb(j.job_key))";
        if !apply {
            let count: i64 = sqlx::query_scalar(&format!(
                "SELECT count(*) FROM collection_jobs j WHERE {ELIGIBLE}"
            ))
            .bind(run_id)
            .bind(RANKED_QUEUE_IDS.as_slice())
            .fetch_one(&self.pool)
            .await?;
            return Ok(count as u64);
        }
        let closed = sqlx::query(&format!(
            "UPDATE collection_jobs j
             SET state = 'done', outcome = 'skipped:unserved_queue', last_error = NULL,
                 updated_at = now()
             WHERE {ELIGIBLE}"
        ))
        .bind(run_id)
        .bind(RANKED_QUEUE_IDS.as_slice())
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(closed)
    }

    /// Cache des deux files de classement pendant 24 h ; aucune absence n'est inférée.
    pub async fn ranks_fresh(
        &self,
        payload: &ParticipantRankPayload,
    ) -> Result<bool, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT count(DISTINCT queue_id) = 2 FROM participant_rank_observations
             WHERE platform_id = $1 AND puuid = $2 AND observed_at >= now() - interval '24 hours'",
        )
        .bind(&payload.platform_id)
        .bind(&payload.puuid)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Une réponse 200 observe les deux files ; une file absente signifie non classé.
    pub async fn store_participant_ranks(
        &self,
        job: &Job,
        payload: &ParticipantRankPayload,
        ranks: &[ParticipantRank],
    ) -> Result<(), StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        for queue_id in RANKED_QUEUE_IDS {
            let rank = ranks.iter().find(|r| r.queue_id == queue_id);
            sqlx::query(
                "INSERT INTO participant_rank_observations
                (platform_id, puuid, queue_id, tier, division, league_points, status)
                VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&payload.platform_id)
            .bind(&payload.puuid)
            .bind(queue_id)
            .bind(rank.map(|r| r.tier.as_str()))
            .bind(rank.map(|r| r.division.as_str()))
            .bind(rank.map(|r| r.league_points))
            .bind(if rank.is_some() { "ranked" } else { "unranked" })
            .execute(&mut *tx)
            .await?;
        }
        finish(&mut tx, job, "observed", 1).await?;
        tx.commit().await?;
        Ok(())
    }

    /// `true` si une timeline est déjà disponible pour cette partie.
    pub async fn timeline_available(&self, match_id: &str) -> Result<bool, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM match_timelines WHERE match_id = $1 AND status = 'available')",
        )
        .bind(match_id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn store_timeline(
        &self,
        job: &Job,
        match_id: &str,
        timeline: &Value,
    ) -> Result<(), StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        sqlx::query(
            "INSERT INTO match_timelines (match_id, status, timeline) VALUES ($1, 'available', $2)
             ON CONFLICT (match_id) DO UPDATE
             SET status = 'available', timeline = EXCLUDED.timeline, fetched_at = now()
             WHERE match_timelines.status <> 'available'",
        )
        .bind(match_id)
        .bind(timeline)
        .execute(&mut *tx)
        .await?;
        finish(&mut tx, job, "available", 1).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Timeline introuvable après les revalidations prévues.
    pub async fn mark_timeline_unavailable(
        &self,
        job: &Job,
        match_id: &str,
    ) -> Result<(), StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        sqlx::query(
            "INSERT INTO match_timelines (match_id, status) VALUES ($1, 'unavailable')
             ON CONFLICT DO NOTHING",
        )
        .bind(match_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE collection_jobs SET not_found_count = not_found_count + 1 WHERE id = $1",
        )
        .bind(job.id)
        .execute(&mut *tx)
        .await?;
        finish(&mut tx, job, "unavailable", 1).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Programme une nouvelle tentative.
    pub async fn retry_job(
        &self,
        job: &Job,
        delay: Duration,
        retry: RetryKind,
        error: &str,
        calls: i64,
    ) -> Result<(), StorageError> {
        let (attempt, not_found) = match retry {
            RetryKind::Attempt => (1, 0),
            RetryKind::NotFound => (0, 1),
            RetryKind::RateLimited => (0, 0),
        };
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        sqlx::query(
            "UPDATE collection_jobs
             SET state = 'retry_wait', attempts = attempts + $2, not_found_count = not_found_count + $3,
                 next_attempt_at = now() + make_interval(secs => $4), last_error = $5, updated_at = now()
             WHERE id = $1",
        )
        .bind(job.id)
        .bind(attempt)
        .bind(not_found)
        .bind(delay.as_secs_f64())
        .bind(error)
        .execute(&mut *tx)
        .await?;
        add_calls(&mut tx, job.run_id, calls).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Échec définitif du travail ; l'exécution continue.
    pub async fn fail_job(&self, job: &Job, error: &str, calls: i64) -> Result<(), StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        sqlx::query(
            "UPDATE collection_jobs
             SET state = 'failed', attempts = attempts + 1, last_error = $2, outcome = 'failed', updated_at = now()
             WHERE id = $1",
        )
        .bind(job.id)
        .bind(error)
        .execute(&mut *tx)
        .await?;
        add_calls(&mut tx, job.run_id, calls).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Conserve le travail réservé jusqu'au drainage pour éviter sa relance sur 401/403.
    pub async fn suspend_job(
        &self,
        job: &Job,
        error: &str,
        calls: i64,
    ) -> Result<(), StorageError> {
        let mut connection = self.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        sqlx::query("UPDATE collection_jobs SET last_error = $2, updated_at = now() WHERE id = $1")
            .bind(job.id)
            .bind(error)
            .execute(&mut *tx)
            .await?;
        add_calls(&mut tx, job.run_id, calls).await?;
        tx.commit().await?;
        Ok(())
    }
}

/// Nature d'une nouvelle tentative, pour les compteurs du travail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryKind {
    /// Erreur serveur, réseau ou réponse invalide : compte une tentative.
    Attempt,
    /// 404 à revalider.
    NotFound,
    /// Quota Riot : ne compte pas comme une tentative.
    RateLimited,
}

fn to_json<T: Serialize>(value: &T) -> Result<Value, StorageError> {
    serde_json::to_value(value).map_err(|e| StorageError::Corrupt(e.to_string()))
}

fn seed_page_key(p: &SeedPagePayload) -> String {
    format!("{}:{}:{}", p.tier.as_str(), p.division.as_str(), p.page)
}

fn match_ids_key(p: &MatchIdsPayload) -> String {
    format!("{}:{}", p.puuid, p.start)
}

fn seed_page_sort(page: u32, stratum: i64, strata_count: i64) -> i64 {
    (i64::from(page) - 1) * strata_count + stratum
}

async fn insert_job<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    run_id: i64,
    kind: JobKind,
    key: &str,
    payload: &T,
    sort_key: i64,
) -> Result<bool, StorageError> {
    let res = sqlx::query(
        "INSERT INTO collection_jobs (run_id, kind, job_key, payload, sort_key)
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
    )
    .bind(run_id)
    .bind(kind.as_str())
    .bind(key)
    .bind(to_json(payload)?)
    .bind(sort_key)
    .execute(&mut **tx)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// Associe la partie à l'exécution et crée son travail de timeline si aucune
/// timeline définitive n'existe.
async fn link_match(
    tx: &mut Transaction<'_, Postgres>,
    job: &Job,
    payload: &MatchPayload,
    already_present: bool,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO run_matches (run_id, match_id, seed_puuid, already_present)
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(job.run_id)
    .bind(&payload.match_id)
    .bind(&payload.seed_puuid)
    .bind(already_present)
    .execute(&mut **tx)
    .await?;
    let has_final_timeline: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM match_timelines WHERE match_id = $1)")
            .bind(&payload.match_id)
            .fetch_one(&mut **tx)
            .await?;
    if !has_final_timeline {
        let t = TimelinePayload {
            match_id: payload.match_id.clone(),
        };
        insert_job(
            tx,
            job.run_id,
            JobKind::Timeline,
            &payload.match_id,
            &t,
            job.sort_key,
        )
        .await?;
    }
    // La même partie peut être réutilisée par une exécution qui active les rangs.
    let rank_source = sqlx::query(
        "SELECT m.platform_id, m.detail
        FROM matches m JOIN collection_runs r ON r.id = $1
        WHERE m.match_id = $2 AND r.params->>'collect_ranks' = 'true'",
    )
    .bind(job.run_id)
    .bind(&payload.match_id)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(source) = rank_source {
        let platform_id: String = source.try_get("platform_id")?;
        let detail: Value = source.try_get("detail")?;
        if let Some(participants) = detail
            .pointer("/info/participants")
            .and_then(Value::as_array)
        {
            for participant in participants {
                let Some(puuid) = participant
                    .get("puuid")
                    .and_then(Value::as_str)
                    .filter(|p| !p.is_empty() && *p != "BOT" && p.bytes().any(|b| b != b'0'))
                else {
                    continue;
                };
                let rank = ParticipantRankPayload {
                    platform_id: platform_id.clone(),
                    puuid: puuid.to_owned(),
                };
                insert_job(
                    tx,
                    job.run_id,
                    JobKind::ParticipantRank,
                    puuid,
                    &rank,
                    job.sort_key,
                )
                .await?;
            }
        }
    }
    Ok(())
}

async fn finish(
    tx: &mut Transaction<'_, Postgres>,
    job: &Job,
    outcome: &str,
    calls: i64,
) -> Result<(), StorageError> {
    sqlx::query(
        "UPDATE collection_jobs SET state = 'done', outcome = $2, last_error = NULL, updated_at = now()
         WHERE id = $1",
    )
    .bind(job.id)
    .bind(outcome)
    .execute(&mut **tx)
    .await?;
    add_calls(tx, job.run_id, calls).await
}

async fn add_calls(
    tx: &mut Transaction<'_, Postgres>,
    run_id: i64,
    calls: i64,
) -> Result<(), StorageError> {
    if calls > 0 {
        sqlx::query(
            "UPDATE collection_runs SET calls_made = calls_made + $2, updated_at = now() WHERE id = $1",
        )
        .bind(run_id)
        .bind(calls)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
