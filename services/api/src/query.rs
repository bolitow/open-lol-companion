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
/// Bans d'une draft (#109) : périmètre et palier de partie, sans rôle ni pagination par offset.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BansQuery {
    pub patch: String,
    pub platform: String,
    pub queue: i32,
    #[serde(default = "default_rank")]
    pub rank: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}
impl BansQuery {
    /// Mêmes contrôles que les statistiques ; le rôle n'intervient pas dans les bans.
    pub(crate) fn as_stats_query(&self) -> StatsQuery {
        StatsQuery {
            patch: self.patch.clone(),
            platform: self.platform.clone(),
            queue: self.queue,
            role: "UNKNOWN".into(),
            rank: self.rank.clone(),
            offset: 0,
            limit: self.limit,
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        self.as_stats_query().validate()
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
    fn les_bans_n_acceptent_ni_role_ni_decalage_et_controlent_le_palier() {
        let q: BansQuery =
            serde_json::from_value(json!({"patch":"16.19","platform":"EUW1","queue":420})).unwrap();
        assert_eq!((q.rank.as_str(), q.limit), ("ALL", 50));
        assert!(q.validate().is_ok());
        for invalid in [
            BansQuery {
                rank: "FAKE".into(),
                ..q.clone()
            },
            BansQuery {
                platform: "EUROPE".into(),
                ..q.clone()
            },
            BansQuery {
                limit: 0,
                ..q.clone()
            },
            BansQuery {
                limit: 201,
                ..q.clone()
            },
        ] {
            assert!(invalid.validate().is_err(), "{invalid:?}");
        }
        for extra in ["role", "offset"] {
            let mut value = json!({"patch":"16.19","platform":"EUW1","queue":420});
            value[extra] = json!(1);
            assert!(
                serde_json::from_value::<BansQuery>(value).is_err(),
                "{extra}"
            );
        }
    }
}
