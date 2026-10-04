//! Assemblage HTTP, bornes de ressources et authentification de l'API.
use crate::auth::Auth;
use crate::profiles::Profiles;
use crate::realtime::Publication;
use crate::{error::ApiError, profiles::HistoryQuery, query::StatsQuery};
use axum::{
    extract::{
        rejection::{PathRejection, QueryRejection},
        DefaultBodyLimit, Path, Query, State,
    },
    http::{HeaderMap, HeaderValue},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use olc_collector::{riot_client::HttpsTransport, shared_quota::CoordinatedTransport};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Semaphore};
use tower_http::cors::CorsLayer;

pub type LiveProfiles = Profiles<CoordinatedTransport<HttpsTransport>>;
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth: Arc<Auth>,
    pub profiles: Option<Arc<LiveProfiles>>,
    pub publications: watch::Sender<Publication>,
    pub shutdown: watch::Sender<bool>,
    pub ws_slots: Arc<Semaphore>,
    pub http_slots: Arc<Semaphore>,
    pub allowed_origins: Vec<axum::http::HeaderValue>,
    /// Sujets de jeton autorisés à exporter ou effacer les données d'un joueur (#99).
    pub privacy_operators: Vec<String>,
}
impl AppState {
    pub fn new(pool: PgPool, auth: Auth) -> Self {
        Self {
            pool,
            auth: Arc::new(auth),
            profiles: None,
            publications: watch::channel(Publication::default()).0,
            shutdown: watch::channel(false).0,
            ws_slots: Arc::new(Semaphore::new(128)),
            http_slots: Arc::new(Semaphore::new(32)),
            allowed_origins: vec![],
            privacy_operators: vec![],
        }
    }
}
/// Routeur complet, sans ouvrir de socket ni déclencher de requête Riot.
pub fn router(state: AppState) -> Router {
    let private = Router::new()
        .route("/v1/tierlist", get(tierlist))
        .route("/v1/builds/{champion_id}", get(builds))
        .route("/v1/profiles/{platform}/{name}/{tag}", get(profile))
        .route("/v1/profiles/{platform}/{name}/{tag}/matches", get(history))
        .route("/v1/privacy/export", post(crate::privacy::export))
        .route("/v1/privacy/erase", post(crate::privacy::erase))
        .route_layer(middleware::from_fn_with_state(state.clone(), authenticate));
    let cors = CorsLayer::new()
        .allow_origin(state.allowed_origins.clone())
        .allow_methods([axum::http::Method::GET, axum::http::Method::OPTIONS])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::IF_NONE_MATCH,
        ])
        .expose_headers([axum::http::header::ETAG]);
    Router::new()
        .merge(private)
        .route("/health", get(health))
        .route("/v1/static/manifest", get(manifest))
        .route("/v1/static/{version}/{locale}/{*resource}", get(document))
        .route(
            "/v1/catalog/{version}/manifest",
            get(crate::catalog::manifest),
        )
        .route(
            "/v1/catalog/{version}/{locale}/{kind}",
            get(crate::catalog::list),
        )
        .route(
            "/v1/catalog/{version}/{locale}/{kind}/{id}",
            get(crate::catalog::detail),
        )
        .route("/v1/catalog-diff", get(crate::catalog::diff))
        .route("/v1/ws", get(crate::realtime::upgrade))
        .fallback(|| async { ApiError::NotFound })
        .layer(DefaultBodyLimit::max(8192))
        .layer(middleware::from_fn_with_state(state.clone(), limits))
        .layer(cors)
        .with_state(state)
}
async fn authenticate(
    State(state): State<AppState>,
    mut request: axum::extract::Request,
    next: Next,
) -> Response {
    let claims = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .and_then(|token| state.auth.verify(token).ok());
    let Some(claims) = claims else {
        return ApiError::Unauthorized.into_response();
    };
    // Les routes RGPD vérifient le sujet du jeton.
    request.extensions_mut().insert(claims);
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    response
}
async fn limits(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let Ok(_permit) = state.http_slots.try_acquire() else {
        return ApiError::RateLimited.into_response();
    };
    if request.uri().to_string().len() > 2048 {
        return ApiError::InvalidRequest.into_response();
    }
    match tokio::time::timeout(Duration::from_secs(45), next.run(request)).await {
        Ok(response) => response,
        Err(_) => ApiError::Unavailable.into_response(),
    }
}
async fn health(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"status":"ok"})))
}
async fn tierlist(
    State(state): State<AppState>,
    query: Result<Query<StatsQuery>, QueryRejection>,
) -> Result<Json<crate::stats::TierlistResponse>, ApiError> {
    Ok(Json(
        crate::stats::tierlist(&state.pool, query.map_err(|_| ApiError::InvalidRequest)?.0).await?,
    ))
}
async fn builds(
    State(state): State<AppState>,
    path: Result<Path<u32>, PathRejection>,
    query: Result<Query<StatsQuery>, QueryRejection>,
) -> Result<Json<crate::stats::BuildsResponse>, ApiError> {
    Ok(Json(
        crate::stats::builds(
            &state.pool,
            query.map_err(|_| ApiError::InvalidRequest)?.0,
            path.map_err(|_| ApiError::InvalidRequest)?.0,
        )
        .await?,
    ))
}
type ProfilePath = Result<Path<(String, String, String)>, PathRejection>;
async fn profile(
    State(state): State<AppState>,
    path: ProfilePath,
) -> Result<Json<crate::profiles::Profile>, ApiError> {
    let Path((platform, name, tag)) = path.map_err(|_| ApiError::InvalidRequest)?;
    let service = state.profiles.as_ref().ok_or(ApiError::Unavailable)?;
    Ok(Json(service.profile(&platform, &name, &tag).await?))
}
async fn history(
    State(state): State<AppState>,
    path: ProfilePath,
    query: Result<Query<HistoryQuery>, QueryRejection>,
) -> Result<Json<crate::profiles::ProfileMatches>, ApiError> {
    let Path((platform, name, tag)) = path.map_err(|_| ApiError::InvalidRequest)?;
    let service = state.profiles.as_ref().ok_or(ApiError::Unavailable)?;
    Ok(Json(
        service
            .history(
                &platform,
                &name,
                &tag,
                query.map_err(|_| ApiError::InvalidRequest)?.0,
            )
            .await?,
    ))
}
async fn manifest(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ApiError> {
    crate::static_data::manifest(&state.pool, &headers).await
}
async fn document(
    State(state): State<AppState>,
    path: Result<Path<(String, String, String)>, PathRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Path((version, locale, resource)) = path.map_err(|_| ApiError::InvalidRequest)?;
    crate::static_data::document(&state.pool, &version, &locale, &resource, &headers).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;
    type Response = axum::response::Response;

    fn state() -> AppState {
        AppState::new(
            PgPoolOptions::new()
                .connect_lazy("postgres://localhost/unused")
                .unwrap(),
            Auth::new(b"synthetic-signing-material-for-http-tests", "test", "api").unwrap(),
        )
    }
    #[tokio::test]
    async fn cors_autorise_la_revalidation_et_expose_etag_au_site() {
        let mut state = state();
        state
            .allowed_origins
            .push("http://localhost:3000".parse().unwrap());
        let app = router(state);
        let preflight = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/v1/static/16.19.1/fr_FR/item.json")
                    .header("origin", "http://localhost:3000")
                    .header("access-control-request-method", "GET")
                    .header("access-control-request-headers", "if-none-match")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(preflight.headers()["access-control-allow-headers"]
            .to_str()
            .unwrap()
            .contains("if-none-match"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/static/invalid/fr_FR/item.json")
                    .header("origin", "http://localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.headers()["access-control-expose-headers"], "etag");
    }

    #[tokio::test]
    async fn les_routes_privees_exigent_le_jwt_avant_toute_requete_sql() {
        let app = router(state());
        for path in [
            "/v1/tierlist",
            "/v1/builds/1",
            "/v1/profiles/EUW1/name/tag",
            "/v1/profiles/EUW1/name/tag/matches",
        ] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
            assert_eq!(response.headers()["cache-control"], "no-store");
            let body = to_bytes(response.into_body(), 4096).await.unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"]["code"],
                "unauthorized"
            );
        }
    }
    async fn post_privacy(app: &Router, path: &str, token: Option<&str>, body: &str) -> Response {
        let mut builder = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json");
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        app.clone()
            .oneshot(builder.body(Body::from(body.to_owned())).unwrap())
            .await
            .unwrap()
    }

    async fn error_code(response: Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"]["code"].clone()
    }

    #[tokio::test]
    async fn les_routes_rgpd_sont_reservees_aux_operateurs_avant_toute_requete_sql() {
        let mut state = state();
        state.privacy_operators.push("dpo".into());
        let now = jsonwebtoken::get_current_timestamp();
        let session = state.auth.issue("session", now, 60).unwrap();
        let operator = state.auth.issue("dpo", now, 60).unwrap();
        let app = router(state);
        for path in ["/v1/privacy/export", "/v1/privacy/erase"] {
            let body = r#"{"puuid":"fake-puuid-1"}"#;
            let response = post_privacy(&app, path, None, body).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
            // Un jeton valide ne suffit pas : la base paresseuse n'est jamais contactée.
            let response = post_privacy(&app, path, Some(&session), body).await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert_eq!(error_code(response).await, "forbidden");
            for invalid in [
                r#"{"puuid":"BOT"}"#,
                r#"{"puuid":""}"#,
                r#"{"id":"x"}"#,
                "pas du json",
            ] {
                let response = post_privacy(&app, path, Some(&operator), invalid).await;
                assert_eq!(
                    response.status(),
                    StatusCode::BAD_REQUEST,
                    "{path} {invalid}"
                );
                assert_eq!(error_code(response).await, "invalid_request");
            }
        }
    }

    #[tokio::test]
    async fn sans_operateur_configure_personne_n_accede_aux_routes_rgpd() {
        let state = state();
        let token = state
            .auth
            .issue("dpo", jsonwebtoken::get_current_timestamp(), 60)
            .unwrap();
        let response = post_privacy(
            &router(state),
            "/v1/privacy/export",
            Some(&token),
            r#"{"puuid":"fake-puuid-1"}"#,
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn les_parametres_invalides_restent_des_erreurs_json_et_le_statique_est_public() {
        let state = state();
        let token = state
            .auth
            .issue("session", jsonwebtoken::get_current_timestamp(), 60)
            .unwrap();
        let app = router(state);
        for (path, auth) in [
            ("/v1/tierlist?patch=invalid", true),
            ("/v1/builds/not-an-id", true),
            ("/v1/static/no-version/fr_FR/item.json", false),
        ] {
            let mut builder = Request::builder().uri(path);
            if auth {
                builder = builder.header("authorization", format!("Bearer {token}"));
            }
            let response = app
                .clone()
                .oneshot(builder.body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
            let body = to_bytes(response.into_body(), 4096).await.unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"]["code"],
                "invalid_request"
            );
        }
    }
}
