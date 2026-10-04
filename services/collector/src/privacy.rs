//! Données personnelles (#99) : rétention configurable des identifiants de joueurs et
//! des parties brutes, purge planifiée, export et effacement d'un PUUID sur demande.
//!
//! Aucun PUUID n'est journalisé ni repris dans un message d'erreur.

use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use sqlx::postgres::{PgQueryResult, PgRow};
use sqlx::{Postgres, Row, Transaction};

use crate::storage::Storage;

/// Durée de conservation proposée des PUUID et Riot ID, en jours (à valider, #99).
pub const DEFAULT_IDENTIFIER_DAYS: u32 = 30;
/// Durée de conservation proposée des parties brutes (détails et timelines), en jours.
pub const DEFAULT_RAW_MATCH_DAYS: u32 = 90;
/// Borne haute des durées configurables (dix ans).
pub const MAX_RETENTION_DAYS: u32 = 3650;

#[derive(Debug, Error)]
pub enum PrivacyError {
    #[error("durée de conservation invalide : entre 1 et {MAX_RETENTION_DAYS} jours")]
    InvalidRetention,
    #[error("identifiant de joueur invalide")]
    InvalidSubject,
    #[error("échec PostgreSQL pendant le traitement des données personnelles")]
    Database(#[from] sqlx::Error),
}

/// Durées de conservation, comptées depuis l'enregistrement de la donnée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RetentionPolicy {
    identifier_days: u32,
    raw_match_days: u32,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            identifier_days: DEFAULT_IDENTIFIER_DAYS,
            raw_match_days: DEFAULT_RAW_MATCH_DAYS,
        }
    }
}

impl RetentionPolicy {
    pub fn new(identifier_days: u32, raw_match_days: u32) -> Result<Self, PrivacyError> {
        let valid = 1..=MAX_RETENTION_DAYS;
        if !valid.contains(&identifier_days) || !valid.contains(&raw_match_days) {
            return Err(PrivacyError::InvalidRetention);
        }
        Ok(Self {
            identifier_days,
            raw_match_days,
        })
    }

    pub fn identifier_days(&self) -> u32 {
        self.identifier_days
    }

    pub fn raw_match_days(&self) -> u32 {
        self.raw_match_days
    }
}

/// PUUID plausible : borné, sans caractère de contrôle, et jamais un marqueur de bot.
pub fn is_valid_puuid(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        && !is_bot_marker(value)
}

/// Marqueurs de bot de match-v5 : ce ne sont pas des données personnelles et
/// l'agrégation des files coop s'en sert pour compter les humains.
fn is_bot_marker(value: &str) -> bool {
    value == "BOT" || (!value.is_empty() && value.bytes().all(|b| b == b'0'))
}

/// Champs qui identifient un joueur dans `info.participants` (détail et timeline).
/// Aucun n'est lu par l'agrégation.
const IDENTIFIER_FIELDS: [&str; 7] = [
    "puuid",
    "summonerId",
    "summonerName",
    "riotIdGameName",
    "riotIdName",
    "riotIdTagline",
    "profileIcon",
];

/// Un identifiant est à effacer s'il appartient à un humain et, en mode ciblé, au sujet.
fn targets(puuid: &str, subject: Option<&str>) -> bool {
    !puuid.is_empty() && !is_bot_marker(puuid) && (subject.is_none() || subject == Some(puuid))
}

/// Retire les identifiants de joueurs d'un détail ou d'une timeline match-v5.
///
/// `subject` limite l'effacement à ce PUUID ; `None` traite tous les joueurs humains.
/// Renvoie `true` si le document a changé.
///
/// Les entrées de `metadata.participants` deviennent `""` au lieu d'être retirées : la
/// validation des agrégats compare leur nombre à celui des participants. Les bots
/// (`BOT`, zéros) gardent leur marqueur.
pub fn redact_identifiers(document: &mut Value, subject: Option<&str>) -> bool {
    let mut changed = false;
    if let Some(ids) = document
        .pointer_mut("/metadata/participants")
        .and_then(Value::as_array_mut)
    {
        for id in ids {
            if id.as_str().is_some_and(|p| targets(p, subject)) {
                *id = Value::String(String::new());
                changed = true;
            }
        }
    }
    if let Some(participants) = document
        .pointer_mut("/info/participants")
        .and_then(Value::as_array_mut)
    {
        for participant in participants {
            let Some(fields) = participant.as_object_mut() else {
                continue;
            };
            let selected = match fields.get("puuid").and_then(Value::as_str) {
                Some(puuid) => targets(puuid, subject),
                // Sans PUUID, seul le mode complet retire les noms restants.
                None => subject.is_none(),
            };
            if selected {
                for field in IDENTIFIER_FIELDS {
                    changed |= fields.remove(field).is_some();
                }
            }
        }
    }
    changed
}

/// Bilan d'une purge : nombre de lignes supprimées ou pseudonymisées par catégorie.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PurgeReport {
    pub identifier_days: u32,
    pub raw_match_days: u32,
    pub matches_deleted: u64,
    pub timelines_deleted: u64,
    pub matches_redacted: u64,
    pub timelines_redacted: u64,
    pub seed_players_deleted: u64,
    pub discoveries_deleted: u64,
    pub sampled_match_seeds_cleared: u64,
    pub jobs_deleted: u64,
    pub rank_observations_deleted: u64,
}

/// Classement d'un joueur de départ relevé par une exécution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SeedEntry {
    pub run_id: i64,
    pub platform_id: String,
    pub tier: String,
    pub division: String,
    pub league_points: Option<i32>,
    pub observed_at_ms: i64,
}

/// Rang Solo/Flex observé pour un participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RankObservation {
    pub platform_id: String,
    pub queue_id: i32,
    pub status: String,
    pub tier: Option<String>,
    pub division: Option<String>,
    pub league_points: Option<i32>,
    pub observed_at_ms: i64,
}

/// Partie trouvée (`discoveries`) ou retenue (`sampled_matches`) via l'historique du joueur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunMatchLink {
    pub run_id: i64,
    pub match_id: String,
    pub recorded_at_ms: i64,
}

/// Partie stockée où le joueur apparaît, avec sa seule fiche de participant.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MatchParticipation {
    pub match_id: String,
    pub platform_id: String,
    pub queue_id: i32,
    pub patch: String,
    pub game_start_ms: i64,
    pub participant: Option<Value>,
}

/// Données personnelles détenues pour un PUUID, sans celles des autres joueurs.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SubjectExport {
    pub puuid: String,
    pub seed_entries: Vec<SeedEntry>,
    pub rank_observations: Vec<RankObservation>,
    pub discoveries: Vec<RunMatchLink>,
    pub sampled_matches: Vec<RunMatchLink>,
    pub matches: Vec<MatchParticipation>,
    pub timeline_match_ids: Vec<String>,
    /// Travaux de collecte, quel que soit leur état, qui portent encore ce PUUID.
    pub collection_jobs: i64,
}

/// Bilan d'un effacement ; `jobs_in_flight` > 0 impose de relancer après la collecte.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SubjectErasure {
    pub seed_entries: u64,
    pub rank_observations: u64,
    pub discoveries: u64,
    pub sampled_matches: u64,
    pub jobs: u64,
    pub jobs_in_flight: u64,
    pub matches: u64,
    pub timelines: u64,
}

/// Lignes traitées par transaction : borne la durée des verrous et la mémoire.
const BATCH: i64 = 200;

/// Millisecondes Unix d'une colonne horodatée, pour l'export JSON.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// Applique la politique de rétention ; idempotent, à lancer périodiquement.
///
/// 1. Parties brutes plus anciennes que `raw_match_days` : supprimées avec leur
///    timeline et leurs liens d'exécution, sauf si une collecte en cours les retient.
/// 2. Identifiants plus anciens que `identifier_days` : joueurs de départ, découvertes
///    et travaux des exécutions inactives supprimés, PUUID des liens vidés,
///    observations de rang supprimées, PUUID et Riot ID retirés du JSONB.
pub async fn purge(storage: &Storage, policy: RetentionPolicy) -> Result<PurgeReport, sqlx::Error> {
    let mut report = PurgeReport {
        identifier_days: policy.identifier_days,
        raw_match_days: policy.raw_match_days,
        ..PurgeReport::default()
    };
    let raw_days = policy.raw_match_days as i32;
    let identifier_days = policy.identifier_days as i32;

    loop {
        let mut connection = storage.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT m.match_id FROM matches m
             WHERE m.fetched_at < now() - make_interval(days => $1)
               AND NOT EXISTS (SELECT 1 FROM run_matches rm JOIN collection_runs r ON r.id = rm.run_id
                               WHERE rm.match_id = m.match_id AND r.status = 'running')
             ORDER BY m.match_id LIMIT $2 FOR UPDATE SKIP LOCKED",
        )
        .bind(raw_days)
        .bind(BATCH)
        .fetch_all(&mut *tx)
        .await?;
        if ids.is_empty() {
            break;
        }
        // Clés étrangères sans cascade : timelines et liens avant la partie.
        report.timelines_deleted += affected(
            sqlx::query("DELETE FROM match_timelines WHERE match_id = ANY($1)")
                .bind(&ids)
                .execute(&mut *tx)
                .await?,
        );
        sqlx::query("DELETE FROM run_matches WHERE match_id = ANY($1)")
            .bind(&ids)
            .execute(&mut *tx)
            .await?;
        report.matches_deleted += affected(
            sqlx::query("DELETE FROM matches WHERE match_id = ANY($1)")
                .bind(&ids)
                .execute(&mut *tx)
                .await?,
        );
        tx.commit().await?;
    }

    // Une exécution sans activité depuis la durée de rétention est terminée ou
    // abandonnée : ses travaux (qui portent des PUUID) ne servent plus à la reprise.
    let runs: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM collection_runs WHERE updated_at < now() - make_interval(days => $1) ORDER BY id",
    )
    .bind(identifier_days)
    .fetch_all(storage.pool())
    .await?;
    for run_id in runs {
        let mut connection = storage.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        // Revérifié sous verrou : une reprise concurrente rend l'exécution active.
        let still_idle: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM collection_runs
             WHERE id = $1 AND updated_at < now() - make_interval(days => $2)
             FOR UPDATE SKIP LOCKED",
        )
        .bind(run_id)
        .bind(identifier_days)
        .fetch_optional(&mut *tx)
        .await?;
        if still_idle.is_none() {
            continue;
        }
        report.seed_players_deleted += affected(
            sqlx::query("DELETE FROM seed_players WHERE run_id = $1")
                .bind(run_id)
                .execute(&mut *tx)
                .await?,
        );
        report.discoveries_deleted += affected(
            sqlx::query("DELETE FROM run_discoveries WHERE run_id = $1")
                .bind(run_id)
                .execute(&mut *tx)
                .await?,
        );
        report.sampled_match_seeds_cleared += affected(
            sqlx::query(
                "UPDATE run_matches SET seed_puuid = '' WHERE run_id = $1 AND seed_puuid <> ''",
            )
            .bind(run_id)
            .execute(&mut *tx)
            .await?,
        );
        report.jobs_deleted += affected(
            sqlx::query("DELETE FROM collection_jobs WHERE run_id = $1")
                .bind(run_id)
                .execute(&mut *tx)
                .await?,
        );
        tx.commit().await?;
    }

    loop {
        let deleted = affected(
            sqlx::query(
                "DELETE FROM participant_rank_observations WHERE id IN (
                    SELECT id FROM participant_rank_observations
                    WHERE observed_at < now() - make_interval(days => $1) LIMIT $2)",
            )
            .bind(identifier_days)
            .bind(BATCH * 25)
            .execute(storage.pool())
            .await?,
        );
        if deleted == 0 {
            break;
        }
        report.rank_observations_deleted += deleted;
    }

    report.matches_redacted = redact_expired(storage, Document::Match, identifier_days).await?;
    report.timelines_redacted =
        redact_expired(storage, Document::Timeline, identifier_days).await?;
    Ok(report)
}

/// Document JSONB qui contient des identifiants de participants.
#[derive(Clone, Copy)]
enum Document {
    Match,
    Timeline,
}

impl Document {
    fn select_expired(self) -> &'static str {
        match self {
            Document::Match => {
                "SELECT match_id, detail AS document FROM matches
                 WHERE identifiers_redacted_at IS NULL AND fetched_at < now() - make_interval(days => $1)
                 ORDER BY match_id LIMIT $2 FOR UPDATE SKIP LOCKED"
            }
            Document::Timeline => {
                "SELECT match_id, timeline AS document FROM match_timelines
                 WHERE identifiers_redacted_at IS NULL AND fetched_at < now() - make_interval(days => $1)
                 ORDER BY match_id LIMIT $2 FOR UPDATE SKIP LOCKED"
            }
        }
    }

    fn select_subject(self) -> &'static str {
        match self {
            Document::Match => {
                "SELECT match_id, detail AS document FROM matches
                 WHERE detail -> 'metadata' -> 'participants' @> jsonb_build_array($1::text)
                 ORDER BY match_id FOR UPDATE"
            }
            Document::Timeline => {
                "SELECT match_id, timeline AS document FROM match_timelines
                 WHERE timeline -> 'metadata' -> 'participants' @> jsonb_build_array($1::text)
                 ORDER BY match_id FOR UPDATE"
            }
        }
    }

    /// `$3` vrai : la purge complète est faite, la ligne ne sera plus relue.
    fn update(self) -> &'static str {
        match self {
            Document::Match => {
                "UPDATE matches SET detail = COALESCE($2, detail),
                    identifiers_redacted_at = CASE WHEN $3 THEN now() ELSE identifiers_redacted_at END
                 WHERE match_id = $1"
            }
            Document::Timeline => {
                "UPDATE match_timelines SET timeline = COALESCE($2, timeline),
                    identifiers_redacted_at = CASE WHEN $3 THEN now() ELSE identifiers_redacted_at END
                 WHERE match_id = $1"
            }
        }
    }
}

/// Retire les identifiants des documents expirés, par lots ; renvoie les documents modifiés.
async fn redact_expired(storage: &Storage, kind: Document, days: i32) -> Result<u64, sqlx::Error> {
    let mut changed = 0;
    loop {
        let mut connection = storage.transaction_connection().await?;
        let mut tx = connection.begin().await?;
        let rows = sqlx::query(kind.select_expired())
            .bind(days)
            .bind(BATCH)
            .fetch_all(&mut *tx)
            .await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            changed += redact_row(&mut tx, kind, &row, None).await?;
        }
        tx.commit().await?;
    }
    Ok(changed)
}

/// Pseudonymise une ligne lue par `select_*` ; renvoie 1 si son document a changé.
async fn redact_row(
    tx: &mut Transaction<'_, Postgres>,
    kind: Document,
    row: &PgRow,
    subject: Option<&str>,
) -> Result<u64, sqlx::Error> {
    let match_id: String = row.try_get("match_id")?;
    // Timeline « unavailable » : pas de document, seulement marquée comme traitée.
    let mut document: Option<Value> = row.try_get("document")?;
    let changed = document
        .as_mut()
        .is_some_and(|d| redact_identifiers(d, subject));
    sqlx::query(kind.update())
        .bind(&match_id)
        .bind(if changed { document } else { None })
        .bind(subject.is_none())
        .execute(&mut **tx)
        .await?;
    Ok(u64::from(changed))
}

fn affected(result: PgQueryResult) -> u64 {
    result.rows_affected()
}

fn checked_subject(puuid: &str) -> Result<&str, PrivacyError> {
    if is_valid_puuid(puuid) {
        Ok(puuid)
    } else {
        Err(PrivacyError::InvalidSubject)
    }
}

/// Condition SQL des travaux de collecte qui portent le PUUID `$1`.
const SUBJECT_JOBS: &str = "(payload ->> 'puuid' = $1 OR payload ->> 'seed_puuid' = $1)";

/// Rassemble, dans un instantané cohérent, les données détenues pour ce PUUID.
pub async fn export_subject(storage: &Storage, puuid: &str) -> Result<SubjectExport, PrivacyError> {
    let puuid = checked_subject(puuid)?;
    let mut connection = storage.transaction_connection().await?;
    let mut tx = connection.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;

    let mut seed_entries = vec![];
    for row in sqlx::query(concat!(
        "SELECT run_id, platform_id, tier, division, league_points, ",
        ms!("observed_at"),
        " AS at FROM seed_players WHERE puuid = $1 ORDER BY observed_at, run_id"
    ))
    .bind(puuid)
    .fetch_all(&mut *tx)
    .await?
    {
        seed_entries.push(SeedEntry {
            run_id: row.try_get("run_id")?,
            platform_id: row.try_get("platform_id")?,
            tier: row.try_get("tier")?,
            division: row.try_get("division")?,
            league_points: row.try_get("league_points")?,
            observed_at_ms: row.try_get("at")?,
        });
    }

    let mut rank_observations = vec![];
    for row in sqlx::query(concat!(
        "SELECT platform_id, queue_id, status, tier, division, league_points, ",
        ms!("observed_at"),
        " AS at FROM participant_rank_observations WHERE puuid = $1 ORDER BY observed_at, id"
    ))
    .bind(puuid)
    .fetch_all(&mut *tx)
    .await?
    {
        rank_observations.push(RankObservation {
            platform_id: row.try_get("platform_id")?,
            queue_id: row.try_get("queue_id")?,
            status: row.try_get("status")?,
            tier: row.try_get("tier")?,
            division: row.try_get("division")?,
            league_points: row.try_get("league_points")?,
            observed_at_ms: row.try_get("at")?,
        });
    }

    let discoveries = links(
        &mut tx,
        concat!(
            "SELECT run_id, match_id, ",
            ms!("discovered_at"),
            " AS at FROM run_discoveries WHERE seed_puuid = $1 ORDER BY discovered_at, run_id, match_id"
        ),
        puuid,
    )
    .await?;
    let sampled_matches = links(
        &mut tx,
        concat!(
            "SELECT run_id, match_id, ",
            ms!("linked_at"),
            " AS at FROM run_matches WHERE seed_puuid = $1 ORDER BY linked_at, run_id, match_id"
        ),
        puuid,
    )
    .await?;

    let mut matches = vec![];
    // Seule la fiche du joueur est extraite : jamais les identifiants des autres.
    for row in sqlx::query(concat!(
        "SELECT m.match_id, m.platform_id, m.queue_id, m.patch, ",
        ms!("m.game_start"),
        " AS game_start_ms,
            (SELECT p FROM jsonb_array_elements(
                CASE WHEN jsonb_typeof(m.detail #> '{info,participants}') = 'array'
                     THEN m.detail #> '{info,participants}' ELSE '[]'::jsonb END) p
             WHERE p ->> 'puuid' = $1 LIMIT 1) AS participant
         FROM matches m
         WHERE m.detail -> 'metadata' -> 'participants' @> jsonb_build_array($1::text)
         ORDER BY m.game_start, m.match_id"
    ))
    .bind(puuid)
    .fetch_all(&mut *tx)
    .await?
    {
        matches.push(MatchParticipation {
            match_id: row.try_get("match_id")?,
            platform_id: row.try_get("platform_id")?,
            queue_id: row.try_get("queue_id")?,
            patch: row.try_get("patch")?,
            game_start_ms: row.try_get("game_start_ms")?,
            participant: row.try_get("participant")?,
        });
    }

    let timeline_match_ids = sqlx::query_scalar(
        "SELECT match_id FROM match_timelines
         WHERE timeline -> 'metadata' -> 'participants' @> jsonb_build_array($1::text)
         ORDER BY match_id",
    )
    .bind(puuid)
    .fetch_all(&mut *tx)
    .await?;
    let collection_jobs = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM collection_jobs WHERE {SUBJECT_JOBS}"
    ))
    .bind(puuid)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(SubjectExport {
        puuid: puuid.to_owned(),
        seed_entries,
        rank_observations,
        discoveries,
        sampled_matches,
        matches,
        timeline_match_ids,
        collection_jobs,
    })
}

async fn links(
    tx: &mut Transaction<'_, Postgres>,
    sql: &'static str,
    puuid: &str,
) -> Result<Vec<RunMatchLink>, sqlx::Error> {
    sqlx::query(sql)
        .bind(puuid)
        .fetch_all(&mut **tx)
        .await?
        .iter()
        .map(|row| {
            Ok(RunMatchLink {
                run_id: row.try_get("run_id")?,
                match_id: row.try_get("match_id")?,
                recorded_at_ms: row.try_get("at")?,
            })
        })
        .collect()
}

/// Efface ce PUUID de toutes les tables, en une transaction ; idempotent.
///
/// Les parties restent, sans les identifiants de ce joueur. Un travail de collecte en
/// cours (`running`) n'est pas supprimé : il est compté dans `jobs_in_flight` et
/// l'effacement doit être relancé après la fin de la collecte.
pub async fn erase_subject(storage: &Storage, puuid: &str) -> Result<SubjectErasure, PrivacyError> {
    let puuid = checked_subject(puuid)?;
    let mut connection = storage.transaction_connection().await?;
    let mut tx = connection.begin().await?;
    let mut erasure = SubjectErasure::default();
    for (sql, counter) in [
        (
            "DELETE FROM seed_players WHERE puuid = $1",
            &mut erasure.seed_entries,
        ),
        (
            "DELETE FROM participant_rank_observations WHERE puuid = $1",
            &mut erasure.rank_observations,
        ),
        (
            "DELETE FROM run_discoveries WHERE seed_puuid = $1",
            &mut erasure.discoveries,
        ),
        (
            "UPDATE run_matches SET seed_puuid = '' WHERE seed_puuid = $1",
            &mut erasure.sampled_matches,
        ),
    ] {
        *counter = affected(sqlx::query(sql).bind(puuid).execute(&mut *tx).await?);
    }
    erasure.jobs = affected(
        sqlx::query(&format!(
            "DELETE FROM collection_jobs WHERE {SUBJECT_JOBS} AND state <> 'running'"
        ))
        .bind(puuid)
        .execute(&mut *tx)
        .await?,
    );
    let in_flight: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM collection_jobs WHERE {SUBJECT_JOBS}"
    ))
    .bind(puuid)
    .fetch_one(&mut *tx)
    .await?;
    erasure.jobs_in_flight = u64::try_from(in_flight).unwrap_or(0);
    for (kind, counter) in [
        (Document::Match, &mut erasure.matches),
        (Document::Timeline, &mut erasure.timelines),
    ] {
        let rows = sqlx::query(kind.select_subject())
            .bind(puuid)
            .fetch_all(&mut *tx)
            .await?;
        for row in &rows {
            *counter += redact_row(&mut tx, kind, row, Some(puuid)).await?;
        }
    }
    tx.commit().await?;
    Ok(erasure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::fixtures::match_detail;
    use serde_json::json;

    fn coop_detail() -> Value {
        let mut detail = match_detail("EUW1_1", "EUW1", 870, 1_000_000);
        for i in 5..10 {
            detail["info"]["participants"][i]["puuid"] = json!("BOT");
            detail["info"]["participants"][i]["riotIdGameName"] = json!("Bot");
        }
        detail["metadata"]["participants"] = json!((0..5)
            .map(|i| format!("fake-puuid-{i}"))
            .collect::<Vec<_>>());
        detail
    }

    #[test]
    fn la_politique_refuse_les_durees_nulles_ou_demesurees() {
        let policy = RetentionPolicy::new(7, 120).unwrap();
        assert_eq!(
            (policy.identifier_days(), policy.raw_match_days()),
            (7, 120)
        );
        assert!(RetentionPolicy::new(0, 90).is_err());
        assert!(RetentionPolicy::new(30, 0).is_err());
        assert!(RetentionPolicy::new(MAX_RETENTION_DAYS + 1, 90).is_err());
        assert_eq!(
            RetentionPolicy::default(),
            RetentionPolicy::new(DEFAULT_IDENTIFIER_DAYS, DEFAULT_RAW_MATCH_DAYS).unwrap()
        );
    }

    #[test]
    fn un_puuid_valide_est_borne_et_n_est_pas_un_bot() {
        assert!(is_valid_puuid("fake-puuid-3"));
        assert!(is_valid_puuid(&"aZ09_-".repeat(13)));
        for invalid in ["", "BOT", "0000", "a b", "a/b", "é", &"a".repeat(129)] {
            assert!(!is_valid_puuid(invalid), "{invalid:?}");
        }
    }

    #[test]
    fn la_purge_retire_tous_les_identifiants_humains_sans_casser_la_structure() {
        let mut detail = match_detail("EUW1_1", "EUW1", 420, 1_000_000);
        detail["info"]["participants"][0]["summonerId"] = json!("enc-id");
        detail["info"]["participants"][0]["summonerName"] = json!("Nom");
        detail["info"]["participants"][0]["riotIdTagline"] = json!("EUW");
        detail["info"]["participants"][0]["profileIcon"] = json!(29);
        let before = detail.clone();
        assert!(redact_identifiers(&mut detail, None));
        let text = detail.to_string();
        for private in [
            "fake-puuid",
            "Joueur",
            "enc-id",
            "Nom",
            "riotIdTagline",
            "profileIcon",
        ] {
            assert!(!text.contains(private), "{private} encore présent");
        }
        // Même nombre d'entrées : la validation des agrégats compare ces longueurs.
        assert_eq!(detail["metadata"]["participants"], json!(vec![""; 10]));
        let participants = detail["info"]["participants"].as_array().unwrap();
        assert_eq!(participants.len(), 10);
        for (after, before) in participants
            .iter()
            .zip(before["info"]["participants"].as_array().unwrap())
        {
            for field in [
                "participantId",
                "championId",
                "teamId",
                "teamPosition",
                "win",
            ] {
                assert_eq!(after[field], before[field], "{field}");
            }
        }
        assert_eq!(detail["metadata"]["matchId"], "EUW1_1");
        assert!(
            !redact_identifiers(&mut detail, None),
            "deuxième passage sans effet"
        );
    }

    #[test]
    fn les_bots_gardent_leur_marqueur_pour_les_files_coop() {
        let mut detail = coop_detail();
        assert!(redact_identifiers(&mut detail, None));
        let participants = detail["info"]["participants"].as_array().unwrap();
        assert!(participants[..5].iter().all(|p| p.get("puuid").is_none()));
        assert!(participants[5..].iter().all(|p| p["puuid"] == "BOT"));
        assert_eq!(detail["metadata"]["participants"], json!(vec![""; 5]));
    }

    #[test]
    fn l_effacement_cible_ne_touche_que_le_joueur_demande() {
        let mut detail = match_detail("EUW1_1", "EUW1", 420, 1_000_000);
        assert!(redact_identifiers(&mut detail, Some("fake-puuid-3")));
        assert!(!detail.to_string().contains("fake-puuid-3\""));
        assert!(!detail.to_string().contains("Joueur3"));
        assert_eq!(detail["metadata"]["participants"][3], "");
        assert_eq!(detail["metadata"]["participants"][4], "fake-puuid-4");
        assert_eq!(
            detail["info"]["participants"][4]["riotIdGameName"],
            "Joueur4"
        );
        assert_eq!(detail["info"]["participants"][3]["championId"], 4);
        assert!(!redact_identifiers(&mut detail, Some("fake-puuid-3")));
        assert!(!redact_identifiers(&mut detail, Some("absent")));
    }

    #[test]
    fn la_timeline_perd_aussi_ses_puuid() {
        let mut timeline = json!({
            "metadata": {"matchId": "EUW1_1", "participants": ["fake-puuid-0", "fake-puuid-1"]},
            "info": {
                "participants": [
                    {"participantId": 1, "puuid": "fake-puuid-0"},
                    {"participantId": 2, "puuid": "fake-puuid-1"}
                ],
                "frames": [{"timestamp": 0, "events": []}]
            }
        });
        assert!(redact_identifiers(&mut timeline, None));
        assert!(!timeline.to_string().contains("fake-puuid"));
        assert_eq!(
            timeline["info"]["participants"][1],
            json!({"participantId": 2})
        );
        assert_eq!(timeline["info"]["frames"][0]["timestamp"], 0);
    }

    #[test]
    fn un_document_inattendu_est_laisse_intact() {
        for mut document in [
            json!(null),
            json!({"metadata": {"participants": 3}}),
            json!([1, 2]),
        ] {
            let before = document.clone();
            assert!(!redact_identifiers(&mut document, None));
            assert_eq!(document, before);
        }
    }
}
