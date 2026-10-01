//! Imports explicites dans le client LoL (cahier des charges, section 5.4).

mod items;
mod runes;
mod spells;

pub use items::{ImportItemsRequest, ItemBlock, ItemStack};
pub use runes::ImportRunesRequest;
pub use spells::{FlashSlot, ImportSpellsRequest};

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
    #[error("sélection des champions inactive")]
    NotInChampSelect,
    #[error("page de runes invalide")]
    InvalidRunes,
    #[error("sorts d'invocateur invalides")]
    InvalidSpells,
    #[error("set d'items invalide")]
    InvalidItems,
    #[error("page de runes de l'application indisponible ou ambiguë")]
    RunePageUnavailable,
    #[error("priorité du set d'items indisponible")]
    ItemSetPriorityUnavailable,
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
