//! Erreurs publiques stables, sans contenu Riot ni détail d'infrastructure.
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiError {
    InvalidRequest,
    Unauthorized,
    NotFound,
    Unavailable,
    RateLimited,
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorCode,
}
#[derive(Serialize)]
struct ErrorCode {
    code: &'static str,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::InvalidRequest => (StatusCode::BAD_REQUEST, "invalid_request"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
            Self::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
        };
        let mut response = (
            status,
            Json(ErrorBody {
                error: ErrorCode { code },
            }),
        )
            .into_response();
        response.headers_mut().insert(
            "cache-control",
            axum::http::HeaderValue::from_static("no-store"),
        );
        response
    }
}
impl From<sqlx::Error> for ApiError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
