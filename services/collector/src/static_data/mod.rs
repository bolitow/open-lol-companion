//! Cache des données publiques Riot par version Data Dragon (#18).

mod download;
mod model;
mod storage;
mod transport;

pub use storage::{
    cached_patches, sync_recent, sync_recent_refresh, sync_with_transport, ReleaseSummary,
    SyncReport,
};
pub use transport::{StaticResponse, StaticTransport};

#[cfg(any(test, feature = "test-fixtures"))]
pub mod test_support;

/// Erreurs publiques sans réponse HTTP, URL ou détail SQL.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StaticError {
    #[error("le nombre de patchs doit être compris entre 1 et 10")]
    InvalidCount,
    #[error("versions publiques Riot indisponibles ou incohérentes")]
    InvalidManifest,
    #[error("données statiques Riot incomplètes ou incohérentes")]
    InvalidDocument,
    #[error("téléchargement des données publiques Riot impossible")]
    Network,
    #[error("échec PostgreSQL ; aucune nouvelle publication statique confirmée")]
    Database,
    #[error("une synchronisation statique utilise déjà cette base")]
    Busy,
    #[error("cache statique absent ou incomplet ; synchronisation requise")]
    CacheMissing,
}

impl From<sqlx::Error> for StaticError {
    fn from(_: sqlx::Error) -> Self {
        Self::Database
    }
}

#[cfg(test)]
mod tests;
