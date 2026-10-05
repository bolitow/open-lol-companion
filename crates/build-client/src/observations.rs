//! Observations agrégées du groupe consulté, distinctes des variantes paginées.
use crate::{BuildError, BuildRequest};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
const MAX_SAFE: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationGroup {
    pub patch: String,
    pub platform_id: String,
    pub queue_id: u32,
    pub role: String,
    pub rank: String,
    pub champion_id: u32,
}
impl ObservationGroup {
    fn matches(&self, request: &BuildRequest) -> bool {
        self.patch == request.patch
            && self.platform_id == request.platform
            && self.queue_id == request.queue
            && self.role == request.role
            && self.rank == request.rank
            && self.champion_id == request.champion_id
    }
}
/// Résumé minimal du champion ; statistiques enrichies et classement restent dans #185.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildSummary {
    #[serde(flatten)]
    pub group: ObservationGroup,
    pub games: u64,
    pub wins: u64,
    pub losses: u64,
    pub population: u64,
    pub win_rate: Option<f64>,
    pub pick_rate: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillObservation {
    #[serde(flatten)]
    pub group: ObservationGroup,
    pub point: u32,
    pub slot: u32,
    pub games: u64,
    pub mean_timestamp_ms: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemObservation {
    #[serde(flatten)]
    pub group: ObservationGroup,
    pub event: String,
    pub item_id: u32,
    pub minute: u32,
    pub events: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OmittedBuildVariants {
    pub category: String,
    pub omitted: u64,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BuildDetails {
    pub summary: Option<BuildSummary>,
    pub skill_levels: Vec<SkillObservation>,
    pub item_events: Vec<ItemObservation>,
    pub omitted_build_variants: Option<u64>,
    pub omitted_build_variants_by_category: Vec<OmittedBuildVariants>,
    pub max_build_variants_per_category: Option<u32>,
    pub max_item_events: Option<u32>,
    pub omitted_item_events: Option<u64>,
}
impl BuildDetails {
    pub(crate) fn check(&mut self, request: &BuildRequest, minimum: u32) -> Result<(), BuildError> {
        let invalid = BuildError::InvalidResponse;
        if let Some(summary) = &mut self.summary {
            if !summary.group.matches(request)
                || summary.population > MAX_SAFE
                || summary.games > summary.population
                || summary.wins.checked_add(summary.losses) != Some(summary.games)
                || [summary.win_rate, summary.pick_rate]
                    .into_iter()
                    .flatten()
                    .any(|rate| !rate.is_finite() || !(0.0..=100.0).contains(&rate))
            {
                return Err(invalid);
            }
            if summary.games < u64::from(minimum) {
                summary.win_rate = None;
                summary.pick_rate = None;
            }
        }
        let max_items = self.max_item_events.unwrap_or(2000);
        if max_items > 10_000
            || self.item_events.len() > max_items as usize
            || self.skill_levels.len() > 256
            || self.omitted_build_variants_by_category.len() > 128
            || self.omitted_build_variants.is_some_and(|n| n > MAX_SAFE)
            || self.omitted_item_events.is_some_and(|n| n > MAX_SAFE)
        {
            return Err(invalid);
        }
        let mut skills = HashSet::new();
        for row in &self.skill_levels {
            if !row.group.matches(request)
                || !(1..=64).contains(&row.point)
                || !(1..=4).contains(&row.slot)
                || row.games > MAX_SAFE
                || !row.mean_timestamp_ms.is_finite()
                || !(0.0..=MAX_SAFE as f64).contains(&row.mean_timestamp_ms)
                || !skills.insert((row.point, row.slot))
            {
                return Err(invalid);
            }
        }
        let mut events = HashSet::new();
        for row in &self.item_events {
            if !row.group.matches(request)
                || row.event.is_empty()
                || row.event.len() > 64
                || row.item_id == 0
                || row.events > MAX_SAFE
                || !events.insert((&row.event, row.item_id, row.minute))
            {
                return Err(invalid);
            }
        }
        let mut categories = HashSet::new();
        let mut total = 0_u64;
        for row in &self.omitted_build_variants_by_category {
            if row.category.is_empty()
                || row.category.len() > 64
                || row.omitted > MAX_SAFE
                || !categories.insert(&row.category)
            {
                return Err(invalid);
            }
            total = total
                .checked_add(row.omitted)
                .filter(|n| *n <= MAX_SAFE)
                .ok_or(invalid)?;
        }
        if !self.omitted_build_variants_by_category.is_empty() {
            if self
                .omitted_build_variants
                .is_some_and(|known| known != total)
            {
                return Err(invalid);
            }
        } else if self.omitted_build_variants.is_some_and(|n| n > 0) {
            // Ancien serveur : ce compteur pouvait être global. Ne jamais l’attribuer au champion.
            self.omitted_build_variants = None;
        }
        Ok(())
    }
}
