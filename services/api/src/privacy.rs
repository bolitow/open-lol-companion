//! Export et effacement des données d'un joueur sur demande (#99).
//!
//! Réservés aux sujets de jeton listés dans `OLC_API_PRIVACY_OPERATORS` : un jeton
//! d'accès ordinaire ne prouve pas l'identité Riot de l'appelant. Le PUUID circule
//! dans le corps JSON, jamais dans l'URL (journaux de proxy).
use crate::{auth::Claims, error::ApiError, server::AppState};
use axum::{
    extract::{rejection::JsonRejection, State},
    Extension, Json,
};
use olc_collector::privacy::{self, PrivacyError, SubjectErasure, SubjectExport};
use olc_collector::storage::Storage;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyRequest {
    pub puuid: String,
}

/// Contrôle l'habilitation avant de lire le corps ou de toucher à la base.
fn subject(
    state: &AppState,
    claims: Option<Extension<Claims>>,
    body: Result<Json<PrivacyRequest>, JsonRejection>,
) -> Result<String, ApiError> {
    let Extension(claims) = claims.ok_or(ApiError::Unauthorized)?;
    if !state.privacy_operators.contains(&claims.sub) {
        return Err(ApiError::Forbidden);
    }
    let Json(request) = body.map_err(|_| ApiError::InvalidRequest)?;
    if !privacy::is_valid_puuid(&request.puuid) {
        return Err(ApiError::InvalidRequest);
    }
    Ok(request.puuid)
}

impl From<PrivacyError> for ApiError {
    fn from(error: PrivacyError) -> Self {
        match error {
            PrivacyError::InvalidSubject | PrivacyError::InvalidRetention => Self::InvalidRequest,
            PrivacyError::Database(_) => Self::Unavailable,
        }
    }
}

pub async fn export(
    State(state): State<AppState>,
    claims: Option<Extension<Claims>>,
    body: Result<Json<PrivacyRequest>, JsonRejection>,
) -> Result<Json<SubjectExport>, ApiError> {
    let puuid = subject(&state, claims, body)?;
    let storage = Storage::from_pool(state.pool.clone());
    Ok(Json(privacy::export_subject(&storage, &puuid).await?))
}

pub async fn erase(
    State(state): State<AppState>,
    claims: Option<Extension<Claims>>,
    body: Result<Json<PrivacyRequest>, JsonRejection>,
) -> Result<Json<SubjectErasure>, ApiError> {
    let puuid = subject(&state, claims, body)?;
    let storage = Storage::from_pool(state.pool.clone());
    Ok(Json(privacy::erase_subject(&storage, &puuid).await?))
}
