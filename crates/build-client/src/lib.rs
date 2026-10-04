//! Lecture des builds communautaires, sans transport de données LCU.

pub mod credentials;
mod observations;
pub use observations::{BuildDetails, BuildSummary, ItemObservation, SkillObservation};
pub mod profiles;
pub mod publications;

use reqwest::{header::HeaderValue, Url};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    net::IpAddr,
    sync::Arc,
    time::Duration,
};

const PAGE_SIZE: usize = 200;
const MAX_VARIANTS: usize = 1_000;
const MAX_BYTES: usize = 8 * 1024 * 1024;

/// Périmètre choisi pour les statistiques, jamais déduit d'un identifiant de compte.
/// Miroir de `BuildRequest` dans @olc/shared.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRequest {
    pub champion_id: u32,
    pub patch: String,
    pub platform: String,
    pub queue: u32,
    pub role: String,
    pub rank: String,
}
impl BuildRequest {
    /// Même périmètre que l'API #19 ; pagination gérée par le transport.
    pub fn validate(&self) -> Result<(), BuildError> {
        let patch: Vec<_> = self.patch.split('.').collect();
        if self.champion_id == 0
            || patch.len() != 2
            || patch
                .iter()
                .any(|p| p.is_empty() || p.len() > 3 || !p.bytes().all(|b| b.is_ascii_digit()))
            || !(1..=100_000).contains(&self.queue)
            || ![
                "BR1", "EUN1", "EUW1", "JP1", "KR", "LA1", "LA2", "ME1", "NA1", "OC1", "RU", "SG2",
                "TR1", "TW2", "VN2",
            ]
            .contains(&self.platform.as_str())
            || !["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY", "UNKNOWN"]
                .contains(&self.role.as_str())
            || ![
                "ALL",
                "IRON",
                "BRONZE",
                "SILVER",
                "GOLD",
                "PLATINUM",
                "EMERALD",
                "DIAMOND",
                "MASTER",
                "GRANDMASTER",
                "CHALLENGER",
                "UNKNOWN",
                "UNRANKED",
                "UNRANKED_MODE",
                // Paliers cumulés (#83), miroir de `CUMULATIVE_RANKS` côté collecteur.
                "IRON_PLUS",
                "BRONZE_PLUS",
                "SILVER_PLUS",
                "GOLD_PLUS",
                "PLATINUM_PLUS",
                "EMERALD_PLUS",
                "DIAMOND_PLUS",
                "MASTER_PLUS",
            ]
            .contains(&self.rank.as_str())
        {
            return Err(BuildError::InvalidRequest);
        }
        Ok(())
    }
}

/// Erreurs stables traduites par le front ; aucun corps HTTP, URL ou jeton.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildError {
    NotConfigured,
    InvalidConfiguration,
    InvalidRequest,
    Unauthorized,
    Unavailable,
    RateLimited,
    InvalidResponse,
    ChangedSnapshot,
}

/// Métadonnées affichées avec les statistiques (miroir BuildReport.meta).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildMeta {
    pub source_snapshot_at: String,
    pub published_at: String,
    pub min_games: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population_label: Option<String>,
    #[serde(default)]
    pub coverage: Vec<BuildPopulationCoverage>,
}

/// Répartition du périmètre entier, tous rôles confondus ; projection de ScopeCoverage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildPopulationCoverage {
    pub patch: String,
    pub platform_id: String,
    pub queue_id: u32,
    pub ranked_participations: u64,
    #[serde(default)]
    pub tier_participations: BTreeMap<String, u64>,
    #[serde(default)]
    pub apex_share: Option<f64>,
    #[serde(default)]
    pub high_elo_biased: bool,
}
impl BuildMeta {
    fn check_population(&self, request: &BuildRequest) -> Result<(), BuildError> {
        if self
            .population_label
            .as_ref()
            .is_some_and(|label| label.len() > 64)
            || self.coverage.len() > 1
        {
            return Err(BuildError::InvalidResponse);
        }
        for scope in &self.coverage {
            if scope.patch != request.patch
                || scope.platform_id != request.platform
                || scope.queue_id != request.queue
                || scope.ranked_participations > 9_007_199_254_740_991
                || scope
                    .apex_share
                    .is_some_and(|share| !share.is_finite() || !(0.0..=1.0).contains(&share))
                || scope.tier_participations.iter().any(|(tier, count)| {
                    ![
                        "IRON",
                        "BRONZE",
                        "SILVER",
                        "GOLD",
                        "PLATINUM",
                        "EMERALD",
                        "DIAMOND",
                        "MASTER",
                        "GRANDMASTER",
                        "CHALLENGER",
                    ]
                    .contains(&tier.as_str())
                        || *count > 9_007_199_254_740_991
                })
                || (!scope.tier_participations.is_empty()
                    && scope
                        .tier_participations
                        .values()
                        .try_fold(0_u64, |sum, count| sum.checked_add(*count))
                        != Some(scope.ranked_participations))
            {
                return Err(BuildError::InvalidResponse);
            }
        }
        // Le drapeau publié fait foi ; aucun seuil de biais n’est recalculé ici.
        Ok(())
    }
}

/// Fiabilité d'un taux au regard de son effectif (#91), miroir de `Reliability` (@olc/shared).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reliability {
    Low,
    Sufficient,
}

/// Variante observée par catégorie, miroir exact de `BuildStats` (#19).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildVariant {
    pub patch: String,
    pub platform_id: String,
    pub queue_id: u32,
    pub role: String,
    pub rank: String,
    pub champion_id: u32,
    pub category: String,
    pub selection: Vec<u32>,
    pub games: u64,
    pub wins: Option<u64>,
    pub performance_available: bool,
    pub population: u64,
    #[serde(default)]
    pub omitted_variants: Option<u64>,
    pub pick_rate: Option<f64>,
    pub win_rate: Option<f64>,
    /// Borne inférieure de Wilson à 95 % (#81) ; absente des instantanés antérieurs.
    #[serde(default)]
    pub win_rate_lower_bound: Option<f64>,
    /// Écart signé (points) au winrate du groupe champion (#112) ; absent des instantanés antérieurs.
    #[serde(default)]
    pub win_rate_delta: Option<f64>,
    /// Taux conditionnel des runes (#86), en pourcentage (0 à 100, comme `pick_rate`) ; absent des instantanés antérieurs.
    #[serde(default)]
    pub conditional_rate: Option<f64>,
    /// Borne supérieure de Wilson à 95 % (#91) ; absente des instantanés antérieurs.
    #[serde(default)]
    pub win_rate_upper_bound: Option<f64>,
    /// `low` sous le plancher de fiabilité du serveur (#91), indépendant de `min_games`.
    #[serde(default)]
    pub reliability: Option<Reliability>,
    /// Arena, variantes hors objets : participations au placement valide (#104) ; 0 sinon.
    #[serde(default)]
    pub placement_games: u64,
    /// Arena, variantes hors objets : placement moyen (1 = première) ; absent des
    /// instantanés antérieurs, nul sous le seuil.
    #[serde(default)]
    pub average_placement: Option<f64>,
}
impl BuildVariant {
    fn check(&mut self, request: &BuildRequest, min_games: u32) -> Result<(), BuildError> {
        if self.champion_id != request.champion_id
            || self.patch != request.patch
            || self.platform_id != request.platform
            || self.queue_id != request.queue
            || self.role != request.role
            || self.rank != request.rank
            || self.category.len() > 64
            || self.selection.len() > 4096
            || self
                .omitted_variants
                .is_some_and(|n| n > 9_007_199_254_740_991)
            || self.games > self.population
            || self.wins.is_some_and(|wins| wins > self.games)
            || [
                self.pick_rate,
                self.win_rate,
                self.win_rate_lower_bound,
                self.win_rate_upper_bound,
            ]
            .into_iter()
            .flatten()
            .any(|v| !v.is_finite() || !(0.0..=100.0).contains(&v))
            || self.placement_games > self.games
            || self
                .average_placement
                .is_some_and(|v| !v.is_finite() || v < 1.0)
            || self
                .win_rate_delta
                .is_some_and(|v| !v.is_finite() || !(-100.0..=100.0).contains(&v))
            || self
                .conditional_rate
                .is_some_and(|v| !v.is_finite() || !(0.0..=100.0).contains(&v))
        {
            return Err(BuildError::InvalidResponse);
        }
        if self.games < u64::from(min_games) {
            self.average_placement = None;
            self.pick_rate = None;
            self.win_rate = None;
            self.win_rate_lower_bound = None;
            self.win_rate_upper_bound = None;
            self.win_rate_delta = None;
            self.conditional_rate = None;
        }
        // Même seuil que le collecteur : le placement moyen n'est publié qu'à partir de
        // `min_games` parties avec placement, pas seulement `min_games` parties jouées.
        if self.placement_games < u64::from(min_games) {
            self.average_placement = None;
        }
        if !self.performance_available {
            self.win_rate = None;
            self.wins = None;
            self.win_rate_lower_bound = None;
            self.win_rate_upper_bound = None;
            self.win_rate_delta = None;
        }
        Ok(())
    }
}

/// Projection complète des variantes d'une seule publication, miroir `BuildReport`.
#[derive(Debug, Serialize)]
pub struct BuildReport {
    pub request: BuildRequest,
    pub meta: BuildMeta,
    pub builds: Vec<BuildVariant>,
    #[serde(flatten)]
    pub details: BuildDetails,
}

#[derive(Deserialize)]
struct Page {
    meta: serde_json::Value,
    query: serde_json::Value,
    champion_id: u32,
    total: usize,
    builds: Vec<BuildVariant>,
    #[serde(flatten)]
    details: BuildDetails,
}

/// Client du serveur interne. Configuration et autorisation restent en Rust.
/// Pas de Debug : les paramètres privés ne doivent jamais être journalisés.
pub struct BuildClient {
    http: reqwest::Client,
    base: Url,
    /// Jeton de lecture, réservé au premier message du WebSocket ; jamais journalisé.
    token: String,
    tls: Arc<rustls::ClientConfig>,
    slots: tokio::sync::Semaphore,
}

impl BuildClient {
    /// URL d'origine HTTPS ou HTTP sur IP loopback. Redirections interdites.
    pub fn new(url: Option<String>, token: Option<String>) -> Result<Self, BuildError> {
        let (Some(url), Some(token)) = (url, token) else {
            return Err(BuildError::NotConfigured);
        };
        let base = Url::parse(&url).map_err(|_| BuildError::InvalidConfiguration)?;
        let loopback = base
            .host_str()
            .and_then(|s| s.trim_matches(['[', ']']).parse::<IpAddr>().ok())
            .is_some_and(|ip| ip.is_loopback());
        if !((base.scheme() == "https" && base.host_str().is_some())
            || (base.scheme() == "http" && loopback))
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
            || base.path() != "/"
            || token.is_empty()
            || token.len() > 8192
        {
            return Err(BuildError::InvalidConfiguration);
        }
        let mut authorization = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| BuildError::InvalidConfiguration)?;
        authorization.set_sensitive(true);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::AUTHORIZATION, authorization);
        let roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| BuildError::InvalidConfiguration)?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let http = reqwest::Client::builder()
            .use_preconfigured_tls(tls.clone())
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| BuildError::InvalidConfiguration)?;
        Ok(Self {
            http,
            base,
            token,
            tls: Arc::new(tls),
            slots: tokio::sync::Semaphore::new(4),
        })
    }

    /// Lit toutes les pages ; refuse de mélanger deux snapshots publiés entre-temps.
    pub async fn builds(&self, request: BuildRequest) -> Result<BuildReport, BuildError> {
        request.validate()?;
        tokio::time::timeout(Duration::from_secs(30), async {
            let _permit = self
                .slots
                .acquire()
                .await
                .map_err(|_| BuildError::Unavailable)?;
            self.read(request).await
        })
        .await
        .map_err(|_| BuildError::Unavailable)?
    }

    async fn read(&self, request: BuildRequest) -> Result<BuildReport, BuildError> {
        let mut rows = vec![];
        let mut publication = None;
        let mut total = None;
        let mut observations = None;
        let mut seen = HashSet::new();
        loop {
            let offset = rows.len();
            let mut url = self
                .base
                .join(&format!("v1/builds/{}", request.champion_id))
                .map_err(|_| BuildError::InvalidRequest)?;
            let expected = serde_json::json!({"patch":request.patch,"platform":request.platform,"queue":request.queue,"role":request.role,"rank":request.rank,"offset":offset,"limit":PAGE_SIZE});
            url.query_pairs_mut().extend_pairs([
                ("patch", request.patch.clone()),
                ("platform", request.platform.clone()),
                ("queue", request.queue.to_string()),
                ("role", request.role.clone()),
                ("rank", request.rank.clone()),
                ("offset", offset.to_string()),
                ("limit", PAGE_SIZE.to_string()),
            ]);
            let mut response = self
                .http
                .get(url)
                .send()
                .await
                .map_err(|_| BuildError::Unavailable)?;
            match response.status().as_u16() {
                200 => {}
                400 => return Err(BuildError::InvalidRequest),
                401 | 403 => return Err(BuildError::Unauthorized),
                429 => return Err(BuildError::RateLimited),
                _ => return Err(BuildError::Unavailable),
            }
            if response
                .content_length()
                .is_some_and(|size| size > MAX_BYTES as u64)
            {
                return Err(BuildError::InvalidResponse);
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| BuildError::Unavailable)?
            {
                if bytes.len() + chunk.len() > MAX_BYTES {
                    return Err(BuildError::InvalidResponse);
                }
                bytes.extend_from_slice(&chunk);
            }
            let mut page: Page =
                serde_json::from_slice(&bytes).map_err(|_| BuildError::InvalidResponse)?;
            if page.champion_id != request.champion_id
                || page.query != expected
                || page.total > MAX_VARIANTS
                || offset + page.builds.len() > page.total
                || page.builds.len() != PAGE_SIZE.min(page.total - offset)
            {
                return Err(BuildError::InvalidResponse);
            }
            if publication.as_ref().is_some_and(|meta| meta != &page.meta)
                || total.is_some_and(|value| value != page.total)
                || observations
                    .as_ref()
                    .is_some_and(|details| details != &page.details)
            {
                return Err(BuildError::ChangedSnapshot);
            }
            let meta: BuildMeta = serde_json::from_value(page.meta.clone())
                .map_err(|_| BuildError::InvalidResponse)?;
            if meta.min_games == 0
                || meta.source_snapshot_at.len() > 64
                || meta.published_at.len() > 64
            {
                return Err(BuildError::InvalidResponse);
            }
            meta.check_population(&request)?;
            observations = Some(page.details.clone());
            page.details.check(&request, meta.min_games)?;
            for row in &mut page.builds {
                row.check(&request, meta.min_games)?;
                if !seen.insert((row.category.clone(), row.selection.clone())) {
                    return Err(BuildError::InvalidResponse);
                }
            }
            rows.extend(page.builds);
            if rows.len() == page.total {
                return Ok(BuildReport {
                    request,
                    meta,
                    builds: rows,
                    details: page.details,
                });
            }
            total = Some(page.total);
            publication = Some(page.meta);
        }
    }
}

#[cfg(test)]
mod tests;
