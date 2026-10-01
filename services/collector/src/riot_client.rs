//! Client Riot Web API : routage, construction des requêtes, transport HTTPS et
//! classement des réponses en erreurs exploitables.
//!
//! Catalogue officiel vérifié le 01/10/2026 : league-v4 (pages par rang, listes
//! Master/Grandmaster/Challenger et classement par PUUID) sur la plateforme ;
//! match-v5 (historiques, détails et timelines) sur sa région de routage.
//! Sources : <https://developer.riotgames.com/api-details/league-v4>,
//! <https://developer.riotgames.com/api-details/match-v5>.

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Url;
use rustls::crypto::ring;
use rustls::{ClientConfig, RootCertStore};
use thiserror::Error;

use crate::config::{ApiKey, ConfigError, Division, Tier, RANKED_SOLO_QUEUE, RANKED_SOLO_QUEUE_ID};
use crate::rate_limit::{parse_retry_after, Governor, LimitScope, RateHeaders};

/// Valeur de routage : plateforme (league-v4) ou région (match-v5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    Euw1,
    Europe,
    Americas,
    Asia,
    Sea,
    Platform(&'static str),
}

impl Route {
    pub fn host(self) -> String {
        let name = match self {
            Route::Euw1 => "euw1",
            Route::Europe => "europe",
            Route::Americas => "americas",
            Route::Asia => "asia",
            Route::Sea => "sea",
            Route::Platform(name) => name,
        };
        format!("{name}.api.riotgames.com")
    }

    /// Routages officiels : catalogue match-v5, vérifié le 01/10/2026.
    pub fn for_platform(platform: &str) -> Result<(Self, Self), ConfigError> {
        let (name, region) = match platform {
            "EUW1" => return Ok((Self::Euw1, Self::Europe)),
            "EUN1" => ("eun1", Self::Europe),
            "ME1" => ("me1", Self::Europe),
            "TR1" => ("tr1", Self::Europe),
            "RU" => ("ru", Self::Europe),
            "NA1" => ("na1", Self::Americas),
            "BR1" => ("br1", Self::Americas),
            "LA1" => ("la1", Self::Americas),
            "LA2" => ("la2", Self::Americas),
            "KR" => ("kr", Self::Asia),
            "JP1" => ("jp1", Self::Asia),
            "OC1" => ("oc1", Self::Sea),
            "SG2" => ("sg2", Self::Sea),
            "TW2" => ("tw2", Self::Sea),
            "VN2" => ("vn2", Self::Sea),
            _ => return Err(ConfigError::Invalid("plateforme inconnue")),
        };
        Ok((Self::Platform(name), region))
    }
}

/// Méthode Riot, clé des quotas par méthode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endpoint {
    LeagueEntries,
    ApexLeague,
    ParticipantRanks,
    MatchIdsByPuuid,
    Match,
    Timeline,
}

impl Endpoint {
    pub fn route(self) -> Route {
        match self {
            Endpoint::LeagueEntries | Endpoint::ApexLeague | Endpoint::ParticipantRanks => {
                Route::Euw1
            }
            Endpoint::MatchIdsByPuuid | Endpoint::Match | Endpoint::Timeline => Route::Europe,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Endpoint::LeagueEntries => "league-v4 entries",
            Endpoint::ApexLeague => "league-v4 apex",
            Endpoint::ParticipantRanks => "league-v4 ranks",
            Endpoint::MatchIdsByPuuid => "match-v5 ids",
            Endpoint::Match => "match-v5 match",
            Endpoint::Timeline => "match-v5 timeline",
        }
    }
}

/// Requête GET vers Riot : segments de chemin (encodés à la construction de l'URL)
/// et paramètres de requête.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub endpoint: Endpoint,
    pub route: Route,
    pub segments: Vec<String>,
    pub query: Vec<(&'static str, String)>,
}

impl Request {
    /// Page (à partir de 1) d'un classement Ranked Solo/Duo.
    pub fn league_entries(tier: Tier, division: Division, page: u32) -> Self {
        Self {
            endpoint: Endpoint::LeagueEntries,
            route: Route::Euw1,
            segments: segs(&[
                "lol",
                "league",
                "v4",
                "entries",
                RANKED_SOLO_QUEUE,
                tier.as_str(),
                division.as_str(),
            ]),
            query: vec![("page", page.to_string())],
        }
    }

    /// Identifiants de parties Ranked Solo/Duo d'un joueur, bornés dans le temps
    /// (secondes Unix). `start` commence à 0, `count` est limité à 100 par Riot.
    pub fn match_ids(puuid: &str, start: u32, count: u32, start_time: i64, end_time: i64) -> Self {
        Self {
            endpoint: Endpoint::MatchIdsByPuuid,
            route: Route::Europe,
            segments: segs(&["lol", "match", "v5", "matches", "by-puuid", puuid, "ids"]),
            query: vec![
                ("startTime", start_time.to_string()),
                ("endTime", end_time.to_string()),
                ("queue", RANKED_SOLO_QUEUE_ID.to_string()),
                ("type", "ranked".to_owned()),
                ("start", start.to_string()),
                ("count", count.min(100).to_string()),
            ],
        }
    }

    pub fn match_detail(match_id: &str) -> Self {
        Self {
            endpoint: Endpoint::Match,
            route: Route::Europe,
            segments: segs(&["lol", "match", "v5", "matches", match_id]),
            query: vec![],
        }
    }

    pub fn timeline(match_id: &str) -> Self {
        Self {
            endpoint: Endpoint::Timeline,
            route: Route::Europe,
            segments: segs(&["lol", "match", "v5", "matches", match_id, "timeline"]),
            query: vec![],
        }
    }

    pub fn league_entries_for(
        platform: &str,
        tier: Tier,
        division: Division,
        page: u32,
    ) -> Result<Self, ConfigError> {
        let route = Route::for_platform(platform)?.0;
        if tier.is_apex() {
            let league = match tier {
                Tier::Master => "masterleagues",
                Tier::Grandmaster => "grandmasterleagues",
                _ => "challengerleagues",
            };
            return Ok(Self {
                endpoint: Endpoint::ApexLeague,
                route,
                segments: segs(&["lol", "league", "v4", league, "by-queue", RANKED_SOLO_QUEUE]),
                query: vec![],
            });
        }
        let mut request = Self::league_entries(tier, division, page);
        request.route = route;
        Ok(request)
    }

    pub fn match_ids_for(
        platform: &str,
        queue_id: i32,
        puuid: &str,
        start: u32,
        count: u32,
        start_time: i64,
        end_time: i64,
    ) -> Result<Self, ConfigError> {
        let mut request = Self::match_ids(puuid, start, count, start_time, end_time);
        request.route = Route::for_platform(platform)?.1;
        request
            .query
            .retain(|(name, _)| !matches!(*name, "queue" | "type"));
        if queue_id > 0 {
            request.query.push(("queue", queue_id.to_string()));
        }
        Ok(request)
    }

    pub fn match_detail_for(platform: &str, match_id: &str) -> Result<Self, ConfigError> {
        let mut request = Self::match_detail(match_id);
        request.route = Route::for_platform(platform)?.1;
        Ok(request)
    }

    pub fn timeline_for(platform: &str, match_id: &str) -> Result<Self, ConfigError> {
        let mut request = Self::timeline(match_id);
        request.route = Route::for_platform(platform)?.1;
        Ok(request)
    }

    /// Source : https://developer.riotgames.com/api-details/league-v4 (01/10/2026).
    pub fn participant_ranks(platform: &str, puuid: &str) -> Result<Self, ConfigError> {
        Ok(Self {
            endpoint: Endpoint::ParticipantRanks,
            route: Route::for_platform(platform)?.0,
            segments: segs(&["lol", "league", "v4", "entries", "by-puuid", puuid]),
            query: vec![],
        })
    }

    /// URL complète. Contient parfois un PUUID : ne jamais la journaliser.
    pub fn url(&self, base: &Url) -> Url {
        let mut url = base.clone();
        if let Ok(mut path) = url.path_segments_mut() {
            path.clear().extend(&self.segments);
        }
        if !self.query.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(self.query.iter().map(|(k, v)| (*k, v.as_str())));
        }
        url
    }
}

fn segs(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| (*s).to_owned()).collect()
}

/// Réponse brute, en-têtes en minuscules.
#[derive(Debug, Clone, Default)]
pub struct RawResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl RawResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn rate_headers(&self) -> RateHeaders<'_> {
        RateHeaders {
            app_limit: self.header("x-app-rate-limit"),
            app_count: self.header("x-app-rate-limit-count"),
            method_limit: self.header("x-method-rate-limit"),
            method_count: self.header("x-method-rate-limit-count"),
        }
    }
}

/// Erreur de transport, sans URL (elle peut contenir un PUUID) ni clé.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TransportError {
    #[error("délai dépassé")]
    Timeout,
    #[error("erreur réseau : {0}")]
    Network(String),
}

/// Envoi d'une requête : HTTPS en production, simulé dans les tests.
pub trait Transport: Send + Sync + 'static {
    fn send(
        &self,
        request: &Request,
    ) -> impl Future<Output = Result<RawResponse, TransportError>> + Send;
}

#[derive(Debug, Clone, Error, PartialEq)]
pub enum RiotError {
    #[error("quota Riot atteint ({scope:?}), pause de {pause:?}")]
    RateLimited { scope: LimitScope, pause: Duration },
    #[error("clé Riot refusée (HTTP {0}) : clé expirée, invalide ou sans accès à cette API")]
    Unauthorized(u16),
    #[error("ressource introuvable (HTTP 404)")]
    NotFound,
    #[error("requête refusée par Riot (HTTP 400)")]
    BadRequest,
    #[error("erreur serveur Riot (HTTP {0})")]
    Server(u16),
    #[error("réponse HTTP inattendue ({0})")]
    Unexpected(u16),
    #[error("transport : {0}")]
    Transport(TransportError),
    #[error("réponse invalide : {0}")]
    InvalidBody(String),
}

/// Classe une réponse non-429 selon son statut HTTP.
pub fn classify_status(status: u16) -> Result<(), RiotError> {
    match status {
        200..=299 => Ok(()),
        400 => Err(RiotError::BadRequest),
        401 | 403 => Err(RiotError::Unauthorized(status)),
        404 => Err(RiotError::NotFound),
        500..=599 => Err(RiotError::Server(status)),
        _ => Err(RiotError::Unexpected(status)),
    }
}

/// Client Riot : chaque appel passe par le gouverneur de quotas.
pub struct RiotClient<T: Transport> {
    transport: T,
    governor: Arc<Governor>,
    default_rate_limit_pause: Duration,
    calls: AtomicU64,
}

impl<T: Transport> RiotClient<T> {
    pub fn new(transport: T, governor: Arc<Governor>, default_rate_limit_pause: Duration) -> Self {
        Self {
            transport,
            governor,
            default_rate_limit_pause,
            calls: AtomicU64::new(0),
        }
    }

    /// Appels envoyés depuis la création du client.
    pub fn calls(&self) -> u64 {
        self.calls.load(Ordering::Relaxed)
    }

    /// Envoie la requête et renvoie le corps d'une réponse 2xx.
    pub async fn get(&self, request: &Request) -> Result<Vec<u8>, RiotError> {
        self.governor
            .acquire_on(request.route, request.endpoint)
            .await;
        self.calls.fetch_add(1, Ordering::Relaxed);
        let res = self
            .transport
            .send(request)
            .await
            .map_err(RiotError::Transport)?;
        self.governor
            .observe_on(request.route, request.endpoint, &res.rate_headers())
            .await;
        if res.status == 429 {
            let scope = LimitScope::from_header(res.header("x-rate-limit-type"));
            let retry_after = parse_retry_after(res.header("retry-after"));
            let pause = self
                .governor
                .on_rate_limited_on(
                    request.route,
                    request.endpoint,
                    scope,
                    retry_after,
                    self.default_rate_limit_pause,
                )
                .await;
            return Err(RiotError::RateLimited { scope, pause });
        }
        classify_status(res.status)?;
        Ok(res.body)
    }
}

/// Transport HTTPS réel, avec validation normale des certificats publics.
pub struct HttpsTransport {
    http: reqwest::Client,
    base_urls: std::collections::HashMap<Route, Url>,
}

#[derive(Debug, Error)]
pub enum HttpsSetupError {
    #[error("configuration TLS invalide : {0}")]
    Tls(#[from] rustls::Error),
    #[error("client HTTP : {0}")]
    Http(String),
    #[error("la clé contient des caractères invalides pour un en-tête HTTP")]
    InvalidKey,
}

impl HttpsTransport {
    pub fn new(api_key: &ApiKey, timeout: Duration) -> Result<Self, HttpsSetupError> {
        let mut key =
            HeaderValue::from_str(api_key.expose()).map_err(|_| HttpsSetupError::InvalidKey)?;
        // Masqué dans les journaux des librairies.
        key.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert("X-Riot-Token", key);

        let http = reqwest::Client::builder()
            .use_preconfigured_tls(public_tls_config()?)
            .default_headers(headers)
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| HttpsSetupError::Http(e.without_url().to_string()))?;
        let base = |route: Route| {
            Url::parse(&format!("https://{}/", route.host()))
                .map_err(|e| HttpsSetupError::Http(e.to_string()))
        };
        let mut base_urls = std::collections::HashMap::new();
        for platform in crate::config::PLATFORMS {
            let (local, regional) =
                Route::for_platform(platform).map_err(|e| HttpsSetupError::Http(e.to_string()))?;
            base_urls.insert(local, base(local)?);
            base_urls.insert(regional, base(regional)?);
        }
        Ok(Self { http, base_urls })
    }

    fn base(&self, route: Route) -> Result<&Url, TransportError> {
        self.base_urls
            .get(&route)
            .ok_or_else(|| TransportError::Network("routage invalide".into()))
    }
}

/// TLS pour les API publiques : racines Mozilla embarquées (`webpki-roots`) et
/// fournisseur `ring`, indépendamment du vérificateur propre au client LoL local.
pub fn public_tls_config() -> Result<ClientConfig, rustls::Error> {
    let roots = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    Ok(
        ClientConfig::builder_with_provider(Arc::new(ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

impl Transport for HttpsTransport {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        let url = request.url(self.base(request.route)?);
        let res = self.http.get(url).send().await.map_err(map_reqwest)?;
        let status = res.status().as_u16();
        let headers = res
            .headers()
            .iter()
            .filter_map(|(k, v)| Some((k.as_str().to_owned(), v.to_str().ok()?.to_owned())))
            .collect();
        let body = res.bytes().await.map_err(map_reqwest)?.to_vec();
        Ok(RawResponse {
            status,
            headers,
            body,
        })
    }
}

fn map_reqwest(e: reqwest::Error) -> TransportError {
    if e.is_timeout() {
        return TransportError::Timeout;
    }
    // Causes en chaîne (proxy, TLS, DNS…), sans l'URL qui peut contenir un PUUID.
    let e = e.without_url();
    let mut message = e.to_string();
    let mut source = std::error::Error::source(&e);
    while let Some(cause) = source {
        message.push_str(" : ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    TransportError::Network(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_limit::RateLimiter;
    use std::sync::Mutex;

    fn base(route: Route) -> Url {
        Url::parse(&format!("https://{}/", route.host())).unwrap()
    }

    #[test]
    fn route_les_requetes_vers_la_plateforme_et_sa_region() {
        for (platform, region) in [
            ("NA1", "americas"),
            ("KR", "asia"),
            ("SG2", "sea"),
            ("ME1", "europe"),
        ] {
            let r = Request::match_ids_for(platform, 0, "fake", 100, 100, 1, 2).unwrap();
            assert_eq!(r.route.host(), format!("{region}.api.riotgames.com"));
            assert!(!r
                .query
                .iter()
                .any(|(name, _)| matches!(*name, "queue" | "type")));
            let ranked = Request::match_ids_for(platform, 440, "fake", 0, 100, 1, 2).unwrap();
            assert!(ranked.query.contains(&("queue", "440".into())));
            let ranks = Request::participant_ranks(platform, "fake").unwrap();
            assert_eq!(
                ranks.route.host(),
                format!("{}.api.riotgames.com", platform.to_lowercase())
            );
        }
        assert!(Request::match_detail_for("PH2", "PH2_1").is_err());
    }

    #[test]
    fn les_listes_apex_ne_sont_pas_paginees() {
        let request = Request::league_entries_for("KR", Tier::Master, Division::I, 1).unwrap();
        assert!(request.query.is_empty());
        assert!(request.segments.iter().any(|s| s == "masterleagues"));
    }

    #[test]
    fn league_v4_vise_euw1_avec_la_page() {
        let r = Request::league_entries(Tier::Gold, Division::II, 3);
        assert_eq!(r.endpoint.route(), Route::Euw1);
        assert_eq!(
            r.url(&base(Route::Euw1)).as_str(),
            "https://euw1.api.riotgames.com/lol/league/v4/entries/RANKED_SOLO_5x5/GOLD/II?page=3"
        );
    }

    #[test]
    fn l_historique_vise_europe_avec_file_fenetre_et_pagination() {
        let r = Request::match_ids("abc-DEF_123", 100, 250, 1_700_000_000, 1_701_209_600);
        assert_eq!(r.endpoint.route(), Route::Europe);
        assert_eq!(
            r.url(&base(Route::Europe)).as_str(),
            "https://europe.api.riotgames.com/lol/match/v5/matches/by-puuid/abc-DEF_123/ids\
             ?startTime=1700000000&endTime=1701209600&queue=420&type=ranked&start=100&count=100"
        );
    }

    #[test]
    fn detail_et_timeline_visent_europe() {
        assert_eq!(
            Request::match_detail("EUW1_42")
                .url(&base(Route::Europe))
                .as_str(),
            "https://europe.api.riotgames.com/lol/match/v5/matches/EUW1_42"
        );
        assert_eq!(
            Request::timeline("EUW1_42")
                .url(&base(Route::Europe))
                .as_str(),
            "https://europe.api.riotgames.com/lol/match/v5/matches/EUW1_42/timeline"
        );
    }

    #[test]
    fn les_segments_sont_encodes() {
        let r = Request::match_detail("a/b?c");
        assert_eq!(
            r.url(&base(Route::Europe)).path(),
            "/lol/match/v5/matches/a%2Fb%3Fc"
        );
    }

    #[test]
    fn classe_les_statuts_http() {
        assert_eq!(classify_status(200), Ok(()));
        assert_eq!(classify_status(400), Err(RiotError::BadRequest));
        assert_eq!(classify_status(401), Err(RiotError::Unauthorized(401)));
        assert_eq!(classify_status(403), Err(RiotError::Unauthorized(403)));
        assert_eq!(classify_status(404), Err(RiotError::NotFound));
        assert_eq!(classify_status(503), Err(RiotError::Server(503)));
        assert_eq!(classify_status(302), Err(RiotError::Unexpected(302)));
    }

    #[test]
    fn la_configuration_tls_publique_se_construit() {
        assert!(public_tls_config().is_ok());
    }

    struct Scripted(Mutex<Vec<Result<RawResponse, TransportError>>>);

    impl Transport for Scripted {
        async fn send(&self, _request: &Request) -> Result<RawResponse, TransportError> {
            self.0.lock().unwrap().remove(0)
        }
    }

    fn client(responses: Vec<Result<RawResponse, TransportError>>) -> RiotClient<Scripted> {
        let governor = Arc::new(Governor::new(RateLimiter::new(vec![(
            100,
            Duration::from_secs(1),
        )])));
        RiotClient::new(
            Scripted(Mutex::new(responses)),
            governor,
            Duration::from_secs(10),
        )
    }

    fn response(status: u16, headers: &[(&str, &str)]) -> RawResponse {
        RawResponse {
            status,
            headers: headers
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            body: b"[]".to_vec(),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn un_429_avec_retry_after_donne_la_pause_de_riot() {
        let c = client(vec![Ok(response(
            429,
            &[("Retry-After", "7"), ("X-Rate-Limit-Type", "method")],
        ))]);
        let err = c.get(&Request::match_detail("EUW1_1")).await.unwrap_err();
        assert_eq!(
            err,
            RiotError::RateLimited {
                scope: LimitScope::Method,
                pause: Duration::from_secs(7)
            }
        );
        assert_eq!(c.calls(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn un_429_sans_en_tetes_applique_la_pause_prudente() {
        let c = client(vec![Ok(response(429, &[])), Ok(response(200, &[]))]);
        let start = tokio::time::Instant::now();
        let err = c.get(&Request::match_detail("EUW1_1")).await.unwrap_err();
        assert_eq!(
            err,
            RiotError::RateLimited {
                scope: LimitScope::Unknown,
                pause: Duration::from_secs(10)
            }
        );
        // L'appel suivant sur le même hôte attend la fin de la pause.
        c.get(&Request::timeline("EUW1_1")).await.unwrap();
        assert!(tokio::time::Instant::now() - start >= Duration::from_secs(10));
    }

    #[tokio::test]
    async fn timeout_et_erreurs_http_sont_remontes() {
        let c = client(vec![
            Err(TransportError::Timeout),
            Ok(response(503, &[])),
            Ok(response(403, &[])),
        ]);
        let r = Request::match_detail("EUW1_1");
        assert_eq!(
            c.get(&r).await,
            Err(RiotError::Transport(TransportError::Timeout))
        );
        assert_eq!(c.get(&r).await, Err(RiotError::Server(503)));
        assert_eq!(c.get(&r).await, Err(RiotError::Unauthorized(403)));
    }
}
