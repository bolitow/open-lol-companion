//! Matchups de lane (#123) : champion contre champion au même rôle, sans identifiant de joueur.
//!
//! Seuls des agrégats sont publiés : effectif, victoires, winrate et borne de Wilson,
//! nuls sous le seuil. Aucune recommandation ni décision de draft n'en est déduite.

use serde::{Deserialize, Serialize};

use super::model::{GroupKey, Role};

/// Files où `validate` garantit un seul participant par (équipe, rôle) : l'appariement de
/// lane y est fiable. Les autres files ne publient aucun matchup.
pub(super) const MATCHUP_QUEUES: [i32; 2] = [420, 440];

/// Définitions publiées avec le rapport : elles précèdent tout usage (#123).
pub(super) const MATCHUP_METHOD: &str = "lane matchups: queues 420 and 440 only; a pair is \
the two participants of opposite teams sharing the same teamPosition (TOP, JUNGLE, MIDDLE, \
BOTTOM, UTILITY), exactly one per team, UNKNOWN never paired; one row per (role, champion_id, \
opponent_champion_id) and per direction; rank = ALL only (no per-game rank yet); games = \
paired games, wins = games won by champion_id; win_rate = wins / games * 100 and \
win_rate_lower_bound = Wilson 95% lower bound, both null below min_games; \
coverage.lane_matchup_participations counts the paired participations";

/// Résultats d'un champion contre un adversaire de lane, dans une population
/// (patch × plateforme × file × rôle, rang `ALL`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchupStats {
    #[serde(flatten)]
    pub key: GroupKey,
    pub opponent_champion_id: u32,
    pub games: u64,
    pub wins: u64,
    pub losses: u64,
    /// Nul sous le seuil `min_games`.
    pub win_rate: Option<f64>,
    /// Borne inférieure de Wilson à 95 %, nulle sous le seuil.
    pub win_rate_lower_bound: Option<f64>,
}

/// Participation réduite à ce qui sert à l'appariement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LaneSlot {
    pub team: u32,
    pub role: Role,
    pub champion: u32,
}

/// Paires (sujet, adversaire) d'indices de `slots`, dans les deux sens. Un rôle n'est
/// apparié que s'il compte exactement deux participations, d'équipes différentes : rien
/// n'est deviné pour un rôle inconnu, absent ou en double.
pub(super) fn lane_opponents(queue_id: i32, slots: &[LaneSlot]) -> Vec<(usize, usize)> {
    if !MATCHUP_QUEUES.contains(&queue_id) {
        return vec![];
    }
    let mut pairs = vec![];
    for role in [
        Role::Top,
        Role::Jungle,
        Role::Middle,
        Role::Bottom,
        Role::Utility,
    ] {
        let lane: Vec<usize> = (0..slots.len())
            .filter(|i| slots[*i].role == role)
            .collect();
        if let [a, b] = lane[..] {
            if slots[a].team != slots[b].team {
                pairs.push((a, b));
                pairs.push((b, a));
            }
        }
    }
    pairs
}

#[cfg(test)]
#[path = "matchups_tests.rs"]
mod tests;
