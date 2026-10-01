//! Imports explicites dans le client LoL (cahier des charges, section 5.4).

mod runes;

pub use runes::ImportRunesRequest;

use serde::Serialize;
use thiserror::Error;

use crate::ClientError;

const APP_NAME: &str = "Open LoL Companion";

/// Codes stables traduits par l'interface, sans réponse brute ni identifiants LCU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Error)]
#[serde(rename_all = "camelCase")]
pub enum ImportError {
    #[error("client LoL indisponible")]
    ClientUnavailable,
    #[error("import refusé par le client LoL")]
    ClientRejected,
    #[error("réponse du client LoL invalide")]
    InvalidClientData,
    #[error("page de runes invalide")]
    InvalidRunes,
    #[error("page de runes de l'application indisponible ou ambiguë")]
    RunePageUnavailable,
}

impl From<ClientError> for ImportError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::UnexpectedStatus(_) => Self::ClientRejected,
            ClientError::Http(error) if error.is_decode() => Self::InvalidClientData,
            ClientError::Http(error) if error.is_status() => Self::ClientRejected,
            _ => Self::ClientUnavailable,
        }
    }
}
