//! Référentiel de jeu normalisé et versionné (#61).
pub mod community;
pub mod model;
pub mod normalize;
pub use model::*;
mod source;
pub use source::{make_source, valid_version};
mod storage;
pub use storage::{archived_sources, publish};
mod pipeline;
pub use pipeline::{
    build, cached_sources, combine_sources, project_sources, rebuild, CommunityPolicy,
};

mod inventory;
pub use inventory::inventory;
