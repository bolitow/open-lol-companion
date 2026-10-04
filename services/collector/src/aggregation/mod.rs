//! Agrégats par champion, rôle, patch, plateforme, file et rang observé (#18).

mod builds;
mod match_tier;
mod model;
mod scheduler;
mod snapshot;
mod stages;
mod storage;

pub use model::{
    AggregationOptions, AggregationReport, BanStats, BuildStats, ChampionStats, Coverage, GroupKey,
    ItemCatalogRef, ItemEventStats, QualityThresholds, Role, ScopeCoverage, ScopeKey, SkillStats,
    DEFAULT_MIN_GAME_DURATION_S, DEFAULT_MIN_PLAYED_PERCENT, DEFAULT_RANK_MAX_AGE_HOURS,
    MAX_MIN_GAME_DURATION_S, MAX_RANK_MAX_AGE_HOURS,
};
pub use scheduler::run_periodic;
pub use storage::{recalculate, recalculate_filtered, recalculate_with_quality};

/// Erreurs d'agrégation : les messages publics n'exposent ni SQL ni donnée brute.
#[derive(Debug, thiserror::Error)]
pub enum AggregationError {
    #[error(transparent)]
    StaticData(#[from] crate::static_data::StaticError),
    #[error("filtres d'agrégation invalides (patch, plateforme, file ou fenêtre)")]
    InvalidFilters,
    #[error("le seuil minimal doit être strictement positif")]
    InvalidThreshold,
    #[error("l'écart maximal du rang doit être compris entre 1 et 8760 heures")]
    InvalidRankMaxAge,
    #[error("la durée minimale d'une partie classée doit être comprise entre 0 et 900 secondes")]
    InvalidMinGameDuration,
    #[error("la part minimale de durée jouée doit être comprise entre 0 et 100 %")]
    InvalidMinPlayedPercent,
    #[error("un autre calcul utilise déjà cette base")]
    Busy,
    #[error("élément d'agrégation trop volumineux ; l'ancien instantané est conservé")]
    SnapshotTooLarge,
    #[error("échec PostgreSQL pendant l'agrégation ; aucune nouvelle publication confirmée")]
    Database(#[from] sqlx::Error),
    #[error("rapport d'agrégation non sérialisable")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests;
