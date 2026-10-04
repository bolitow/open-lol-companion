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
/// Population demandée pour une série entre patchs : tout patch publié, jamais mélangé.
/// Aucune pagination : la série compte au plus un point par patch de l'instantané.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TrendsQuery {
    pub platform: String,
    pub queue: i32,
    pub role: String,
    #[serde(default = "default_rank")]
    pub rank: String,
}
fn is_patch(patch: &str) -> bool {
    let parts: Vec<_> = patch.split('.').collect();
    parts.len() == 2
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 3 && p.bytes().all(|b| b.is_ascii_digit()))
}
/// Dimensions communes à toutes les routes statistiques ; les bornes restent identiques.
fn are_dimensions(platform: &str, queue: i32, role: &str, rank: &str) -> bool {
    olc_collector::config::PLATFORMS.contains(&platform)
        && (1..=100_000).contains(&queue)
        && ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY", "UNKNOWN"].contains(&role)
        && (olc_collector::aggregation::CUMULATIVE_RANKS.contains(&rank)
            || [
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
            .contains(&rank))
}
impl StatsQuery {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !is_patch(&self.patch)
            || !are_dimensions(&self.platform, self.queue, &self.role, &self.rank)
            || self.offset > 10_000
            || !(1..=200).contains(&self.limit)
        {
            return Err("invalid_request");
        }
        Ok(())
    }
}
impl TrendsQuery {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !are_dimensions(&self.platform, self.queue, &self.role, &self.rank) {
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
    fn les_paliers_cumules_sont_acceptes_et_les_autres_suffixes_refuses() {
        let q: StatsQuery = serde_json::from_value(
            json!({"patch":"16.19","platform":"EUW1","queue":420,"role":"MIDDLE"}),
        )
        .unwrap();
        for rank in olc_collector::aggregation::CUMULATIVE_RANKS {
            let stats = StatsQuery {
                rank: rank.into(),
                ..q.clone()
            };
            assert!(stats.validate().is_ok(), "{rank}");
            let bans = BansQuery {
                patch: q.patch.clone(),
                platform: q.platform.clone(),
                queue: q.queue,
                rank: rank.into(),
                limit: 10,
            };
            assert!(bans.validate().is_ok(), "{rank}");
        }
        for rank in [
            "GRANDMASTER_PLUS",
            "CHALLENGER_PLUS",
            "ALL_PLUS",
            "UNKNOWN_PLUS",
            "emerald_plus",
            "EMERALD+",
            "PLUS",
        ] {
            let invalid = StatsQuery {
                rank: rank.into(),
                ..q.clone()
            };
            assert!(invalid.validate().is_err(), "{rank}");
        }
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

    #[test]
    fn les_tendances_controlent_les_memes_dimensions_sans_patch_ni_pagination() {
        let q: TrendsQuery =
            serde_json::from_value(json!({"platform":"EUW1","queue":420,"role":"MIDDLE"})).unwrap();
        assert_eq!(q.rank, "ALL");
        assert!(q.validate().is_ok());
        for invalid in [
            TrendsQuery {
                platform: "EUROPE".into(),
                ..q.clone()
            },
            TrendsQuery {
                queue: 0,
                ..q.clone()
            },
            TrendsQuery {
                role: "MID".into(),
                ..q.clone()
            },
            TrendsQuery {
                rank: "FAKE".into(),
                ..q.clone()
            },
        ] {
            assert!(invalid.validate().is_err(), "{invalid:?}");
        }
        // Une série couvre tous les patchs : le patch et la pagination sont refusés.
        for extra in [("patch", json!("16.19")), ("limit", json!(10))] {
            let mut value = json!({"platform":"EUW1","queue":420,"role":"MIDDLE"});
            value[extra.0] = extra.1;
            assert!(serde_json::from_value::<TrendsQuery>(value).is_err());
        }
    }
}
