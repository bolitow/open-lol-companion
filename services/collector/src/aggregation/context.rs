//! Winrate selon la durée de partie, le côté et les premiers objectifs (#119).
//!
//! Tout vient de données publiques de match-v5 : `gameDuration`, `teamId` et
//! `teams[].objectives.*.first`. Aucune ventilation n'est publiée pour Arena ni pour les
//! modes qui n'opposent pas les équipes 100 et 200 : le booléen `win` n'y désigne pas
//! une victoire de côté.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::GroupKey;

/// Équipe bleue et équipe rouge dans match-v5.
pub(super) const BLUE_TEAM: u32 = 100;
pub(super) const RED_TEAM: u32 = 200;

/// Axe d'une ligne `SplitStats`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitDimension {
    Duration,
    Side,
}

/// Tranche de durée (début inclus, fin exclue) ou côté. Les tranches suivent le ticket #119 :
/// moins de 20, 20-25, 25-30, 30-35, 35-40 et 40 minutes ou plus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SplitBucket {
    #[serde(rename = "lt_20")]
    Under20,
    #[serde(rename = "20_25")]
    From20To25,
    #[serde(rename = "25_30")]
    From25To30,
    #[serde(rename = "30_35")]
    From30To35,
    #[serde(rename = "35_40")]
    From35To40,
    #[serde(rename = "gte_40")]
    From40,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "red")]
    Red,
}

impl SplitBucket {
    /// Tranche d'une durée en secondes ; nulle si la durée est absente ou nulle, pour ne
    /// pas ranger une donnée inconnue parmi les parties très courtes.
    pub(super) fn from_duration_s(seconds: i32) -> Option<Self> {
        match seconds {
            i32::MIN..=0 => None,
            1..=1199 => Some(Self::Under20),
            1200..=1499 => Some(Self::From20To25),
            1500..=1799 => Some(Self::From25To30),
            1800..=2099 => Some(Self::From30To35),
            2100..=2399 => Some(Self::From35To40),
            _ => Some(Self::From40),
        }
    }

    pub(super) fn from_team(team: u32) -> Option<Self> {
        match team {
            BLUE_TEAM => Some(Self::Blue),
            RED_TEAM => Some(Self::Red),
            _ => None,
        }
    }

    pub fn dimension(self) -> SplitDimension {
        match self {
            Self::Blue | Self::Red => SplitDimension::Side,
            _ => SplitDimension::Duration,
        }
    }
}

/// Winrate d'un champion (population `GroupKey`) pour une tranche de durée ou un côté.
/// Le côté n'est publié que pour le rang `ALL` : en ventilant par rang, les effectifs
/// deviendraient trop faibles pour être lisibles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitStats {
    #[serde(flatten)]
    pub key: GroupKey,
    pub dimension: SplitDimension,
    pub bucket: SplitBucket,
    pub games: u64,
    pub wins: u64,
    /// Nul sous le seuil minimal du rapport.
    pub win_rate: Option<f64>,
    /// Borne inférieure de Wilson à 95 %, nulle sous le seuil.
    pub win_rate_lower_bound: Option<f64>,
}

/// Parties où une équipe a pris un premier objectif, et issue de ces parties. Une partie
/// n'est comptée que si exactement une équipe est marquée « première » ; sans objectif
/// pris (ou donnée contradictoire), elle n'apparaît pas ici.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FirstObjectiveStats {
    /// Parties où l'objectif a été pris en premier par une équipe identifiée.
    #[serde(default)]
    pub matches: u64,
    /// Parmi elles, victoires de l'équipe qui l'a pris en premier.
    #[serde(default)]
    pub wins: u64,
    /// Parties où l'équipe bleue l'a pris en premier.
    #[serde(default)]
    pub blue_matches: u64,
    /// Parmi elles, victoires de l'équipe bleue (la victoire rouge se déduit par différence).
    #[serde(default)]
    pub blue_wins: u64,
    /// Winrate de l'équipe qui prend l'objectif en premier ; nul sous le seuil.
    #[serde(default)]
    pub win_rate: Option<f64>,
    /// Winrate de l'équipe bleue quand elle le prend en premier ; nul sous le seuil.
    #[serde(default)]
    pub blue_win_rate: Option<f64>,
}

impl FirstObjectiveStats {
    pub(super) fn record(&mut self, blue_first: bool, first_team_won: bool) {
        self.matches += 1;
        self.wins += u64::from(first_team_won);
        if blue_first {
            self.blue_matches += 1;
            self.blue_wins += u64::from(first_team_won);
        }
    }

    pub(super) fn finish(&mut self, minimum: u64) {
        self.win_rate = super::model::rate(self.wins, self.matches, minimum);
        self.blue_win_rate = super::model::rate(self.blue_wins, self.blue_matches, minimum);
    }
}

/// Équipe ayant pris `objective` en premier (`teams[].objectives.<objective>.first`).
/// Exactement deux équipes 100 et 200, chacune avec un booléen, dont une seule vraie.
pub(super) fn first_team(detail: &Value, objective: &str) -> Option<u32> {
    let teams = detail["info"]["teams"].as_array()?;
    let [a, b] = teams.as_slice() else {
        return None;
    };
    let read = |team: &Value| {
        let id = u32::try_from(team["teamId"].as_u64()?).ok()?;
        Some((id, team["objectives"][objective]["first"].as_bool()?))
    };
    let (a, b) = (read(a)?, read(b)?);
    let ids = [a.0.min(b.0), a.0.max(b.0)];
    if ids != [BLUE_TEAM, RED_TEAM] || a.1 == b.1 {
        return None;
    }
    Some(if a.1 { a.0 } else { b.0 })
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
