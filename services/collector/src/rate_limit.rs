//! Gestionnaire de quotas Riot : fenêtres applicatives par hôte de routage et fenêtres
//! par méthode, alimentées par les en-têtes des réponses.
//!
//! Riot documente trois niveaux de limites (application, méthode, service) et renvoie
//! les limites effectives de la clé dans `X-App-Rate-Limit` et `X-Method-Rate-Limit`,
//! avec les compteurs courants dans `X-App-Rate-Limit-Count` et
//! `X-Method-Rate-Limit-Count` (format `limite:secondes,…`).
//! Source : <https://developer.riotgames.com/docs/portal#web-apis_rate-limiting>.

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::riot_client::{Endpoint, Route};

/// Portée d'un `429`, lue dans `X-Rate-Limit-Type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitScope {
    Application,
    Method,
    /// Limite du service Riot sous-jacent, indépendante de notre clé.
    Service,
    /// En-tête absent ou inconnu.
    Unknown,
}

impl LimitScope {
    pub fn from_header(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some(v) if v.eq_ignore_ascii_case("application") => LimitScope::Application,
            Some(v) if v.eq_ignore_ascii_case("method") => LimitScope::Method,
            Some(v) if v.eq_ignore_ascii_case("service") => LimitScope::Service,
            _ => LimitScope::Unknown,
        }
    }
}

/// Lit `20:1,100:120` en `[(20, 1 s), (100, 120 s)]`. `None` si le format est invalide.
pub fn parse_limits(value: &str) -> Option<Vec<(u32, Duration)>> {
    let mut out = Vec::new();
    for part in value.split(',') {
        let (count, secs) = part.trim().split_once(':')?;
        let count: u32 = count.trim().parse().ok()?;
        let secs: u64 = secs.trim().parse().ok()?;
        if secs == 0 {
            return None;
        }
        out.push((count, Duration::from_secs(secs)));
    }
    (!out.is_empty()).then_some(out)
}

/// Lit `Retry-After` exprimé en secondes. `None` si absent ou invalide.
pub fn parse_retry_after(value: Option<&str>) -> Option<Duration> {
    value?.trim().parse::<u64>().ok().map(Duration::from_secs)
}

#[derive(Debug)]
struct Window {
    limit: u32,
    period: Duration,
    /// Instants d'envoi encore dans la fenêtre (fenêtre glissante, plus prudente que
    /// la fenêtre fixe de Riot).
    sent: VecDeque<Instant>,
}

impl Window {
    fn new(limit: u32, period: Duration) -> Self {
        Self {
            limit,
            period,
            sent: VecDeque::new(),
        }
    }

    fn prune(&mut self, now: Instant) {
        while let Some(&first) = self.sent.front() {
            if now.duration_since(first) >= self.period {
                self.sent.pop_front();
            } else {
                break;
            }
        }
    }

    /// `None` si un envoi est possible maintenant, sinon l'instant où il le sera.
    fn available_at(&mut self, now: Instant) -> Option<Instant> {
        self.prune(now);
        if (self.sent.len() as u64) < u64::from(self.limit) {
            return None;
        }
        if self.limit == 0 {
            return Some(now + self.period);
        }
        let idx = self.sent.len() - self.limit as usize;
        Some(self.sent[idx] + self.period)
    }

    /// Aligne notre compteur sur celui de Riot quand il est plus élevé (par exemple
    /// après un redémarrage du processus, alors que la fenêtre de Riot court toujours).
    fn sync_count(&mut self, riot_count: u32, now: Instant) {
        self.prune(now);
        while (self.sent.len() as u64) < u64::from(riot_count) {
            self.sent.push_back(now);
        }
    }
}

#[derive(Debug, Default)]
struct Bucket {
    windows: Vec<Window>,
    blocked_until: Option<Instant>,
}

impl Bucket {
    fn with_limits(limits: &[(u32, Duration)]) -> Self {
        let mut bucket = Bucket::default();
        bucket.set_limits(limits);
        bucket
    }

    /// Adopte les limites annoncées par Riot en conservant l'historique des fenêtres
    /// de même durée.
    fn set_limits(&mut self, limits: &[(u32, Duration)]) {
        let mut old = std::mem::take(&mut self.windows);
        for &(limit, period) in limits {
            match old.iter().position(|w| w.period == period) {
                Some(i) => {
                    let mut w = old.swap_remove(i);
                    w.limit = limit;
                    self.windows.push(w);
                }
                None => self.windows.push(Window::new(limit, period)),
            }
        }
    }

    fn sync_counts(&mut self, counts: &[(u32, Duration)], now: Instant) {
        for &(count, period) in counts {
            if let Some(w) = self.windows.iter_mut().find(|w| w.period == period) {
                w.sync_count(count, now);
            }
        }
    }

    fn available_at(&mut self, now: Instant) -> Option<Instant> {
        let mut at = self.blocked_until.filter(|&b| b > now);
        for w in &mut self.windows {
            at = max_opt(at, w.available_at(now));
        }
        at
    }

    fn record(&mut self, now: Instant) {
        for w in &mut self.windows {
            w.sent.push_back(now);
        }
    }

    fn block(&mut self, until: Instant) {
        self.blocked_until = max_opt(self.blocked_until, Some(until));
    }
}

fn max_opt(a: Option<Instant>, b: Option<Instant>) -> Option<Instant> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (x, None) => x,
        (None, y) => y,
    }
}

/// État des quotas, sans horloge propre : chaque méthode reçoit l'instant courant.
#[derive(Debug)]
pub struct RateLimiter {
    initial_app_limits: Vec<(u32, Duration)>,
    app: HashMap<Route, Bucket>,
    methods: HashMap<(Route, Endpoint), Bucket>,
}

impl RateLimiter {
    pub fn new(initial_app_limits: Vec<(u32, Duration)>) -> Self {
        Self {
            initial_app_limits,
            app: HashMap::new(),
            methods: HashMap::new(),
        }
    }

    fn app_bucket(&mut self, route: Route) -> &mut Bucket {
        let initial = &self.initial_app_limits;
        self.app
            .entry(route)
            .or_insert_with(|| Bucket::with_limits(initial))
    }

    /// Réserve un envoi si tous les quotas le permettent ; sinon renvoie l'instant
    /// à partir duquel réessayer.
    pub fn try_acquire(&mut self, endpoint: Endpoint, now: Instant) -> Result<(), Instant> {
        self.try_acquire_on(endpoint.route(), endpoint, now)
    }

    pub fn try_acquire_on(
        &mut self,
        route: Route,
        endpoint: Endpoint,
        now: Instant,
    ) -> Result<(), Instant> {
        let app_at = self.app_bucket(route).available_at(now);
        let method_at = self
            .methods
            .entry((route, endpoint))
            .or_default()
            .available_at(now);
        match max_opt(app_at, method_at) {
            Some(at) => Err(at),
            None => {
                self.app_bucket(route).record(now);
                self.methods
                    .entry((route, endpoint))
                    .or_default()
                    .record(now);
                Ok(())
            }
        }
    }

    /// Met à jour limites et compteurs à partir des en-têtes d'une réponse.
    pub fn observe(&mut self, endpoint: Endpoint, headers: &RateHeaders<'_>, now: Instant) {
        self.observe_on(endpoint.route(), endpoint, headers, now)
    }

    pub fn observe_on(
        &mut self,
        route: Route,
        endpoint: Endpoint,
        headers: &RateHeaders<'_>,
        now: Instant,
    ) {
        if let Some(limits) = headers.app_limit.and_then(parse_limits) {
            self.app_bucket(route).set_limits(&limits);
        }
        if let Some(counts) = headers.app_count.and_then(parse_limits) {
            self.app_bucket(route).sync_counts(&counts, now);
        }
        let method = self.methods.entry((route, endpoint)).or_default();
        if let Some(limits) = headers.method_limit.and_then(parse_limits) {
            method.set_limits(&limits);
        }
        if let Some(counts) = headers.method_count.and_then(parse_limits) {
            method.sync_counts(&counts, now);
        }
    }

    /// Bloque la portée concernée par un `429` et renvoie la pause appliquée.
    ///
    /// Sans `Retry-After`, la pause par défaut s'applique ; sans type de limite,
    /// tout l'hôte de routage est mis en pause par prudence.
    pub fn on_rate_limited(
        &mut self,
        endpoint: Endpoint,
        scope: LimitScope,
        retry_after: Option<Duration>,
        default_pause: Duration,
        now: Instant,
    ) -> Duration {
        self.on_rate_limited_on(
            endpoint.route(),
            endpoint,
            scope,
            retry_after,
            default_pause,
            now,
        )
    }

    pub fn on_rate_limited_on(
        &mut self,
        route: Route,
        endpoint: Endpoint,
        scope: LimitScope,
        retry_after: Option<Duration>,
        default_pause: Duration,
        now: Instant,
    ) -> Duration {
        let pause = retry_after.unwrap_or(default_pause);
        let until = now + pause;
        match scope {
            LimitScope::Method | LimitScope::Service => self
                .methods
                .entry((route, endpoint))
                .or_default()
                .block(until),
            LimitScope::Application | LimitScope::Unknown => self.app_bucket(route).block(until),
        }
        pause
    }
}

/// En-têtes de quotas d'une réponse.
#[derive(Debug, Default, Clone, Copy)]
pub struct RateHeaders<'a> {
    pub app_limit: Option<&'a str>,
    pub app_count: Option<&'a str>,
    pub method_limit: Option<&'a str>,
    pub method_count: Option<&'a str>,
}

/// Point de passage unique de tous les appels Riot, nouvelles tentatives comprises.
#[derive(Debug)]
pub struct Governor {
    inner: Mutex<RateLimiter>,
}

impl Governor {
    pub fn new(limiter: RateLimiter) -> Self {
        Self {
            inner: Mutex::new(limiter),
        }
    }

    /// Attend qu'un envoi soit autorisé puis le réserve.
    pub async fn acquire(&self, endpoint: Endpoint) {
        self.acquire_on(endpoint.route(), endpoint).await
    }

    pub async fn acquire_on(&self, route: Route, endpoint: Endpoint) {
        loop {
            let wait_until = {
                let mut limiter = self.inner.lock().await;
                match limiter.try_acquire_on(route, endpoint, Instant::now()) {
                    Ok(()) => return,
                    Err(at) => at,
                }
            };
            tokio::time::sleep_until(wait_until).await;
        }
    }

    pub async fn observe(&self, endpoint: Endpoint, headers: &RateHeaders<'_>) {
        self.observe_on(endpoint.route(), endpoint, headers).await
    }

    pub async fn observe_on(&self, route: Route, endpoint: Endpoint, headers: &RateHeaders<'_>) {
        self.inner
            .lock()
            .await
            .observe_on(route, endpoint, headers, Instant::now());
    }

    pub async fn on_rate_limited(
        &self,
        endpoint: Endpoint,
        scope: LimitScope,
        retry_after: Option<Duration>,
        default_pause: Duration,
    ) -> Duration {
        self.on_rate_limited_on(
            endpoint.route(),
            endpoint,
            scope,
            retry_after,
            default_pause,
        )
        .await
    }

    pub async fn on_rate_limited_on(
        &self,
        route: Route,
        endpoint: Endpoint,
        scope: LimitScope,
        retry_after: Option<Duration>,
        default_pause: Duration,
    ) -> Duration {
        self.inner.lock().await.on_rate_limited_on(
            route,
            endpoint,
            scope,
            retry_after,
            default_pause,
            Instant::now(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: Duration = Duration::from_secs(1);

    fn limiter(limits: &[(u32, u64)]) -> RateLimiter {
        RateLimiter::new(
            limits
                .iter()
                .map(|&(c, s)| (c, Duration::from_secs(s)))
                .collect(),
        )
    }

    #[test]
    fn les_quotas_de_methode_sont_separes_par_region() {
        let now = Instant::now();
        let mut limiter = limiter(&[(100, 1)]);
        limiter.observe_on(
            Route::Europe,
            Endpoint::Match,
            &RateHeaders {
                method_limit: Some("1:60"),
                method_count: Some("1:60"),
                ..RateHeaders::default()
            },
            now,
        );
        assert!(limiter
            .try_acquire_on(Route::Europe, Endpoint::Match, now)
            .is_err());
        assert!(limiter
            .try_acquire_on(Route::Asia, Endpoint::Match, now)
            .is_ok());
    }

    #[test]
    fn lit_les_limites_riot() {
        assert_eq!(
            parse_limits("20:1,100:120"),
            Some(vec![(20, S), (100, Duration::from_secs(120))])
        );
        assert_eq!(parse_limits(""), None);
        assert_eq!(parse_limits("20"), None);
        assert_eq!(parse_limits("20:0"), None);
        assert_eq!(parse_limits("a:1"), None);
    }

    #[test]
    fn lit_retry_after_et_le_type_de_limite() {
        assert_eq!(parse_retry_after(Some("7")), Some(Duration::from_secs(7)));
        assert_eq!(parse_retry_after(Some("demain")), None);
        assert_eq!(parse_retry_after(None), None);
        assert_eq!(
            LimitScope::from_header(Some("application")),
            LimitScope::Application
        );
        assert_eq!(LimitScope::from_header(Some("method")), LimitScope::Method);
        assert_eq!(
            LimitScope::from_header(Some("service")),
            LimitScope::Service
        );
        assert_eq!(LimitScope::from_header(None), LimitScope::Unknown);
    }

    #[test]
    fn respecte_toutes_les_fenetres_applicatives() {
        let mut rl = limiter(&[(2, 1), (3, 10)]);
        let t0 = Instant::now();
        assert!(rl.try_acquire(Endpoint::Match, t0).is_ok());
        assert!(rl.try_acquire(Endpoint::Match, t0).is_ok());
        // Fenêtre d'une seconde pleine.
        assert_eq!(rl.try_acquire(Endpoint::Match, t0), Err(t0 + S));
        assert!(rl.try_acquire(Endpoint::Match, t0 + S).is_ok());
        // Fenêtre de dix secondes pleine : le premier envoi en sort à t0 + 10 s.
        assert_eq!(
            rl.try_acquire(Endpoint::Match, t0 + S),
            Err(t0 + Duration::from_secs(10))
        );
    }

    #[test]
    fn les_quotas_applicatifs_sont_separes_par_hote_de_routage() {
        let mut rl = limiter(&[(1, 1)]);
        let t0 = Instant::now();
        assert!(rl.try_acquire(Endpoint::Match, t0).is_ok());
        // euw1 (league-v4) ne partage pas le compteur d'europe (match-v5).
        assert!(rl.try_acquire(Endpoint::LeagueEntries, t0).is_ok());
        assert!(rl.try_acquire(Endpoint::Timeline, t0).is_err());
    }

    #[test]
    fn adopte_les_limites_de_methode_annoncees() {
        let mut rl = limiter(&[(100, 1)]);
        let t0 = Instant::now();
        let headers = RateHeaders {
            method_limit: Some("1:10"),
            method_count: Some("1:10"),
            ..Default::default()
        };
        assert!(rl.try_acquire(Endpoint::Timeline, t0).is_ok());
        rl.observe(Endpoint::Timeline, &headers, t0);
        assert_eq!(
            rl.try_acquire(Endpoint::Timeline, t0),
            Err(t0 + Duration::from_secs(10))
        );
        // Une autre méthode du même hôte n'est pas concernée.
        assert!(rl.try_acquire(Endpoint::Match, t0).is_ok());
    }

    #[test]
    fn se_recale_sur_le_compteur_riot_apres_un_redemarrage() {
        let mut rl = limiter(&[(100, 120)]);
        let t0 = Instant::now();
        assert!(rl.try_acquire(Endpoint::Match, t0).is_ok());
        // Riot a déjà compté 100 requêtes (processus précédent) : on doit attendre.
        let headers = RateHeaders {
            app_limit: Some("100:120"),
            app_count: Some("100:120"),
            ..Default::default()
        };
        rl.observe(Endpoint::Match, &headers, t0);
        assert!(rl.try_acquire(Endpoint::Match, t0).is_err());
    }

    #[test]
    fn un_429_bloque_la_portee_indiquee() {
        let mut rl = limiter(&[(100, 1)]);
        let t0 = Instant::now();
        let d = Duration::from_secs(10);
        let pause = rl.on_rate_limited(
            Endpoint::Timeline,
            LimitScope::Method,
            Some(Duration::from_secs(3)),
            d,
            t0,
        );
        assert_eq!(pause, Duration::from_secs(3));
        assert_eq!(
            rl.try_acquire(Endpoint::Timeline, t0),
            Err(t0 + Duration::from_secs(3))
        );
        assert!(rl.try_acquire(Endpoint::Match, t0).is_ok());
    }

    #[test]
    fn un_429_sans_type_ni_retry_after_met_l_hote_en_pause_par_defaut() {
        let mut rl = limiter(&[(100, 1)]);
        let t0 = Instant::now();
        let d = Duration::from_secs(10);
        let pause = rl.on_rate_limited(Endpoint::Match, LimitScope::Unknown, None, d, t0);
        assert_eq!(pause, d);
        assert_eq!(rl.try_acquire(Endpoint::Timeline, t0), Err(t0 + d));
        // L'autre hôte continue.
        assert!(rl.try_acquire(Endpoint::LeagueEntries, t0).is_ok());
        assert!(rl.try_acquire(Endpoint::Match, t0 + d).is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn le_gouverneur_attend_la_fin_de_la_fenetre() {
        let governor = Governor::new(limiter(&[(2, 1)]));
        let start = Instant::now();
        governor.acquire(Endpoint::Match).await;
        governor.acquire(Endpoint::Match).await;
        assert_eq!(Instant::now(), start);
        governor.acquire(Endpoint::Match).await;
        assert_eq!(Instant::now() - start, S);
    }
}
