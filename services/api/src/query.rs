//! Dimensions statistiques et pagination contrôlées avant tout accès SQL.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StatsQuery {
    pub patch: String,
    pub platform: String,
    pub queue: i32,
    pub role: String,
    #[serde(default = "default_rank")]
    pub rank: String,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
}
/// Ordre des variantes de build au sein de chaque catégorie (#112). L'effectif reste le défaut :
/// le tri par performance est un choix explicite et ne change aucune autre route.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildSort {
    /// Effectif décroissant (comportement historique).
    #[default]
    Games,
    /// Borne basse de Wilson décroissante, puis effectif ; les variantes sans borne en dernier.
    Performance,
}

/// Paramètres de `/v1/builds/{champion}` : ceux de [`StatsQuery`] plus le tri. Champs répétés
/// à dessein : `flatten` empêcherait de lire les nombres d'une chaîne de requête.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BuildsQuery {
    pub patch: String,
    pub platform: String,
    pub queue: i32,
    pub role: String,
    #[serde(default = "default_rank")]
    pub rank: String,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub sort: BuildSort,
}
impl BuildsQuery {
    /// Dimensions et pagination, validées par les mêmes règles que les autres routes.
    pub fn stats(&self) -> StatsQuery {
        StatsQuery {
            patch: self.patch.clone(),
            platform: self.platform.clone(),
            queue: self.queue,
            role: self.role.clone(),
            rank: self.rank.clone(),
            offset: self.offset,
            limit: self.limit,
        }
    }
}
fn default_rank() -> String {
    "ALL".into()
}
fn default_limit() -> usize {
    50
}
impl StatsQuery {
    pub fn validate(&self) -> Result<(), &'static str> {
        let parts: Vec<_> = self.patch.split('.').collect();
        if parts.len() != 2
            || parts
                .iter()
                .any(|p| p.is_empty() || p.len() > 3 || !p.bytes().all(|b| b.is_ascii_digit()))
            || !olc_collector::config::PLATFORMS.contains(&self.platform.as_str())
            || self.queue <= 0
            || self.queue > 100_000
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
            ]
            .contains(&self.rank.as_str())
            || self.offset > 10_000
            || !(1..=200).contains(&self.limit)
        {
            return Err("invalid_request");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn controle_les_dimensions_sans_melanger_les_populations() {
        let q: StatsQuery = serde_json::from_value(
            json!({"patch":"16.19","platform":"EUW1","queue":420,"role":"MIDDLE"}),
        )
        .unwrap();
        assert_eq!(q.rank, "ALL");
        assert!(q.validate().is_ok());
        for invalid in [
            StatsQuery {
                platform: "EUROPE".into(),
                ..q.clone()
            },
            StatsQuery {
                patch: "x".into(),
                ..q.clone()
            },
            StatsQuery {
                role: "MID".into(),
                ..q.clone()
            },
            StatsQuery {
                rank: "FAKE".into(),
                ..q.clone()
            },
            StatsQuery {
                limit: 0,
                ..q.clone()
            },
            StatsQuery {
                limit: 201,
                ..q.clone()
            },
            StatsQuery {
                queue: 0,
                ..q.clone()
            },
            StatsQuery {
                offset: 10_001,
                ..q.clone()
            },
        ] {
            assert!(invalid.validate().is_err(), "{invalid:?}");
        }
        assert!(serde_json::from_value::<StatsQuery>(
            json!({"patch":"16.19","platform":"EUW1","queue":420,"role":"MIDDLE","start_ms":1})
        )
        .is_err());
    }

    #[test]
    fn le_tri_des_builds_est_explicite_et_l_effectif_reste_le_defaut() {
        let base = json!({"patch":"16.19","platform":"EUW1","queue":420,"role":"MIDDLE"});
        let q: BuildsQuery = serde_json::from_value(base.clone()).unwrap();
        assert_eq!(q.sort, BuildSort::Games);
        let mut performance = base.clone();
        performance["sort"] = json!("performance");
        let q: BuildsQuery = serde_json::from_value(performance).unwrap();
        assert_eq!(q.sort, BuildSort::Performance);
        // La requête de statistiques reste celle des autres routes, sans le tri.
        assert_eq!(q.stats().patch, "16.19");
        assert!(q.stats().validate().is_ok());
        for invalid in [json!("winrate"), json!(""), json!(null), json!(1)] {
            let mut bad = base.clone();
            bad["sort"] = invalid;
            assert!(serde_json::from_value::<BuildsQuery>(bad).is_err());
        }
        let mut unknown = base;
        unknown["start_ms"] = json!(1);
        assert!(serde_json::from_value::<BuildsQuery>(unknown).is_err());
    }
}
