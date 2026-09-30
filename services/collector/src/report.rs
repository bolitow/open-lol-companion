//! Bilan d'une exécution, calculé depuis la base (et donc exact après une reprise).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Serialize;
use serde_json::Value;
use sqlx::Row;

use crate::storage::{Storage, StorageError};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Report {
    pub run_id: i64,
    pub status: String,
    pub status_reason: Option<String>,
    /// `true` seulement si la cible est atteinte.
    pub complete: bool,
    pub target: i64,
    pub params: Value,
    pub window_start_utc: String,
    pub window_end_utc: String,
    pub started_at_utc: String,
    pub finished_at_utc: Option<String>,
    pub duration_s: f64,
    pub calls_made: i64,
    pub seeds: Vec<SeedCount>,
    pub discovery: Discovery,
    pub matches: Matches,
    pub timelines: Timelines,
    /// Travaux en échec définitif, toutes étapes confondues.
    pub failed_jobs: i64,
    pub errors: Vec<ErrorCount>,
    pub storage: StorageSize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SeedCount {
    pub tier: String,
    pub division: String,
    pub players: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Discovery {
    /// Mentions dans les historiques (une partie vue par deux joueurs compte deux fois).
    pub sightings: i64,
    pub distinct_matches: i64,
    pub duplicates: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Matches {
    /// Parties retenues par l'exécution (téléchargées ou déjà en base).
    pub retained: i64,
    pub downloaded: i64,
    pub already_present: i64,
    pub remakes: i64,
    pub excluded: BTreeMap<String, i64>,
    pub failed: i64,
    pub retry_wait: i64,
    /// Candidates non téléchargées (cible atteinte ou exécution arrêtée).
    pub not_fetched: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Timelines {
    pub available: i64,
    pub unavailable: i64,
    pub pending: i64,
    pub retry_wait: i64,
    pub failed: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ErrorCount {
    pub kind: String,
    pub state: String,
    pub error: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StorageSize {
    pub details_bytes: i64,
    pub timelines_bytes: i64,
    pub database_bytes: i64,
}

const UTC: &str = r#"'YYYY-MM-DD"T"HH24:MI:SS"Z"'"#;

pub async fn generate(storage: &Storage, run_id: i64) -> Result<Report, StorageError> {
    let pool = storage.pool();
    let run = sqlx::query(&format!(
        "SELECT status, status_reason, target_matches, params, calls_made,
                to_char(window_start AT TIME ZONE 'UTC', {UTC}) AS ws,
                to_char(window_end AT TIME ZONE 'UTC', {UTC}) AS we,
                to_char(started_at AT TIME ZONE 'UTC', {UTC}) AS sa,
                to_char(finished_at AT TIME ZONE 'UTC', {UTC}) AS fa,
                extract(epoch FROM (coalesce(finished_at, updated_at) - started_at))::float8 AS dur
         FROM collection_runs WHERE id = $1"
    ))
    .bind(run_id)
    .fetch_optional(pool)
    .await?
    .ok_or(StorageError::RunNotFound(run_id))?;

    let seeds = sqlx::query(
        "SELECT tier, division, count(*) AS n FROM seed_players WHERE run_id = $1
         GROUP BY tier, division ORDER BY min(seed_index), tier, division",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| {
        Ok(SeedCount {
            tier: r.try_get("tier")?,
            division: r.try_get("division")?,
            players: r.try_get("n")?,
        })
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?;

    let d = sqlx::query(
        "SELECT count(*) AS sightings, count(DISTINCT match_id) AS distinct_ids
         FROM run_discoveries WHERE run_id = $1",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let sightings: i64 = d.try_get("sightings")?;
    let distinct_matches: i64 = d.try_get("distinct_ids")?;

    let mut matches = Matches {
        retained: 0,
        downloaded: 0,
        already_present: 0,
        remakes: 0,
        excluded: BTreeMap::new(),
        failed: 0,
        retry_wait: 0,
        not_fetched: 0,
    };
    let rows = sqlx::query(
        "SELECT state, outcome, count(*) AS n FROM collection_jobs
         WHERE run_id = $1 AND kind = 'match' GROUP BY state, outcome",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    for r in rows {
        let state: String = r.try_get("state")?;
        let outcome: Option<String> = r.try_get("outcome")?;
        let n: i64 = r.try_get("n")?;
        match (state.as_str(), outcome.as_deref()) {
            ("done", Some(o)) if o.starts_with("excluded:") => {
                *matches
                    .excluded
                    .entry(o.trim_start_matches("excluded:").to_owned())
                    .or_default() += n;
            }
            ("done", _) => {}
            ("failed", _) => matches.failed += n,
            ("retry_wait", _) => matches.retry_wait += n,
            _ => matches.not_fetched += n,
        }
    }
    let r = sqlx::query(
        "SELECT count(*) AS retained,
                count(*) FILTER (WHERE rm.already_present) AS already,
                count(*) FILTER (WHERE m.is_remake) AS remakes
         FROM run_matches rm JOIN matches m USING (match_id) WHERE rm.run_id = $1",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    matches.retained = r.try_get("retained")?;
    matches.already_present = r.try_get("already")?;
    matches.downloaded = matches.retained - matches.already_present;
    matches.remakes = r.try_get("remakes")?;

    let t = sqlx::query(
        "SELECT count(*) FILTER (WHERE t.status = 'available') AS available,
                count(*) FILTER (WHERE t.status = 'unavailable') AS unavailable,
                count(*) FILTER (WHERE t.status IS NULL AND j.state = 'retry_wait') AS retry_wait,
                count(*) FILTER (WHERE t.status IS NULL AND j.state = 'failed') AS failed,
                count(*) FILTER (WHERE t.status IS NULL
                                  AND (j.state IS NULL OR j.state IN ('pending', 'running', 'done'))) AS pending
         FROM run_matches rm
         LEFT JOIN match_timelines t USING (match_id)
         LEFT JOIN collection_jobs j
                ON j.run_id = rm.run_id AND j.kind = 'timeline' AND j.job_key = rm.match_id
         WHERE rm.run_id = $1",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let timelines = Timelines {
        available: t.try_get("available")?,
        unavailable: t.try_get("unavailable")?,
        pending: t.try_get("pending")?,
        retry_wait: t.try_get("retry_wait")?,
        failed: t.try_get("failed")?,
    };

    let errors = sqlx::query(
        "SELECT kind, state, last_error, count(*) AS n FROM collection_jobs
         WHERE run_id = $1 AND state IN ('failed', 'retry_wait') AND last_error IS NOT NULL
         GROUP BY kind, state, last_error ORDER BY n DESC, kind LIMIT 10",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| {
        Ok(ErrorCount {
            kind: r.try_get("kind")?,
            state: r.try_get("state")?,
            error: r.try_get("last_error")?,
            count: r.try_get("n")?,
        })
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?;

    let failed_jobs: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM collection_jobs WHERE run_id = $1 AND state = 'failed'",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;

    let s = sqlx::query(
        "SELECT coalesce(sum(pg_column_size(m.detail)), 0)::bigint AS details,
                coalesce(sum(pg_column_size(t.timeline)), 0)::bigint AS timelines,
                pg_database_size(current_database()) AS db
         FROM run_matches rm
         JOIN matches m USING (match_id)
         LEFT JOIN match_timelines t USING (match_id)
         WHERE rm.run_id = $1",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;

    let target: i32 = run.try_get("target_matches")?;
    Ok(Report {
        run_id,
        status: run.try_get("status")?,
        status_reason: run.try_get("status_reason")?,
        complete: matches.retained >= i64::from(target),
        target: i64::from(target),
        params: run.try_get("params")?,
        window_start_utc: run.try_get("ws")?,
        window_end_utc: run.try_get("we")?,
        started_at_utc: run.try_get("sa")?,
        finished_at_utc: run.try_get("fa")?,
        duration_s: run.try_get("dur")?,
        calls_made: run.try_get("calls_made")?,
        seeds,
        discovery: Discovery {
            sightings,
            distinct_matches,
            duplicates: sightings - distinct_matches,
        },
        matches,
        timelines,
        failed_jobs,
        errors,
        storage: StorageSize {
            details_bytes: s.try_get("details")?,
            timelines_bytes: s.try_get("timelines")?,
            database_bytes: s.try_get("db")?,
        },
    })
}

fn mib(bytes: i64) -> String {
    format!("{:.1} Mio", bytes as f64 / (1024.0 * 1024.0))
}

impl Report {
    /// Bilan lisible, en français.
    pub fn render(&self) -> String {
        let mut o = String::new();
        let verdict = if self.complete {
            "COMPLET"
        } else {
            "INCOMPLET"
        };
        let _ = writeln!(
            o,
            "Exécution #{} — {} ({}{})",
            self.run_id,
            verdict,
            self.status,
            self.status_reason
                .as_deref()
                .map(|r| format!(", {r}"))
                .unwrap_or_default()
        );
        let _ = writeln!(
            o,
            "Parties retenues : {} / {}",
            self.matches.retained, self.target
        );
        let _ = writeln!(
            o,
            "Fenêtre : {} → {} (UTC)",
            self.window_start_utc, self.window_end_utc
        );
        let _ = writeln!(
            o,
            "Début : {} · fin : {} · durée depuis le lancement : {:.0} s · appels Riot : {}",
            self.started_at_utc,
            self.finished_at_utc.as_deref().unwrap_or("—"),
            self.duration_s,
            self.calls_made
        );
        let seeds: i64 = self.seeds.iter().map(|s| s.players).sum();
        let _ = writeln!(o, "\nJoueurs de départ : {seeds}");
        for s in &self.seeds {
            let _ = writeln!(o, "  {} {} : {}", s.tier, s.division, s.players);
        }
        let d = &self.discovery;
        let _ = writeln!(
            o,
            "\nDécouverte : {} mentions, {} parties distinctes, {} doublons",
            d.sightings, d.distinct_matches, d.duplicates
        );
        let m = &self.matches;
        let _ = writeln!(
            o,
            "Parties : {} téléchargées, {} déjà en base, {} remakes, {} en échec, {} à réessayer, {} non téléchargées",
            m.downloaded, m.already_present, m.remakes, m.failed, m.retry_wait, m.not_fetched
        );
        if m.excluded.is_empty() {
            let _ = writeln!(o, "Exclues : 0");
        } else {
            let list: Vec<String> = m.excluded.iter().map(|(k, v)| format!("{k} {v}")).collect();
            let _ = writeln!(o, "Exclues : {}", list.join(", "));
        }
        let t = &self.timelines;
        let _ = writeln!(
            o,
            "Timelines : {} disponibles, {} indisponibles, {} en attente, {} à réessayer, {} en échec",
            t.available, t.unavailable, t.pending, t.retry_wait, t.failed
        );
        if !self.errors.is_empty() {
            let _ = writeln!(o, "\nErreurs principales :");
            for e in &self.errors {
                let _ = writeln!(o, "  {} × {} [{}] {}", e.count, e.kind, e.state, e.error);
            }
        }
        let _ = writeln!(
            o,
            "\nStockage : détails {}, timelines {} (base entière : {})",
            mib(self.storage.details_bytes),
            mib(self.storage.timelines_bytes),
            mib(self.storage.database_bytes)
        );
        o
    }
}
