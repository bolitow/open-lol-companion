//! Connexion au client League of Legends (LCU) sur Windows et macOS.
//!
//! Le client écrit un fichier `lockfile` au format `nom:pid:port:mot_de_passe:protocole`
//! pendant qu'il tourne. Ce crate le trouve (ou lit les arguments du processus
//! `LeagueClientUx` en secours) et en déduit l'URL et l'en-tête d'authentification
//! de l'API locale. Aucune donnée ne sort de la machine.
//!
//! Voir la section 4.3 du cahier des charges.

mod credentials;
mod discovery;
mod gameflow;

pub use credentials::{Credentials, ParseError};
pub use discovery::{default_lockfile_paths, discover, DiscoveryError, LOCKFILE_ENV};
pub use gameflow::GameflowPhase;

/// Adresse de la Live Client Data API, disponible uniquement pendant une partie.
pub const LIVE_CLIENT_DATA_URL: &str = "https://127.0.0.1:2999/liveclientdata";
