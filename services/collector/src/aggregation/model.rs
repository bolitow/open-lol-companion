use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::builds::{self, BuildObservation};
use super::matchups::{self, LaneSlot, MatchupStats, MATCHUP_METHOD};
use super::performance::{self, PerformanceStats, PerformanceSums, PERFORMANCE_METHOD};
use super::stages::{ItemCatalog, STAGE_CATEGORIES, STAGE_METHOD};
use super::AggregationError;
use crate::model::patch_from_version;

/// Écart maximal par défaut entre le début de la partie et l'observation de rang retenue.
pub const DEFAULT_RANK_MAX_AGE_HOURS: u32 = 168;
/// Borne haute de l'écart configurable : au-delà, le rang ne décrit plus la partie.
pub const MAX_RANK_MAX_AGE_HOURS: u32 = 8760;

/// Durée minimale par défaut (secondes) d'une partie classée : en dessous, la partie est écartée.
pub const DEFAULT_MIN_GAME_DURATION_S: u32 = 300;
/// Borne haute de la durée minimale configurable : au-delà, des parties classées légitimes
/// (reddition possible dès 15 minutes) seraient écartées.
pub const MAX_MIN_GAME_DURATION_S: u32 = 900;
/// Part minimale (%) de la durée d'une partie classée que chaque participant doit avoir jouée.
pub const DEFAULT_MIN_PLAYED_PERCENT: u32 = 80;
/// Files concernées par les contrôles de qualité : Solo/Duo et Flex.
const QUALITY_QUEUES: [i32; 2] = [420, 440];

/// Seuils des contrôles de qualité des parties classées (#111). Zéro désactive le contrôle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualityThresholds {
    /// Durée minimale de la partie, en secondes (`short_game` en dessous).
    pub min_game_duration_s: u32,
    /// Part minimale, en %, de la durée jouée par chaque participant (`early_departure`).
    pub min_played_percent: u32,
    /// Écarte la partie dès qu'un participant porte `wasAfk = true` (`afk`).
    pub exclude_afk: bool,
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            min_game_duration_s: DEFAULT_MIN_GAME_DURATION_S,
            min_played_percent: DEFAULT_MIN_PLAYED_PERCENT,
            exclude_afk: true,
        }
    }
}

impl QualityThresholds {
    pub fn validate(&self) -> Result<(), AggregationError> {
        if self.min_game_duration_s > MAX_MIN_GAME_DURATION_S {
            return Err(AggregationError::InvalidMinGameDuration);
        }
        if self.min_played_percent > 100 {
            return Err(AggregationError::InvalidMinPlayedPercent);
        }
        Ok(())
    }
}

/// Rôle fourni par Riot ; UNKNOWN conserve les participations sans rôle exploitable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Top,
    Jungle,
    Middle,
    Bottom,
    Utility,
    Unknown,
}

/// Filtres d'un instantané. Les listes vides acceptent toutes les valeurs stockées.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AggregationOptions {
    pub patches: Vec<String>,
    pub platforms: Vec<String>,
    pub queues: Vec<i32>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
}

/// Dimensions sans identifiant de joueur. ALL est une population distincte des rangs observés.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GroupKey {
    pub patch: String,
    pub platform_id: String,
    pub queue_id: i32,
    pub role: Role,
    pub rank: String,
    pub champion_id: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChampionStats {
    #[serde(flatten)]
    pub key: GroupKey,
    pub games: u64,
    pub wins: u64,
    pub losses: u64,
    /// Participations de tous les champions du même patch/plateforme/file/rôle/rang.
    pub population: u64,
    pub win_rate: Option<f64>,
    pub pick_rate: Option<f64>,
    pub win_rate_lower_bound: Option<f64>,
    pub position: Option<u32>,
    pub tier: Option<String>,
    /// Rang connu comptant le plus de sélections ; ce n'est pas un taux de popularité corrigé.
    pub most_picked_rank: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ScopeKey {
    pub patch: String,
    pub platform_id: String,
    pub queue_id: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BanStats {
    #[serde(flatten)]
    pub scope: ScopeKey,
    pub champion_id: u32,
    pub banned_matches: u64,
    pub draft_matches: u64,
    pub ban_rate: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildStats {
    #[serde(flatten)]
    pub key: GroupKey,
    pub category: String,
    pub selection: Vec<u32>,
    pub games: u64,
    pub wins: Option<u64>,
    pub performance_available: bool,
    pub population: u64,
    pub pick_rate: Option<f64>,
    pub win_rate: Option<f64>,
    /// Borne inférieure de Wilson à 95 %, nulle sous le seuil ou sans performance publiable.
    #[serde(default)]
    pub win_rate_lower_bound: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillStats {
    #[serde(flatten)]
    pub key: GroupKey,
    /// Rang du point investi, distinct du niveau du champion.
    pub point: u32,
    pub slot: u32,
    pub games: u64,
    pub mean_timestamp_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemEventStats {
    #[serde(flatten)]
    pub key: GroupKey,
    pub event: String,
    pub item_id: u32,
    pub minute: u32,
    pub events: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Coverage {
    pub matches: u64,
    pub participations: u64,
    pub excluded_bot_participations: u64,
    pub ranked_participations: u64,
    pub unranked_participations: u64,
    pub unknown_rank_participations: u64,
    pub unranked_mode_participations: u64,
    pub unknown_role_participations: u64,
    pub timeline_matches: u64,
    pub timeline_participations: u64,
    pub invalid_timeline_participations: u64,
    pub unidentified_item_undos: u64,
    pub draft_matches: u64,
    /// Part des participations Solo/Flex sans rang attribuable (%), nulle hors files classées.
    #[serde(default)]
    pub unknown_rank_rate: Option<f64>,
    /// Écart médian, en heures, entre début de partie et observation de rang retenue.
    #[serde(default)]
    pub rank_gap_median_hours: Option<f64>,
    /// Écart maximal retenu, en heures ; toujours inférieur ou égal à `rank_max_age_hours`.
    #[serde(default)]
    pub rank_gap_max_hours: Option<f64>,
    /// Participations dont les étapes d'achat (#81) ont été dérivées du catalogue du patch.
    #[serde(default)]
    pub item_stage_participations: u64,
    /// Participations à achats nets connus, sans catalogue d'objets publié pour leur patch.
    #[serde(default)]
    pub missing_item_catalog_participations: u64,
    /// Participations appariées à un adversaire de lane (#123) ; 0 hors files 420 et 440.
    #[serde(default)]
    pub lane_matchup_participations: u64,
}

/// Version du catalogue normalisé (#61) jointe à un patch agrégé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCatalogRef {
    pub patch: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopeCoverage {
    #[serde(flatten)]
    pub scope: ScopeKey,
    #[serde(flatten)]
    pub counts: Coverage,
}

/// Instantané publiable, exclusivement agrégé. Le rang d'une participation est l'observation
/// la plus proche du début de sa partie, dans la limite de `rank_max_age_hours`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregationReport {
    pub schema_version: u32,
    pub rank_scope: String,
    pub rank_max_age_hours: u32,
    /// Durée minimale (s) d'une partie classée (#111) ; 0 pour un rapport antérieur ou sans contrôle.
    #[serde(default)]
    pub min_game_duration_s: u32,
    /// Part minimale (%) de la durée jouée par chaque participant (#111) ; 0 sans contrôle.
    #[serde(default)]
    pub min_played_percent: u32,
    /// Parties classées avec un participant `wasAfk` exclues (#111) ; `false` pour un rapport antérieur.
    #[serde(default)]
    pub exclude_afk: bool,
    pub pick_rate_definition: String,
    pub tier_method: String,
    pub min_games: u32,
    pub filters: AggregationOptions,
    pub source_matches: u64,
    pub included_matches: u64,
    pub exclusions: BTreeMap<String, u64>,
    pub coverage: Vec<ScopeCoverage>,
    pub groups: Vec<ChampionStats>,
    pub bans: Vec<BanStats>,
    pub builds: Vec<BuildStats>,
    pub skill_levels: Vec<SkillStats>,
    pub item_events: Vec<ItemEventStats>,
    pub max_build_variants_per_category: u32,
    pub omitted_build_variants: u64,
    /// Règles des catégories d'étapes (#81) ; vide dans les rapports antérieurs.
    #[serde(default)]
    pub build_stage_method: String,
    #[serde(default)]
    pub item_catalogs: Vec<ItemCatalogRef>,
    /// Définitions des moyennes de performance (#100) ; vide dans les rapports antérieurs.
    #[serde(default)]
    pub performance_method: String,
    /// Moyennes de performance par population (#100) ; vide dans les rapports antérieurs.
    #[serde(default)]
    pub performance: Vec<PerformanceStats>,
    /// Définitions des matchups de lane (#123) ; vide dans les rapports antérieurs.
    #[serde(default)]
    pub matchup_method: String,
    /// Matchups de lane par rôle, rang `ALL` seulement (#123) ; vide dans les rapports antérieurs.
    #[serde(default)]
    pub matchups: Vec<MatchupStats>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ObservedRank {
    pub status: String,
    pub tier: Option<String>,
    /// Écart absolu, en secondes, entre le début de la partie et cette observation.
    pub gap_s: u64,
}

pub(super) struct StoredMatch {
    pub match_id: String,
    pub platform_id: String,
    pub queue_id: i32,
    pub patch: String,
    pub is_remake: bool,
    /// Durée stockée de la partie, en secondes (normalisée à la collecte).
    pub game_duration_s: i32,
    pub detail: Value,
    pub timeline: Option<Value>,
    pub ranks: BTreeMap<String, ObservedRank>,
}

type Population = (ScopeKey, Role, String);
type BuildKey = (GroupKey, String, Vec<u32>);
#[derive(Default)]
struct Count {
    games: u64,
    wins: u64,
}

pub(super) struct Accumulator {
    report: AggregationReport,
    counts: BTreeMap<GroupKey, Count>,
    arena_scopes: BTreeSet<ScopeKey>,
    populations: BTreeMap<Population, u64>,
    coverage: BTreeMap<ScopeKey, Coverage>,
    bans: BTreeMap<(ScopeKey, u32), u64>,
    builds: BTreeMap<BuildKey, Count>,
    build_populations: BTreeMap<(GroupKey, String), u64>,
    skills: BTreeMap<(GroupKey, u32, u32), (u64, u128)>,
    events: BTreeMap<(GroupKey, String, u32, u32), u64>,
    /// Écarts partie → observation des rangs retenus, en secondes, par périmètre.
    rank_gaps: BTreeMap<ScopeKey, Vec<u64>>,
    /// Catalogue d'objets par patch (« 16.19 »), joint pour dériver les étapes (#81).
    item_catalogs: BTreeMap<String, ItemCatalog>,
    /// Sommes des valeurs de performance (#100), mêmes clés que `counts`.
    performance: BTreeMap<GroupKey, PerformanceSums>,
    /// Matchups de lane (#123) : (population du sujet, champion adverse).
    matchups: BTreeMap<(GroupKey, u32), Count>,
}

impl Accumulator {
    pub fn new(min_games: u32) -> Result<Self, AggregationError> {
        if min_games == 0 {
            return Err(AggregationError::InvalidThreshold);
        }
        Ok(Self {
            report: AggregationReport {
                schema_version: 2, rank_scope: "observed_rank_nearest_to_game_start_of_same_ranked_queue".into(),
                rank_max_age_hours: DEFAULT_RANK_MAX_AGE_HOURS,
                min_game_duration_s: DEFAULT_MIN_GAME_DURATION_S,
                min_played_percent: DEFAULT_MIN_PLAYED_PERCENT,
                exclude_afk: true,
                pick_rate_definition: "champion_participations / bucket_participations * 100".into(),
                tier_method: "Wilson95 lower bound; S/A/B/C/D percentiles 10/30/60/90/100; at least 5 eligible champions".into(),
                min_games, filters: AggregationOptions::default(), source_matches: 0,
                included_matches: 0, exclusions: BTreeMap::new(), coverage: vec![], groups: vec![],
                bans: vec![], builds: vec![], skill_levels: vec![], item_events: vec![],
                max_build_variants_per_category: 20, omitted_build_variants: 0,
                build_stage_method: STAGE_METHOD.into(), item_catalogs: vec![],
                performance_method: PERFORMANCE_METHOD.into(), performance: vec![],
                matchup_method: MATCHUP_METHOD.into(), matchups: vec![],
            },
            counts:BTreeMap::new(), arena_scopes:BTreeSet::new(), populations:BTreeMap::new(), coverage:BTreeMap::new(),
            bans:BTreeMap::new(), builds:BTreeMap::new(), build_populations:BTreeMap::new(),
            skills:BTreeMap::new(), events:BTreeMap::new(), rank_gaps:BTreeMap::new(),
            item_catalogs:BTreeMap::new(), performance:BTreeMap::new(), matchups:BTreeMap::new(),
        })
    }

    pub fn set_rank_max_age_hours(&mut self, hours: u32) -> Result<(), AggregationError> {
        if !(1..=MAX_RANK_MAX_AGE_HOURS).contains(&hours) {
            return Err(AggregationError::InvalidRankMaxAge);
        }
        self.report.rank_max_age_hours = hours;
        Ok(())
    }

    /// Seuils des contrôles de qualité des parties classées (#111), publiés dans le rapport.
    pub fn set_quality_thresholds(
        &mut self,
        thresholds: &QualityThresholds,
    ) -> Result<(), AggregationError> {
        thresholds.validate()?;
        self.report.min_game_duration_s = thresholds.min_game_duration_s;
        self.report.min_played_percent = thresholds.min_played_percent;
        self.report.exclude_afk = thresholds.exclude_afk;
        Ok(())
    }

    pub fn set_filters(&mut self, filters: AggregationOptions) {
        self.report.filters = filters;
    }

    /// Un patch absent de `catalogs` ne reçoit aucune étape : rien n'est deviné.
    pub fn set_item_catalogs(&mut self, catalogs: BTreeMap<String, ItemCatalog>) {
        self.report.item_catalogs = catalogs
            .iter()
            .map(|(patch, catalog)| ItemCatalogRef {
                patch: patch.clone(),
                version: catalog.version.clone(),
            })
            .collect();
        self.item_catalogs = catalogs;
    }

    pub fn add(&mut self, game: &StoredMatch) {
        self.report.source_matches += 1;
        let thresholds = QualityThresholds {
            min_game_duration_s: self.report.min_game_duration_s,
            min_played_percent: self.report.min_played_percent,
            exclude_afk: self.report.exclude_afk,
        };
        let participants =
            match validate(game).and_then(|p| quality_check(game, &p, thresholds).map(|()| p)) {
                Ok(p) => p,
                Err(reason) => {
                    *self.report.exclusions.entry(reason.into()).or_default() += 1;
                    return;
                }
            };
        self.report.included_matches += 1;
        let scope = ScopeKey {
            patch: game.patch.clone(),
            platform_id: game.platform_id.clone(),
            queue_id: game.queue_id,
        };
        // Politique Riot : aucun winrate d'item Arena dans le rapport publiable.
        // https://developer.riotgames.com/docs/lol#game-policy
        if game.detail["info"]["gameMode"] == "CHERRY"
            || [1700, 1710, 1740, 1750].contains(&game.queue_id)
        {
            self.arena_scopes.insert(scope.clone());
        }
        let coverage = self.coverage.entry(scope.clone()).or_default();
        coverage.matches += 1;
        if let Some(bans) = valid_bans(&game.detail) {
            coverage.draft_matches += 1;
            for champion in bans {
                *self.bans.entry((scope.clone(), champion)).or_default() += 1;
            }
        }
        let mut timeline_counted = false;
        // Appariement de lane (#123) après la boucle, qui consomme les participations.
        let (mut slots, mut wins) = (vec![], vec![]);
        for p in participants {
            if is_coop(game.queue_id) && is_bot(&p.raw) {
                self.coverage
                    .entry(scope.clone())
                    .or_default()
                    .excluded_bot_participations += 1;
                continue;
            }
            slots.push(LaneSlot {
                team: p.team,
                role: p.role,
                champion: p.champion,
            });
            wins.push(p.win);
            let max_age_s = u64::from(self.report.rank_max_age_hours) * 3600;
            let (rank, gap) = rank_for(game, &p.raw, max_age_s);
            if let Some(gap) = gap {
                self.rank_gaps.entry(scope.clone()).or_default().push(gap);
            }
            let coverage = self.coverage.entry(scope.clone()).or_default();
            coverage.participations += 1;
            match rank.as_str() {
                "UNKNOWN" => coverage.unknown_rank_participations += 1,
                "UNRANKED" => coverage.unranked_participations += 1,
                "UNRANKED_MODE" => coverage.unranked_mode_participations += 1,
                _ => coverage.ranked_participations += 1,
            }
            if p.role == Role::Unknown {
                coverage.unknown_role_participations += 1;
            }
            let mut observations = builds::extract_detail(&p.raw);
            let end_of_game = performance::extract_end_of_game(&p.raw, game.game_duration_s);
            let mut frames = BTreeMap::new();
            if let Some(timeline) = &game.timeline {
                let participant_id = p.raw["participantId"]
                    .as_u64()
                    .and_then(|id| u32::try_from(id).ok());
                let result = participant_id
                    .ok_or("missing_participant_id")
                    .and_then(|id| builds::extract_timeline(timeline, &game.match_id, id));
                match result {
                    Ok(t) => {
                        // Timeline déjà validée (partie, participant) : mêmes règles de couverture.
                        if let Some(id) = participant_id {
                            frames = performance::extract_frames(timeline, id);
                        }
                        coverage.unidentified_item_undos += t.unidentified_item_undos;
                        coverage.timeline_participations += 1;
                        if !timeline_counted {
                            coverage.timeline_matches += 1;
                            timeline_counted = true;
                        }
                        observations.variants.extend(t.variants);
                        observations.skill_steps = t.skill_steps;
                        observations.item_events = t.item_events;
                        if let Some(purchases) = t.net_purchases {
                            match self.item_catalogs.get(&game.patch) {
                                Some(catalog) => {
                                    observations
                                        .variants
                                        .extend(catalog.derive_steps(&purchases));
                                    coverage.item_stage_participations += 1;
                                }
                                None => coverage.missing_item_catalog_participations += 1,
                            }
                        }
                    }
                    Err(_) => coverage.invalid_timeline_participations += 1,
                }
            }
            for rank in ["ALL".to_owned(), rank] {
                let key = GroupKey {
                    patch: game.patch.clone(),
                    platform_id: game.platform_id.clone(),
                    queue_id: game.queue_id,
                    role: p.role,
                    rank: rank.clone(),
                    champion_id: p.champion,
                };
                let c = self.counts.entry(key.clone()).or_default();
                c.games += 1;
                c.wins += u64::from(p.win);
                *self
                    .populations
                    .entry((scope.clone(), p.role, rank))
                    .or_default() += 1;
                self.performance.entry(key.clone()).or_default().add(
                    game.game_duration_s,
                    end_of_game.as_ref(),
                    &frames,
                );
                self.add_builds(&key, p.win, &observations);
            }
        }
        let pairs = matchups::lane_opponents(game.queue_id, &slots);
        self.coverage
            .entry(scope)
            .or_default()
            .lane_matchup_participations += pairs.len() as u64;
        for (subject, opponent) in pairs {
            // Rang ALL seulement : le rang individuel rendrait « A contre B » et
            // « B contre A » non complémentaires ; le rang de partie n'existe pas encore.
            let key = GroupKey {
                patch: game.patch.clone(),
                platform_id: game.platform_id.clone(),
                queue_id: game.queue_id,
                role: slots[subject].role,
                rank: "ALL".into(),
                champion_id: slots[subject].champion,
            };
            let c = self
                .matchups
                .entry((key, slots[opponent].champion))
                .or_default();
            c.games += 1;
            c.wins += u64::from(wins[subject]);
        }
    }

    fn add_builds(&mut self, key: &GroupKey, win: bool, observations: &BuildObservation) {
        for (category, selection) in &observations.variants {
            *self
                .build_populations
                .entry((key.clone(), category.clone()))
                .or_default() += 1;
            let c = self
                .builds
                .entry((key.clone(), category.clone(), selection.clone()))
                .or_default();
            c.games += 1;
            c.wins += u64::from(win);
            if category == "final_items" {
                *self
                    .build_populations
                    .entry((key.clone(), "item".into()))
                    .or_default() += 1;
                for item in selection {
                    let c = self
                        .builds
                        .entry((key.clone(), "item".into(), vec![*item]))
                        .or_default();
                    c.games += 1;
                    c.wins += u64::from(win);
                }
            }
        }
        for step in &observations.skill_steps {
            let (n, sum) = self
                .skills
                .entry((key.clone(), step.level, step.slot))
                .or_default();
            *n += 1;
            *sum += u128::from(step.timestamp_ms);
        }
        for event in &observations.item_events {
            let minute = u32::try_from(event.timestamp_ms / 60_000).unwrap_or(u32::MAX);
            *self
                .events
                .entry((key.clone(), event.kind.clone(), event.item_id, minute))
                .or_default() += 1;
        }
    }

    pub fn finish(mut self) -> AggregationReport {
        let minimum = u64::from(self.report.min_games);
        let mut popular: BTreeMap<(ScopeKey, Role, u32), (u64, String)> = BTreeMap::new();
        for (key, c) in &self.counts {
            if is_ranked_tier(&key.rank) {
                let best = popular
                    .entry((scope_of(key), key.role, key.champion_id))
                    .or_default();
                if c.games > best.0 || (c.games == best.0 && key.rank < best.1) {
                    *best = (c.games, key.rank.clone());
                }
            }
        }
        self.report.groups = self
            .counts
            .into_iter()
            .map(|(key, c)| {
                let population = self.populations[&(scope_of(&key), key.role, key.rank.clone())];
                let most_picked_rank = popular
                    .get(&(scope_of(&key), key.role, key.champion_id))
                    .map(|(_, r)| r.clone());
                ChampionStats {
                    key,
                    games: c.games,
                    wins: c.wins,
                    losses: c.games - c.wins,
                    population,
                    win_rate: rate(c.wins, c.games, minimum),
                    pick_rate: rate(c.games, population, minimum).filter(|_| c.games >= minimum),
                    win_rate_lower_bound: (c.games >= minimum).then(|| wilson(c.wins, c.games)),
                    position: None,
                    tier: None,
                    most_picked_rank,
                }
            })
            .collect();
        let bucket = |key: &GroupKey| (scope_of(key), key.role, key.rank.clone());
        self.report.groups.sort_by(|a, b| {
            bucket(&a.key)
                .cmp(&bucket(&b.key))
                .then_with(|| {
                    b.win_rate_lower_bound
                        .unwrap_or(-1.0)
                        .total_cmp(&a.win_rate_lower_bound.unwrap_or(-1.0))
                })
                .then_with(|| {
                    (u128::from(b.wins) * u128::from(a.games))
                        .cmp(&(u128::from(a.wins) * u128::from(b.games)))
                })
                .then_with(|| b.games.cmp(&a.games))
                .then_with(|| a.key.champion_id.cmp(&b.key.champion_id))
        });
        let mut eligible = BTreeMap::<Population, u32>::new();
        for g in &self.report.groups {
            if g.win_rate.is_some() {
                *eligible.entry(bucket(&g.key)).or_default() += 1;
            }
        }
        let mut positions = BTreeMap::<Population, u32>::new();
        for g in &mut self.report.groups {
            if g.win_rate.is_some() {
                let b = bucket(&g.key);
                let position = positions.entry(b.clone()).or_default();
                *position += 1;
                g.position = Some(*position);
                if eligible[&b] >= 5 {
                    let pct = 100 * (*position - 1) / eligible[&b];
                    g.tier = Some(
                        match pct {
                            0..=9 => "S",
                            10..=29 => "A",
                            30..=59 => "B",
                            60..=89 => "C",
                            _ => "D",
                        }
                        .into(),
                    );
                }
            }
        }
        self.report.bans = self
            .bans
            .into_iter()
            .map(|((scope, champion_id), banned_matches)| {
                let draft_matches = self.coverage[&scope].draft_matches;
                BanStats {
                    scope,
                    champion_id,
                    banned_matches,
                    draft_matches,
                    ban_rate: rate(banned_matches, draft_matches, minimum),
                }
            })
            .collect();
        let mut builds: Vec<_> = self
            .builds
            .into_iter()
            .map(|((key, category, selection), c)| {
                let population = self.build_populations[&(key.clone(), category.clone())];
                let performance_available = !(self.arena_scopes.contains(&scope_of(&key))
                    && (matches!(
                        category.as_str(),
                        "item" | "final_items" | "trinket" | "purchase_order"
                    ) || STAGE_CATEGORIES.contains(&category.as_str())));
                BuildStats {
                    key,
                    category,
                    selection,
                    games: c.games,
                    wins: performance_available.then_some(c.wins),
                    performance_available,
                    population,
                    pick_rate: rate(c.games, population, minimum).filter(|_| c.games >= minimum),
                    win_rate: if performance_available {
                        rate(c.wins, c.games, minimum)
                    } else {
                        None
                    },
                    win_rate_lower_bound: (performance_available && c.games >= minimum)
                        .then(|| wilson(c.wins, c.games)),
                }
            })
            .collect();
        builds.sort_by(|a, b| {
            (&a.key, &a.category)
                .cmp(&(&b.key, &b.category))
                .then_with(|| b.games.cmp(&a.games))
                .then_with(|| b.wins.cmp(&a.wins))
                .then_with(|| a.selection.cmp(&b.selection))
        });
        let mut variants = BTreeMap::<(GroupKey, String), u32>::new();
        for build in builds {
            let n = variants
                .entry((build.key.clone(), build.category.clone()))
                .or_default();
            *n += 1;
            if *n <= self.report.max_build_variants_per_category {
                self.report.builds.push(build);
            } else {
                self.report.omitted_build_variants += 1;
            }
        }
        self.report.skill_levels = self
            .skills
            .into_iter()
            .map(|((key, point, slot), (games, time))| SkillStats {
                key,
                point,
                slot,
                games,
                mean_timestamp_ms: time as f64 / games as f64,
            })
            .collect();
        self.report.item_events = self
            .events
            .into_iter()
            .map(|((key, event, item_id, minute), events)| ItemEventStats {
                key,
                event,
                item_id,
                minute,
                events,
            })
            .collect();
        self.report.performance = self
            .performance
            .into_iter()
            .map(|(key, sums)| sums.finish(key, minimum))
            .collect();
        self.report.matchups = self
            .matchups
            .into_iter()
            .map(|((key, opponent_champion_id), c)| MatchupStats {
                key,
                opponent_champion_id,
                games: c.games,
                wins: c.wins,
                losses: c.games - c.wins,
                win_rate: rate(c.wins, c.games, minimum),
                win_rate_lower_bound: (c.games >= minimum).then(|| wilson(c.wins, c.games)),
            })
            .collect();
        let mut rank_gaps = self.rank_gaps;
        self.report.coverage = self
            .coverage
            .into_iter()
            .map(|(scope, mut counts)| {
                let ranked_queue = counts.participations - counts.unranked_mode_participations;
                counts.unknown_rank_rate =
                    rate(counts.unknown_rank_participations, ranked_queue, 1);
                let mut gaps = rank_gaps.remove(&scope).unwrap_or_default();
                gaps.sort_unstable();
                counts.rank_gap_median_hours = median(&gaps).map(|s| s / 3600.0);
                counts.rank_gap_max_hours = gaps.last().map(|s| *s as f64 / 3600.0);
                ScopeCoverage { scope, counts }
            })
            .collect();
        self.report
    }
}

fn rate(n: u64, d: u64, min: u64) -> Option<f64> {
    (d >= min && d > 0).then(|| 100.0 * n as f64 / d as f64)
}
/// Médiane d'une liste triée ; moyenne des deux valeurs centrales si l'effectif est pair.
fn median(sorted: &[u64]) -> Option<f64> {
    let mid = sorted.len() / 2;
    match sorted.len() {
        0 => None,
        n if n % 2 == 1 => Some(sorted[mid] as f64),
        _ => Some((sorted[mid - 1] as f64 + sorted[mid] as f64) / 2.0),
    }
}
/// Borne inférieure de Wilson à 95 %, en pourcentage. Bornée à 0..=100 : pour 0 victoire,
/// l'arrondi flottant donne parfois une valeur infime négative (≈ -1e-16), que le client
/// desktop rejette avec toute la page.
fn wilson(wins: u64, games: u64) -> f64 {
    let n = games as f64;
    let p = wins as f64 / n;
    let z = 1.959963984540054_f64;
    let z2 = z * z;
    let bound = 100.0 * (p + z2 / (2.0 * n) - z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt())
        / (1.0 + z2 / n);
    bound.clamp(0.0, 100.0)
}
fn scope_of(key: &GroupKey) -> ScopeKey {
    ScopeKey {
        patch: key.patch.clone(),
        platform_id: key.platform_id.clone(),
        queue_id: key.queue_id,
    }
}
fn is_ranked_tier(tier: &str) -> bool {
    matches!(
        tier,
        "IRON"
            | "BRONZE"
            | "SILVER"
            | "GOLD"
            | "PLATINUM"
            | "EMERALD"
            | "DIAMOND"
            | "MASTER"
            | "GRANDMASTER"
            | "CHALLENGER"
    )
}
/// Rang figé à la partie : l'observation la plus proche du début, si son écart reste
/// dans la limite. L'heure du calcul n'intervient pas. Renvoie aussi l'écart retenu.
fn rank_for(game: &StoredMatch, raw: &Value, max_age_s: u64) -> (String, Option<u64>) {
    if ![420, 440].contains(&game.queue_id) {
        return ("UNRANKED_MODE".into(), None);
    }
    let observed = raw["puuid"]
        .as_str()
        .and_then(|id| game.ranks.get(id))
        .filter(|r| r.gap_s <= max_age_s);
    let rank = match observed {
        Some(r) if r.status == "unranked" => Some("UNRANKED".to_owned()),
        Some(r) if r.status == "ranked" => r.tier.clone().filter(|t| is_ranked_tier(t)),
        _ => None,
    };
    match rank {
        Some(rank) => (rank, observed.map(|r| r.gap_s)),
        None => ("UNKNOWN".into(), None),
    }
}
struct Participant {
    raw: Value,
    champion: u32,
    /// `teamId` (sous-équipe en Arena), déjà contrôlé par `validate`.
    team: u32,
    role: Role,
    win: bool,
}
fn number(raw: &Value, key: &str) -> Option<u32> {
    raw[key].as_u64().and_then(|v| u32::try_from(v).ok())
}
fn is_coop(queue: i32) -> bool {
    [830, 840, 850, 870, 880, 890].contains(&queue)
}
fn is_bot(participant: &Value) -> bool {
    participant["puuid"]
        .as_str()
        .is_some_and(|p| p == "BOT" || (!p.is_empty() && p.bytes().all(|b| b == b'0')))
}
/// Contrôles de qualité des files classées, après la validité structurelle (#111).
/// Ordre des motifs : `remake`, `invalid_match`, puis `short_game`, `afk` et enfin
/// `early_departure` ; une partie n'est comptée que sous le premier motif rencontré.
/// `afk` : au moins un participant a `wasAfk = true` (champ match-v5 déjà stocké).
/// Une clé absente n'est pas jugée ; un type invalide rend la partie incohérente, mais
/// seulement si le contrôle correspondant est actif. Une reddition normale n'est jamais
/// un motif d'exclusion.
fn quality_check(
    game: &StoredMatch,
    participants: &[Participant],
    thresholds: QualityThresholds,
) -> Result<(), &'static str> {
    if !QUALITY_QUEUES.contains(&game.queue_id) {
        return Ok(());
    }
    let duration_s = i64::from(game.game_duration_s);
    if duration_s < i64::from(thresholds.min_game_duration_s) {
        return Err("short_game");
    }
    let (mut afk, mut left_early) = (false, false);
    for p in participants {
        if thresholds.exclude_afk {
            if let Some(value) = p.raw.get("wasAfk") {
                afk |= value.as_bool().ok_or("invalid_match")?;
            }
        }
        if thresholds.min_played_percent > 0 {
            if let Some(value) = p.raw.get("timePlayed") {
                let played_s = value.as_u64().ok_or("invalid_match")?;
                // Entiers uniquement : pas d'arrondi flottant à la limite exacte des 80 %.
                left_early |= i128::from(played_s) * 100
                    < i128::from(thresholds.min_played_percent) * i128::from(duration_s);
            }
        }
    }
    if afk {
        return Err("afk");
    }
    if left_early {
        return Err("early_departure");
    }
    Ok(())
}

fn validate(game: &StoredMatch) -> Result<Vec<Participant>, &'static str> {
    if game.is_remake {
        return Err("remake");
    }
    let info = &game.detail["info"];
    if game.detail["metadata"]["matchId"].as_str() != Some(&game.match_id)
        || game.match_id.split_once('_').map(|(p, _)| p) != Some(&game.platform_id)
        || info["platformId"].as_str() != Some(&game.platform_id)
        || info["queueId"].as_i64() != Some(i64::from(game.queue_id))
        || info["gameVersion"]
            .as_str()
            .and_then(patch_from_version)
            .as_deref()
            != Some(&game.patch)
    {
        return Err("invalid_match");
    }
    let raw = info["participants"].as_array().ok_or("invalid_match")?;
    let ranked = [420, 440].contains(&game.queue_id);
    let standard = [
        400, 420, 430, 440, 450, 480, 490, 700, 720, 830, 840, 850, 870, 880, 890, 900, 1020, 1300,
        1400, 1900, 2300, 2400,
    ]
    .contains(&game.queue_id);
    let arena = info["gameMode"].as_str() == Some("CHERRY")
        || [1700, 1710, 1740, 1750].contains(&game.queue_id);
    let swarm = [1810, 1820, 1830, 1840].contains(&game.queue_id);
    if !(1..=64).contains(&raw.len()) || (standard && raw.len() != 10) {
        return Err("invalid_match");
    }
    if swarm && raw.len() != ((game.queue_id - 1800) / 10) as usize {
        return Err("invalid_match");
    }
    // Les métadonnées coop recensent les humains ; les dix fiches incluent les bots.
    let metadata_players = if is_coop(game.queue_id) {
        raw.iter().filter(|p| !is_bot(p)).count()
    } else {
        raw.len()
    };
    if metadata_players == 0
        || game.detail["metadata"]["participants"]
            .as_array()
            .is_some_and(|ids| ids.len() != metadata_players)
    {
        return Err("invalid_match");
    }
    let roles = [400, 420, 430, 440, 480, 490, 700].contains(&game.queue_id);
    let mut champions = BTreeSet::new();
    let mut slots = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut outcomes = BTreeMap::new();
    let mut team_sizes = BTreeMap::<u32, u32>::new();
    let mut participants = vec![];
    for p in raw {
        match p.get("gameEndedInEarlySurrender") {
            Some(Value::Bool(true)) => return Err("remake"),
            Some(Value::Bool(false)) | None => {}
            _ => return Err("invalid_match"),
        }
        let champion = number(p, "championId")
            .filter(|v| *v > 0)
            .ok_or("invalid_match")?;
        let mut team = number(p, "teamId")
            .filter(|v| *v > 0)
            .ok_or("invalid_match")?;
        if arena {
            team = number(p, "playerSubteamId")
                .filter(|v| *v > 0)
                .ok_or("invalid_match")?;
        }
        let win = p["win"].as_bool().ok_or("invalid_match")?;
        if (ranked && !champions.insert(champion)) || (standard && ![100, 200].contains(&team)) {
            return Err("invalid_match");
        }
        *team_sizes.entry(team).or_default() += 1;
        if let Some(id) = p.get("participantId") {
            if id.as_u64().filter(|v| *v > 0).is_none() || !ids.insert(id.as_u64()) {
                return Err("invalid_match");
            }
        }
        let role = if roles {
            match p["teamPosition"].as_str() {
                Some("TOP") => Role::Top,
                Some("JUNGLE") => Role::Jungle,
                Some("MIDDLE") => Role::Middle,
                Some("BOTTOM") => Role::Bottom,
                Some("UTILITY") => Role::Utility,
                _ => Role::Unknown,
            }
        } else {
            Role::Unknown
        };
        if ranked && role != Role::Unknown && !slots.insert((team, role)) {
            return Err("invalid_match");
        }
        if outcomes.insert(team, win).is_some_and(|old| old != win) {
            return Err("invalid_match");
        }
        participants.push(Participant {
            raw: p.clone(),
            champion,
            team,
            role,
            win,
        });
    }
    if standard
        && (team_sizes.get(&100) != Some(&5)
            || team_sizes.get(&200) != Some(&5)
            || outcomes.get(&100) == outcomes.get(&200))
    {
        return Err("invalid_match");
    }
    if arena {
        let size = if [1740, 1750].contains(&game.queue_id) {
            3
        } else {
            2
        };
        if team_sizes.len() < 2 || team_sizes.values().any(|n| *n != size) {
            return Err("invalid_match");
        }
    }
    if !arena && outcomes.len() == 2 && outcomes.values().all(|v| *v) {
        return Err("invalid_match");
    }
    if !swarm && (!outcomes.values().any(|v| *v) || !outcomes.values().any(|v| !*v)) {
        return Err("invalid_match");
    }
    Ok(participants)
}

fn valid_bans(detail: &Value) -> Option<BTreeSet<u32>> {
    let teams = detail["info"]["teams"].as_array()?;
    if teams.len() != 2 {
        return None;
    }
    let mut ids = BTreeSet::new();
    let mut bans = BTreeSet::new();
    let mut turns = BTreeSet::new();
    let mut entries = 0;
    for team in teams {
        let id = number(team, "teamId")?;
        if ![100, 200].contains(&id) || !ids.insert(id) {
            return None;
        }
        let team_bans = team["bans"].as_array()?;
        if team_bans.len() != 5 {
            return None;
        }
        for ban in team_bans {
            let turn = number(ban, "pickTurn")?;
            if !(1..=10).contains(&turn) || !turns.insert(turn) {
                return None;
            }
            let champion = ban["championId"].as_i64()?;
            if champion < -1 {
                return None;
            }
            entries += 1;
            if champion > 0 {
                bans.insert(u32::try_from(champion).ok()?);
            }
        }
    }
    // Un tableau vide signifie aussi un mode sans bans ; il n'est pas une draft observée.
    (entries > 0).then_some(bans)
}
