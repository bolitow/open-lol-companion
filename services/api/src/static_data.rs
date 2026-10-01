//! Données publiques et revalidation HTTP des versions Data Dragon corrigibles.
use crate::error::ApiError;
use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

/// Manifeste public du cache FR/EN.
pub async fn manifest(pool: &PgPool, headers: &HeaderMap) -> Result<Response, ApiError> {
    let row = sqlx::query("SELECT live_version,checked_at::text,versions,catalogs FROM static_data_manifest WHERE id=1").fetch_optional(pool).await?.ok_or(ApiError::Unavailable)?;
    let data = StaticManifest {
        live_version: row.try_get("live_version")?,
        checked_at: row.try_get("checked_at")?,
        versions: row
            .try_get::<sqlx::types::Json<Vec<String>>, _>("versions")?
            .0,
        catalogs: row.try_get("catalogs")?,
    };
    cached_response(
        &data,
        headers,
        "public, max-age=60, s-maxage=300, stale-while-revalidate=60",
    )
}
/// Document exact d'une release stockée ; aucun chemin disque ni URL arbitraire.
pub async fn document(
    pool: &PgPool,
    version: &str,
    locale: &str,
    resource: &str,
    headers: &HeaderMap,
) -> Result<Response, ApiError> {
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || p.len() > 4 || !p.bytes().all(|b| b.is_ascii_digit()))
        || !["fr_FR", "en_US"].contains(&locale)
        || !valid_resource(resource)
    {
        return Err(ApiError::InvalidRequest);
    }
    let key = format!("{locale}/{resource}");
    let value: Option<Value> = sqlx::query_scalar(
        "SELECT bundle->'documents'->$2->'data' FROM static_data_releases WHERE version=$1",
    )
    .bind(version)
    .bind(key)
    .fetch_optional(pool)
    .await?
    .flatten();
    let value = value.filter(|v| !v.is_null()).ok_or(ApiError::NotFound)?;
    cached_response(
        &value,
        headers,
        "public, max-age=3600, s-maxage=3600, stale-while-revalidate=300",
    )
}

#[derive(Serialize)]
pub struct StaticManifest {
    pub live_version: String,
    pub checked_at: String,
    pub versions: Vec<String>,
    pub catalogs: Value,
}

fn valid_resource(resource: &str) -> bool {
    if [
        "champion.json",
        "item.json",
        "summoner.json",
        "runesReforged.json",
        "map.json",
        "profileicon.json",
        "mode/classic/champion.json",
    ]
    .contains(&resource)
    {
        return true;
    }
    let name = resource
        .strip_prefix("champion/")
        .or_else(|| resource.strip_prefix("mode/classic/champion/"));
    name.and_then(|s| s.strip_suffix(".json")).is_some_and(|s| {
        !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    })
}

fn cached_response(
    value: &impl Serialize,
    headers: &HeaderMap,
    cache: &'static str,
) -> Result<Response, ApiError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ApiError::Unavailable)?;
    let tag = format!("\"{:x}\"", Sha256::digest(&bytes));
    let matched = headers
        .get_all("if-none-match")
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(','))
        .any(|candidate| {
            let candidate = candidate.trim();
            candidate == "*" || candidate.strip_prefix("W/").unwrap_or(candidate) == tag
        });
    let mut response = if matched {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        Response::new(Body::from(bytes))
    };
    response.headers_mut().insert(
        "etag",
        HeaderValue::from_str(&tag).map_err(|_| ApiError::Unavailable)?,
    );
    response
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static(cache));
    response
        .headers_mut()
        .insert("content-type", HeaderValue::from_static("application/json"));
    Ok(response)
}
