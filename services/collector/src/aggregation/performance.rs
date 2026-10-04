//! Moyennes de performance post-partie par population (#100), sans identifiant de joueur.
//!
//! Seuls des agrégats sont publiés : aucune note, aucun benchmark, aucun radar.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::model::GroupKey;

/// Minutes de la timeline publiées ; d'autres repères pourront s'ajouter sans changer le format.
pub(super) const FRAME_MINUTES: [u32; 2] = [10, 15];
/// Intervalle Riot des frames quand `info.frameInterval` est absent (match-v5 : 60 000 ms).
const DEFAULT_FRAME_INTERVAL_MS: u64 = 60_000;

/// Définitions publiées avec le rapport : elles précèdent tout calcul (#100).
pub(super) const PERFORMANCE_METHOD: &str = "games = participations with kills, deaths, assists, \
totalDamageDealtToChampions, totalMinionsKilled, neutralMinionsKilled, goldEarned and visionScore \
as non-negative integers and a stored game duration > 0; kills/deaths/assists/damage_to_champions/\
vision_score = sum / games; kda = (sum kills + sum assists) / max(sum deaths, 1); \
cs = totalMinionsKilled + neutralMinionsKilled; cs_per_min and gold_per_min = sum / (sum game \
duration in s / 60); frames: first timeline frame with minute*60000 <= timestamp < \
minute*60000 + frameInterval, gold = totalGold, cs = minionsKilled + jungleMinionsKilled, \
xp = xp, mean over the participations having that frame; means are null below min_games";

/// Moyennes d'une population (patch × plateforme × file × rôle × rang × champion).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceStats {
    #[serde(flatten)]
    pub key: GroupKey,
    /// Toutes les participations de la population, complètes ou non (dénominateur de couverture).
    pub participations: u64,
    /// Participations dont toutes les valeurs de fin de partie sont exploitables.
    pub games: u64,
    pub kills: Option<f64>,
    pub deaths: Option<f64>,
    pub assists: Option<f64>,
    /// `(ΣK + ΣA) / max(ΣD, 1)`, calculé sur les sommes de la population.
    pub kda: Option<f64>,
    pub damage_to_champions: Option<f64>,
    pub cs_per_min: Option<f64>,
    pub gold_per_min: Option<f64>,
    pub vision_score: Option<f64>,
    /// Une entrée par minute de `FRAME_MINUTES`, dans l'ordre croissant.
    pub frames: Vec<PerformanceFrameStats>,
}

/// Valeurs moyennes à une minute donnée, avec leur propre effectif.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceFrameStats {
    pub minute: u32,
    pub games: u64,
    pub gold: Option<f64>,
    pub cs: Option<f64>,
    pub xp: Option<f64>,
}

/// Valeurs de fin de partie d'une participation (match-v5, `info.participants[]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EndOfGame {
    pub kills: u64,
    pub deaths: u64,
    pub assists: u64,
    pub damage_to_champions: u64,
    pub cs: u64,
    pub gold: u64,
    pub vision_score: u64,
    pub duration_s: u64,
}

/// Valeurs d'une participation à une minute de la timeline (`participantFrames`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FrameValues {
    pub gold: u64,
    pub cs: u64,
    pub xp: u64,
}

/// Une participation incomplète est écartée en bloc : aucune valeur n'est devinée.
pub(super) fn extract_end_of_game(participant: &Value, duration_s: i32) -> Option<EndOfGame> {
    let field = |key: &str| participant[key].as_u64();
    let duration_s = u64::try_from(duration_s).ok().filter(|d| *d > 0)?;
    Some(EndOfGame {
        kills: field("kills")?,
        deaths: field("deaths")?,
        assists: field("assists")?,
        damage_to_champions: field("totalDamageDealtToChampions")?,
        cs: field("totalMinionsKilled")?.checked_add(field("neutralMinionsKilled")?)?,
        gold: field("goldEarned")?,
        vision_score: field("visionScore")?,
        duration_s,
    })
}

/// Valeurs aux minutes de `FRAME_MINUTES`. La première frame de la fenêtre
/// `[minute, minute + frameInterval)` est retenue : Riot horodate la frame périodique
/// quelques millisecondes après la minute (600 278 ms observés) et ajoute une frame
/// finale à la fin de partie. Une minute sans frame exploitable est absente.
pub(super) fn extract_frames(timeline: &Value, participant_id: u32) -> BTreeMap<u32, FrameValues> {
    let mut result = BTreeMap::new();
    let Some(frames) = timeline["info"]["frames"].as_array() else {
        return result;
    };
    let interval = timeline["info"]["frameInterval"]
        .as_u64()
        .filter(|i| *i > 0)
        .unwrap_or(DEFAULT_FRAME_INTERVAL_MS);
    let key = participant_id.to_string();
    for minute in FRAME_MINUTES {
        let start = u64::from(minute) * 60_000;
        let Some(frame) = frames.iter().find(|f| {
            f["timestamp"]
                .as_u64()
                .is_some_and(|t| t >= start && t - start < interval)
        }) else {
            continue;
        };
        if let Some(values) = frame_values(&frame["participantFrames"][&key]) {
            result.insert(minute, values);
        }
    }
    result
}

fn frame_values(values: &Value) -> Option<FrameValues> {
    let field = |name: &str| values[name].as_u64();
    Some(FrameValues {
        gold: field("totalGold")?,
        cs: field("minionsKilled")?.checked_add(field("jungleMinionsKilled")?)?,
        xp: field("xp")?,
    })
}

#[derive(Default)]
struct FrameSums {
    games: u64,
    gold: u128,
    cs: u128,
    xp: u128,
}

/// Sommes entières d'une population : aucune perte d'arrondi avant la division finale.
#[derive(Default)]
pub(super) struct PerformanceSums {
    participations: u64,
    games: u64,
    kills: u128,
    deaths: u128,
    assists: u128,
    damage_to_champions: u128,
    cs: u128,
    gold: u128,
    vision_score: u128,
    duration_s: u128,
    frames: BTreeMap<u32, FrameSums>,
}

impl PerformanceSums {
    pub fn add(&mut self, end: Option<&EndOfGame>, frames: &BTreeMap<u32, FrameValues>) {
        self.participations += 1;
        if let Some(end) = end {
            self.games += 1;
            self.kills += u128::from(end.kills);
            self.deaths += u128::from(end.deaths);
            self.assists += u128::from(end.assists);
            self.damage_to_champions += u128::from(end.damage_to_champions);
            self.cs += u128::from(end.cs);
            self.gold += u128::from(end.gold);
            self.vision_score += u128::from(end.vision_score);
            self.duration_s += u128::from(end.duration_s);
        }
        for (minute, values) in frames {
            let sums = self.frames.entry(*minute).or_default();
            sums.games += 1;
            sums.gold += u128::from(values.gold);
            sums.cs += u128::from(values.cs);
            sums.xp += u128::from(values.xp);
        }
    }

    pub fn finish(self, key: GroupKey, min_games: u64) -> PerformanceStats {
        let enough = self.games >= min_games && self.games > 0;
        let mean = |sum: u128| enough.then(|| sum as f64 / self.games as f64);
        let minutes = self.duration_s as f64 / 60.0;
        let per_min = |sum: u128| enough.then(|| sum as f64 / minutes);
        let frames = FRAME_MINUTES
            .iter()
            .map(|minute| {
                let sums = self.frames.get(minute);
                let games = sums.map_or(0, |s| s.games);
                let mean = |pick: fn(&FrameSums) -> u128| {
                    sums.filter(|_| games >= min_games && games > 0)
                        .map(|s| pick(s) as f64 / games as f64)
                };
                PerformanceFrameStats {
                    minute: *minute,
                    games,
                    gold: mean(|s| s.gold),
                    cs: mean(|s| s.cs),
                    xp: mean(|s| s.xp),
                }
            })
            .collect();
        PerformanceStats {
            key,
            participations: self.participations,
            games: self.games,
            kills: mean(self.kills),
            deaths: mean(self.deaths),
            assists: mean(self.assists),
            kda: enough.then(|| (self.kills + self.assists) as f64 / self.deaths.max(1) as f64),
            damage_to_champions: mean(self.damage_to_champions),
            cs_per_min: per_min(self.cs),
            gold_per_min: per_min(self.gold),
            vision_score: mean(self.vision_score),
            frames,
        }
    }
}

#[cfg(test)]
#[path = "performance_tests.rs"]
mod tests;
