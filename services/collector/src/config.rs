//! Paramètres d'une collecte, options d'exécution et secrets lus dans l'environnement.

use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Plateforme et file du prototype : EUW, Ranked Solo/Duo.
pub const PLATFORM_ID: &str = "EUW1";
pub const RANKED_SOLO_QUEUE_ID: i32 = 420;
/// Nom de la file côté league-v4.
pub const RANKED_SOLO_QUEUE: &str = "RANKED_SOLO_5x5";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("RIOT_API_KEY est absente ou vide : définissez-la dans l'environnement ou dans un fichier .env")]
    MissingApiKey,
    #[error("DATABASE_URL est absente ou vide : définissez-la dans l'environnement ou dans un fichier .env")]
    MissingDatabaseUrl,
    #[error(
        "rang inconnu « {0} » (attendu : IRON, BRONZE, SILVER, GOLD, PLATINUM, EMERALD ou DIAMOND)"
    )]
    UnknownTier(String),
    #[error("division inconnue « {0} » (attendu : I, II, III ou IV)")]
    UnknownDivision(String),
    #[error("paramètre invalide : {0}")]
    Invalid(&'static str),
}

/// Clé API Riot. Jamais affichée : `Debug` et `Display` sont masqués.
#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    /// Construit la clé à partir de la valeur de `RIOT_API_KEY`.
    pub fn from_env_value(value: Option<String>) -> Result<Self, ConfigError> {
        match value.map(|v| v.trim().to_owned()) {
            Some(v) if !v.is_empty() => Ok(Self(v)),
            _ => Err(ConfigError::MissingApiKey),
        }
    }

    /// Valeur brute, à n'utiliser que pour l'en-tête `X-Riot-Token`.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(***)")
    }
}

/// Valide `DATABASE_URL`.
pub fn database_url(value: Option<String>) -> Result<String, ConfigError> {
    match value.map(|v| v.trim().to_owned()) {
        Some(v) if !v.is_empty() => Ok(v),
        _ => Err(ConfigError::MissingDatabaseUrl),
    }
}

/// Rangs divisés en I à IV (les rangs Master et plus utilisent d'autres endpoints).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Tier {
    Iron,
    Bronze,
    Silver,
    Gold,
    Platinum,
    Emerald,
    Diamond,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Iron => "IRON",
            Tier::Bronze => "BRONZE",
            Tier::Silver => "SILVER",
            Tier::Gold => "GOLD",
            Tier::Platinum => "PLATINUM",
            Tier::Emerald => "EMERALD",
            Tier::Diamond => "DIAMOND",
        }
    }
}

impl FromStr for Tier {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "IRON" => Ok(Tier::Iron),
            "BRONZE" => Ok(Tier::Bronze),
            "SILVER" => Ok(Tier::Silver),
            "GOLD" => Ok(Tier::Gold),
            "PLATINUM" => Ok(Tier::Platinum),
            "EMERALD" => Ok(Tier::Emerald),
            "DIAMOND" => Ok(Tier::Diamond),
            _ => Err(ConfigError::UnknownTier(s.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Division {
    I,
    II,
    III,
    IV,
}

impl Division {
    pub fn as_str(self) -> &'static str {
        match self {
            Division::I => "I",
            Division::II => "II",
            Division::III => "III",
            Division::IV => "IV",
        }
    }
}

impl FromStr for Division {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "I" => Ok(Division::I),
            "II" => Ok(Division::II),
            "III" => Ok(Division::III),
            "IV" => Ok(Division::IV),
            _ => Err(ConfigError::UnknownDivision(s.to_owned())),
        }
    }
}

/// Paramètres figés d'une exécution, enregistrés dans `collection_runs.params`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunParams {
    pub target_matches: u32,
    pub tiers: Vec<Tier>,
    pub divisions: Vec<Division>,
    pub window_days: u32,
    /// Joueurs de départ retenus par rang et division.
    pub seeds_per_division: u32,
    /// Plafond de parties découvertes par joueur, pour qu'aucun joueur ne domine l'échantillon.
    pub max_matches_per_seed: u32,
    /// Nombre maximal d'appels Riot pour toute l'exécution (reprises comprises).
    pub call_budget: u64,
}

impl Default for RunParams {
    fn default() -> Self {
        Self {
            target_matches: 1000,
            tiers: vec![Tier::Gold, Tier::Platinum, Tier::Emerald],
            divisions: vec![Division::I, Division::II, Division::III, Division::IV],
            window_days: 14,
            seeds_per_division: 15,
            max_matches_per_seed: 10,
            call_budget: 3000,
        }
    }
}

impl RunParams {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.target_matches == 0 {
            return Err(ConfigError::Invalid("la cible doit être supérieure à 0"));
        }
        if self.tiers.is_empty() || self.divisions.is_empty() {
            return Err(ConfigError::Invalid(
                "au moins un rang et une division sont nécessaires",
            ));
        }
        if self.window_days == 0 || self.window_days > 365 {
            return Err(ConfigError::Invalid(
                "la fenêtre doit compter entre 1 et 365 jours",
            ));
        }
        if self.seeds_per_division == 0 || self.max_matches_per_seed == 0 {
            return Err(ConfigError::Invalid(
                "seeds par division et parties par joueur doivent être supérieurs à 0",
            ));
        }
        if self.call_budget == 0 {
            return Err(ConfigError::Invalid(
                "le budget d'appels doit être supérieur à 0",
            ));
        }
        Ok(())
    }

    /// Strates (rang, division) dans l'ordre de parcours.
    pub fn strata(&self) -> Vec<(Tier, Division)> {
        self.tiers
            .iter()
            .flat_map(|&t| self.divisions.iter().map(move |&d| (t, d)))
            .collect()
    }
}

/// Politique de nouvelles tentatives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Tentatives maximales pour une erreur serveur, réseau ou une réponse invalide.
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    /// Nouvelles vérifications après un premier 404 (partie ou timeline).
    pub not_found_retries: u32,
    pub not_found_delay: Duration,
    /// Pause après un 429 sans `Retry-After` exploitable.
    pub default_rate_limit_pause: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(300),
            not_found_retries: 2,
            not_found_delay: Duration::from_secs(600),
            default_rate_limit_pause: Duration::from_secs(10),
        }
    }
}

impl RetryPolicy {
    /// Délai avant la tentative suivante : exponentiel, plafonné, avec une part aléatoire
    /// (`jitter` entre 0 et 1) qui évite de relancer toutes les requêtes en même temps.
    pub fn backoff(&self, attempts: u32, jitter: f64) -> Duration {
        let exp = self
            .base_delay
            .saturating_mul(2u32.saturating_pow(attempts.min(20)));
        let capped = exp.min(self.max_delay);
        capped.mul_f64(0.5 + 0.5 * jitter.clamp(0.0, 1.0))
    }
}

/// Réglages propres à un lancement du processus (non figés dans l'exécution).
#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub concurrency: usize,
    pub max_duration: Option<Duration>,
    /// Limites applicatives supposées avant la première réponse Riot, ensuite remplacées
    /// par les en-têtes `X-App-Rate-Limit`.
    pub initial_app_limits: Vec<(u32, Duration)>,
    pub retry: RetryPolicy,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            concurrency: 2,
            max_duration: None,
            // Limites publiées d'une clé de développement : 20 requêtes/s et 100 requêtes/2 min.
            initial_app_limits: vec![
                (20, Duration::from_secs(1)),
                (100, Duration::from_secs(120)),
            ],
            retry: RetryPolicy::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_cle_absente_ou_vide_donne_une_erreur_claire() {
        assert_eq!(
            ApiKey::from_env_value(None).unwrap_err(),
            ConfigError::MissingApiKey
        );
        assert_eq!(
            ApiKey::from_env_value(Some("  ".into())).unwrap_err(),
            ConfigError::MissingApiKey
        );
        assert!(ConfigError::MissingApiKey
            .to_string()
            .contains("RIOT_API_KEY"));
    }

    #[test]
    fn la_cle_n_apparait_jamais_dans_debug() {
        let key = ApiKey::from_env_value(Some("valeur-de-test".into())).unwrap();
        assert_eq!(format!("{key:?}"), "ApiKey(***)");
        assert_eq!(key.expose(), "valeur-de-test");
    }

    #[test]
    fn rangs_et_divisions_se_lisent_sans_tenir_compte_de_la_casse() {
        assert_eq!("gold".parse::<Tier>().unwrap(), Tier::Gold);
        assert_eq!("iv".parse::<Division>().unwrap(), Division::IV);
        assert!(matches!(
            "MASTER".parse::<Tier>(),
            Err(ConfigError::UnknownTier(_))
        ));
        assert!("V".parse::<Division>().is_err());
    }

    #[test]
    fn les_strates_suivent_l_ordre_rang_puis_division() {
        let params = RunParams {
            tiers: vec![Tier::Gold, Tier::Emerald],
            divisions: vec![Division::I, Division::II],
            ..RunParams::default()
        };
        assert_eq!(
            params.strata(),
            vec![
                (Tier::Gold, Division::I),
                (Tier::Gold, Division::II),
                (Tier::Emerald, Division::I),
                (Tier::Emerald, Division::II),
            ]
        );
    }

    #[test]
    fn les_parametres_par_defaut_sont_valides_et_les_zeros_refuses() {
        assert!(RunParams::default().validate().is_ok());
        let zero_target = RunParams {
            target_matches: 0,
            ..RunParams::default()
        };
        assert!(zero_target.validate().is_err());
        let no_tier = RunParams {
            tiers: vec![],
            ..RunParams::default()
        };
        assert!(no_tier.validate().is_err());
    }

    #[test]
    fn le_delai_croit_puis_plafonne() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.backoff(0, 1.0), Duration::from_secs(2));
        assert_eq!(policy.backoff(2, 1.0), Duration::from_secs(8));
        assert_eq!(policy.backoff(30, 1.0), Duration::from_secs(300));
        // La part aléatoire réduit le délai jusqu'à la moitié, jamais en dessous.
        assert_eq!(policy.backoff(2, 0.0), Duration::from_secs(4));
    }
}
