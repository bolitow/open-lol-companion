//! Agrégats par champion, rôle, patch, plateforme, file et rang observé (#18).

mod builds;
mod model;
mod scheduler;
mod snapshot;
mod storage;

pub use model::{AggregationOptions, AggregationReport, ChampionStats, GroupKey, Role};
pub use scheduler::run_periodic;
pub use storage::{recalculate, recalculate_filtered};

/// Erreurs d'agrégation : les messages publics n'exposent ni SQL ni donnée brute.
#[derive(Debug, thiserror::Error)]
pub enum AggregationError {
    #[error(transparent)]
    StaticData(#[from] crate::static_data::StaticError),
    #[error("filtres d'agrégation invalides (patch, plateforme, file ou fenêtre)")]
    InvalidFilters,
    #[error("le seuil minimal doit être strictement positif")]
    InvalidThreshold,
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
