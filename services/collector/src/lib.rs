//! Collecteur de parties Riot pour Open LoL Companion (#17).
//!
//! Chaîne : joueurs de départ (league-v4) → historiques Ranked Solo/Duo → détails
//! et timelines (match-v5), stockés dans PostgreSQL avec une file de travaux reprenable.

pub mod aggregation;
pub mod campaign;
pub mod catalog;
pub mod collector;
pub mod config;
pub mod model;
pub mod rate_limit;
pub mod report;
pub mod riot_client;
pub mod shared_quota;
pub mod static_data;
pub mod storage;
mod transaction;
