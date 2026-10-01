//! Contrats du référentiel normalisé ; chaque valeur reste reliée à sa source.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const NORMALIZER_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogSource {
    pub id: String,
    pub provider: String,
    pub key: String,
    pub version: String,
    pub locale: Option<String>,
    pub url: String,
    pub observed_at: String,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceMeta {
    pub id: String,
    pub provider: String,
    pub key: String,
    pub version: String,
    pub locale: Option<String>,
    pub url: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValueStatus {
    Verified,
    Derived,
    Descriptive,
    Missing,
    Unsupported,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValueSource {
    pub source_id: String,
    pub pointer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogValue {
    pub value: Value,
    pub unit: Option<String>,
    pub status: ValueStatus,
    pub sources: Vec<ValueSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogEffect {
    pub id: String,
    pub description: Option<String>,
    pub parameters: BTreeMap<String, CatalogValue>,
    pub calculation: Option<CatalogValue>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordCoverage {
    pub source_fields: u32,
    pub normalized_fields: u32,
    pub unmapped_fields: Vec<String>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogRecord {
    /// item, champion, ability, rune, rune_shard, summoner_spell, map, queue, mode, game_type, profile_icon.
    pub kind: String,
    pub id: String,
    pub namespace: String,
    pub locale: String,
    pub name: String,
    /// Texte seul : le consommateur ne doit jamais l'interpréter comme HTML.
    pub description: Option<String>,
    pub icon: Option<String>,
    /// Noms normalisés et stables : price_total, price_base, price_sell, builds_from, builds_into, purchasable, maps…
    pub fields: BTreeMap<String, CatalogValue>,
    pub stats: BTreeMap<String, CatalogValue>,
    pub effects: Vec<CatalogEffect>,
    pub coverage: RecordCoverage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatalogCoverage {
    pub records: usize,
    pub source_fields: u64,
    pub normalized_fields: u64,
    pub unmapped_fields: u64,
    pub records_with_issues: usize,
    pub by_kind: BTreeMap<String, usize>,
}

/// Une branche sans lien vers une valeur publiée ; ce n'est pas une mesure de normalisation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawBranch {
    pub pointer: String,
    pub leaf_fields: u64,
    pub reason: String,
}

/// Inventaire intégral de la source, distinct de la couverture des fiches normalisées.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceInventory {
    pub source_id: String,
    pub total_leaf_fields: u64,
    pub branches: u64,
    pub linked_branches: u64,
    pub raw_branches: Vec<RawBranch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogManifest {
    pub publication_id: String,
    pub version: String,
    pub normalizer_version: u32,
    pub published_at: String,
    pub degraded: bool,
    pub warnings: Vec<String>,
    pub sources: Vec<SourceMeta>,
    #[serde(default)]
    pub source_inventory: Vec<SourceInventory>,
    pub coverage: CatalogCoverage,
}

#[derive(Debug, Clone)]
pub struct CatalogProjection {
    pub version: String,
    pub sources: Vec<CatalogSource>,
    pub records: Vec<CatalogRecord>,
    pub degraded: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("version ou paramètres du catalogue invalides")]
    InvalidRequest,
    #[error("sources du catalogue absentes ou incohérentes")]
    InvalidSource,
    #[error("complément public indisponible")]
    Network,
    #[error("échec PostgreSQL ; publication du catalogue non confirmée")]
    Database,
    #[error("catalogue absent")]
    NotFound,
    #[error("une publication de catalogue est déjà en cours")]
    Busy,
}

impl From<sqlx::Error> for CatalogError {
    fn from(_: sqlx::Error) -> Self {
        Self::Database
    }
}
