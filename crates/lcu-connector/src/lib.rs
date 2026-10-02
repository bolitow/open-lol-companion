//! Connexion au client League of Legends (LCU) sur Windows et macOS.
//!
//! Le client écrit un fichier `lockfile` au format `nom:pid:port:mot_de_passe:protocole`
//! pendant qu'il tourne. Ce crate le trouve (ou lit les arguments du processus
//! `LeagueClientUx` en secours) et en déduit l'URL et l'en-tête d'authentification
//! de l'API locale, puis suit le client via HTTPS et le WebSocket (WAMP), avec
//! reconnexion automatique. Aucune donnée ne sort de la machine.
//!
//! Voir la section 4.3 du cahier des charges.

mod client;
mod credentials;
mod discovery;
mod draft;
pub use draft::{DraftSession, DRAFT_ENDPOINT};
mod gameflow;
pub mod imports;
mod runes;
mod session;
#[cfg(test)]
mod test_support;
pub use runes::{RunePage, RUNES_ENDPOINT};
mod wamp;
mod watcher;
pub use session::LcuSession;

pub use client::{ClientError, LcuClient};
pub use credentials::{Credentials, ParseError};
pub use discovery::{default_lockfile_paths, discover, DiscoveryError, LOCKFILE_ENV};
pub use gameflow::GameflowPhase;
pub use wamp::JsonApiEvent;
pub use watcher::{watch, LcuEvent, RETRY_DELAY};

/// Adresse de la Live Client Data API, disponible uniquement pendant une partie.
pub const LIVE_CLIENT_DATA_URL: &str = "https://127.0.0.1:2999/liveclientdata";
