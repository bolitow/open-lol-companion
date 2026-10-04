use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::builds::{self, BuildObservation};
use super::context::{self, FirstObjectiveStats, SplitBucket, SplitStats};
use super::stages::{ItemCatalog, STAGE_CATEGORIES, STAGE_METHOD};
use super::AggregationError;
use crate::model::patch_from_version;

/// Écart maximal par défaut entre le début de la partie et l'observation de rang retenue.
pub const DEFAULT_RANK_MAX_AGE_HOURS: u32 = 168;
/// Borne haute de l'écart configurable : au-delà, le rang ne décrit plus la partie.
pub const MAX_RANK_MAX_AGE_HOURS: u32 = 8760;

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
    /// Nul en Arena : le booléen `win` n'y désigne pas une première place (#104).
    pub win_rate: Option<f64>,
    pub pick_rate: Option<f64>,
    /// Nul en Arena, comme `win_rate`.
    pub win_rate_lower_bound: Option<f64>,
    pub position: Option<u32>,
    pub tier: Option<String>,
    /// Rang connu comptant le plus de sélections ; ce n'est pas un taux de popularité corrigé.
    pub most_picked_rank: Option<String>,
    /// Arena : participations dont le placement de sous-équipe est valide ; 0 hors Arena.
    #[serde(default)]
    pub placement_games: u64,
    /// Arena : placement moyen de la sous-équipe (1 = première), nul sous le seuil.
    #[serde(default)]
    pub average_placement: Option<f64>,
    /// Arena : part (%) des participations classées première, nulle sous le seuil.
    #[serde(default)]
    pub top1_rate: Option<f64>,
    /// Arena : part (%) des participations classées première ou deuxième, nulle sous le seuil.
    #[serde(default)]
    pub top2_rate: Option<f64>,
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
    /// Début (ms Unix) de la plus ancienne partie incluse du périmètre (#103) ; nul si
    /// l'instantané est antérieur ou si aucune partie n'est incluse.
    #[serde(default)]
    pub first_game_start_ms: Option<i64>,
    /// Début (ms Unix) de la plus récente partie incluse : la vraie fraîcheur des données,
    /// distincte de l'heure du calcul (#103).
    #[serde(default)]
    pub last_game_start_ms: Option<i64>,
    /// Participations dont les étapes d'achat (#81) ont été dérivées du catalogue du patch.
    #[serde(default)]
    pub item_stage_participations: u64,
    /// Participations à achats nets connus, sans catalogue d'objets publié pour leur patch.
    #[serde(default)]
    pub missing_item_catalog_participations: u64,
    /// Participations Arena sans placement de sous-équipe valide (#104) ; comptées dans
    /// `games` mais absentes des métriques de placement.
    #[serde(default)]
    pub unknown_placement_participations: u64,
    /// Parties à deux camps (équipes 100 et 200, hors Arena) dont le côté a été compté (#119).
    #[serde(default)]
    pub blue_side_matches: u64,
    /// Parmi elles, victoires de l'équipe bleue.
    #[serde(default)]
    pub blue_side_wins: u64,
    /// Winrate du côté bleu (%), nul sous le seuil.
    #[serde(default)]
    pub blue_side_win_rate: Option<f64>,
    /// Issue des parties selon l'équipe ayant pris le premier sang (#119).
    #[serde(default)]
    pub first_blood: FirstObjectiveStats,
    /// Issue des parties selon l'équipe ayant pris le premier dragon.
    #[serde(default)]
    pub first_dragon: FirstObjectiveStats,
    /// Issue des parties selon l'équipe ayant pris la première tour.
    #[serde(default)]
    pub first_tower: FirstObjectiveStats,
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
    /// Winrate par tranche de durée et par côté (#119) ; vide dans les rapports antérieurs.
    #[serde(default)]
    pub splits: Vec<SplitStats>,
    pub max_build_variants_per_category: u32,
    pub omitted_build_variants: u64,
    /// Règles des catégories d'étapes (#81) ; vide dans les rapports antérieurs.
    #[serde(default)]
    pub build_stage_method: String,
    #[serde(default)]
    pub item_catalogs: Vec<ItemCatalogRef>,
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
    pub detail: Value,
    pub timeline: Option<Value>,
    pub ranks: BTreeMap<String, ObservedRank>,
    /// Début de la partie (ms Unix), colonne `game_start` : même source que les filtres de fenêtre.
    pub game_start_ms: i64,
    /// Durée en secondes, colonne `game_duration_s` déjà normalisée à l'ingestion.
    pub game_duration_s: i32,
}

type Population = (ScopeKey, Role, String);
type BuildKey = (GroupKey, String, Vec<u32>);
#[derive(Default)]
struct Count {
    games: u64,
    wins: u64,
    placement_games: u64,
    placement_sum: u64,
    top1: u64,
    top2: u64,
}

pub(super) struct Accumulator {
    report: AggregationReport,
    counts: BTreeMap<GroupKey, Count>,
    arena_scopes: BTreeSet<ScopeKey>,
    populations: BTreeMap<Population, u64>,
    coverage: BTreeMap<ScopeKey, Coverage>,
    bans: BTreeMap<(ScopeKey, u32), u64>,
    builds: BTreeMap<BuildKey, Count>,
    /// Parties où la paire de sorts a été observée en ordre décroissant (case D > case F),
    /// par paire triée ; l'autre orientation se déduit de `games` (#124).
    inverted_spells: BTreeMap<(GroupKey, Vec<u32>), u64>,
    build_populations: BTreeMap<(GroupKey, String), u64>,
    skills: BTreeMap<(GroupKey, u32, u32), (u64, u128)>,
    events: BTreeMap<(GroupKey, String, u32, u32), u64>,
    /// Parties et victoires par groupe et par tranche de durée ou côté (#119).
    splits: BTreeMap<(GroupKey, SplitBucket), Count>,
    /// Écarts partie → observation des rangs retenus, en secondes, par périmètre.
    rank_gaps: BTreeMap<ScopeKey, Vec<u64>>,
    /// Catalogue d'objets par patch (« 16.19 »), joint pour dériver les étapes (#81).
    item_catalogs: BTreeMap<String, ItemCatalog>,
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
                pick_rate_definition: "champion_participations / bucket_participations * 100".into(),
                tier_method: "Wilson95 lower bound (Arena: ascending average placement); S/A/B/C/D percentiles 10/30/60/90/100; at least 5 eligible champions".into(),
                min_games, filters: AggregationOptions::default(), source_matches: 0,
                included_matches: 0, exclusions: BTreeMap::new(), coverage: vec![], groups: vec![],
                bans: vec![], builds: vec![], skill_levels: vec![], item_events: vec![], splits: vec![],
                max_build_variants_per_category: 20, omitted_build_variants: 0,
                build_stage_method: STAGE_METHOD.into(), item_catalogs: vec![],
            },
            counts:BTreeMap::new(), arena_scopes:BTreeSet::new(), populations:BTreeMap::new(), coverage:BTreeMap::new(),
            bans:BTreeMap::new(), builds:BTreeMap::new(), inverted_spells:BTreeMap::new(), build_populations:BTreeMap::new(),
            skills:BTreeMap::new(), events:BTreeMap::new(), splits:BTreeMap::new(), rank_gaps:BTreeMap::new(),
            item_catalogs:BTreeMap::new(),
        })
    }

    pub fn set_rank_max_age_hours(&mut self, hours: u32) -> Result<(), AggregationError> {
        if !(1..=MAX_RANK_MAX_AGE_HOURS).contains(&hours) {
            return Err(AggregationError::InvalidRankMaxAge);
        }
        self.report.rank_max_age_hours = hours;
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
        let participants = match validate(game) {
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
        let arena_game = is_arena(game);
        if arena_game {
            self.arena_scopes.insert(scope.clone());
        }
        let coverage = self.coverage.entry(scope.clone()).or_default();
        coverage.matches += 1;
        coverage.first_game_start_ms = Some(
            coverage
                .first_game_start_ms
                .map_or(game.game_start_ms, |v| v.min(game.game_start_ms)),
        );
        coverage.last_game_start_ms = Some(
            coverage
                .last_game_start_ms
                .map_or(game.game_start_ms, |v| v.max(game.game_start_ms)),
        );
        // Victoire du côté bleu : seulement pour deux camps réellement opposés (#119). La coop
        // contre l'IA est exclue : les humains y occupent toujours le même camp.
        let blue_won = (!arena_game && !is_coop(game.queue_id))
            .then(|| blue_side_won(&participants))
            .flatten();
        if let Some(blue_won) = blue_won {
            coverage.blue_side_matches += 1;
            coverage.blue_side_wins += u64::from(blue_won);
            for (stats, objective) in [
                (&mut coverage.first_blood, "champion"),
                (&mut coverage.first_dragon, "dragon"),
                (&mut coverage.first_tower, "tower"),
            ] {
                if let Some(team) = context::first_team(&game.detail, objective) {
                    let blue_first = team == context::BLUE_TEAM;
                    stats.record(blue_first, blue_first == blue_won);
                }
            }
        }
        let duration_bucket = SplitBucket::from_duration_s(game.game_duration_s);
        if let Some(bans) = valid_bans(&game.detail) {
            coverage.draft_matches += 1;
            for champion in bans {
                *self.bans.entry((scope.clone(), champion)).or_default() += 1;
            }
        }
        let mut timeline_counted = false;
        for p in participants {
            if is_coop(game.queue_id) && is_bot(&p.raw) {
                self.coverage
                    .entry(scope.clone())
                    .or_default()
                    .excluded_bot_participations += 1;
                continue;
            }
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
            if arena_game && p.placement.is_none() {
                coverage.unknown_placement_participations += 1;
            }
            let mut observations = builds::extract_detail(&p.raw);
            if let Some(timeline) = &game.timeline {
                let result = p.raw["participantId"]
                    .as_u64()
                    .and_then(|id| u32::try_from(id).ok())
                    .ok_or("missing_participant_id")
                    .and_then(|id| builds::extract_timeline(timeline, &game.match_id, id));
                match result {
                    Ok(t) => {
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
                if let Some(placement) = p.placement {
                    c.placement_games += 1;
                    c.placement_sum += u64::from(placement);
                    c.top1 += u64::from(placement == 1);
                    c.top2 += u64::from(placement <= 2);
                }
                if blue_won.is_some() {
                    let side = (key.rank == "ALL")
                        .then(|| SplitBucket::from_team(p.team))
                        .flatten();
                    for bucket in duration_bucket.into_iter().chain(side) {
                        let c = self.splits.entry((key.clone(), bucket)).or_default();
                        c.games += 1;
                        c.wins += u64::from(p.win);
                    }
                }
                *self
                    .populations
                    .entry((scope.clone(), p.role, rank))
                    .or_default() += 1;
                self.add_builds(&key, p.win, &observations);
            }
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
            if category == "summoner_spells"
                && matches!(observations.spell_slots, Some([d, f]) if d > f)
            {
                *self
                    .inverted_spells
                    .entry((key.clone(), selection.clone()))
                    .or_default() += 1;
            }
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
                // Arena : le booléen `win` n'est pas une première place (#104). Aucun taux
                // de victoire ni borne de Wilson ; le classement repose sur le placement.
                let arena = self.arena_scopes.contains(&scope_of(&key));
                let placement_rate = |n: u64| rate(n, c.placement_games, minimum);
                ChampionStats {
                    key,
                    games: c.games,
                    wins: c.wins,
                    losses: c.games - c.wins,
                    population,
                    win_rate: if arena {
                        None
                    } else {
                        rate(c.wins, c.games, minimum)
                    },
                    pick_rate: rate(c.games, population, minimum).filter(|_| c.games >= minimum),
                    win_rate_lower_bound: (!arena && c.games >= minimum)
                        .then(|| wilson(c.wins, c.games)),
                    position: None,
                    tier: None,
                    most_picked_rank,
                    placement_games: c.placement_games,
                    average_placement: (c.placement_games >= minimum && c.placement_games > 0)
                        .then(|| c.placement_sum as f64 / c.placement_games as f64),
                    top1_rate: placement_rate(c.top1),
                    top2_rate: placement_rate(c.top2),
                }
            })
            .collect();
        let bucket = |key: &GroupKey| (scope_of(key), key.role, key.rank.clone());
        self.report.groups.sort_by(|a, b| {
            bucket(&a.key)
                .cmp(&bucket(&b.key))
                // Arena : placement moyen croissant (les non éligibles en dernier).
                .then_with(|| {
                    a.average_placement
                        .unwrap_or(f64::INFINITY)
                        .total_cmp(&b.average_placement.unwrap_or(f64::INFINITY))
                })
                .then_with(|| {
                    b.win_rate_lower_bound
                        .unwrap_or(-1.0)
                        .total_cmp(&a.win_rate_lower_bound.unwrap_or(-1.0))
                })
                .then_with(|| {
                    // Arena : le booléen `win` n'est pas une première place, son ratio brut
                    // ne départage donc jamais deux champions (effectif, puis id).
                    if self.arena_scopes.contains(&scope_of(&a.key)) {
                        Ordering::Equal
                    } else {
                        (u128::from(b.wins) * u128::from(a.games))
                            .cmp(&(u128::from(a.wins) * u128::from(b.games)))
                    }
                })
                .then_with(|| b.games.cmp(&a.games))
                .then_with(|| a.key.champion_id.cmp(&b.key.champion_id))
        });
        let mut eligible = BTreeMap::<Population, u32>::new();
        // Un groupe Arena est classé sur son placement moyen, les autres sur leur taux.
        let rankable = |g: &ChampionStats| g.win_rate.is_some() || g.average_placement.is_some();
        for g in &self.report.groups {
            if rankable(g) {
                *eligible.entry(bucket(&g.key)).or_default() += 1;
            }
        }
        let mut positions = BTreeMap::<Population, u32>::new();
        for g in &mut self.report.groups {
            if rankable(g) {
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
        for mut build in builds {
            // La variante est comptée sur la paire triée, sans changer population ni
            // classement ; seul l'ordre publié suit l'orientation D/F majoritaire.
            // Égalité : ordre numérique, pas de préférence inventée.
            if build.category == "summoner_spells" {
                let inverted = self
                    .inverted_spells
                    .get(&(build.key.clone(), build.selection.clone()))
                    .copied()
                    .unwrap_or(0);
                if inverted * 2 > build.games {
                    build.selection.reverse();
                }
            }
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
        self.report.splits = self
            .splits
            .into_iter()
            .map(|((key, bucket), c)| SplitStats {
                key,
                dimension: bucket.dimension(),
                bucket,
                games: c.games,
                wins: c.wins,
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
                counts.blue_side_win_rate =
                    rate(counts.blue_side_wins, counts.blue_side_matches, minimum);
                for stats in [
                    &mut counts.first_blood,
                    &mut counts.first_dragon,
                    &mut counts.first_tower,
                ] {
                    stats.finish(minimum);
                }
                ScopeCoverage { scope, counts }
            })
            .collect();
        self.report
    }
}

pub(super) fn rate(n: u64, d: u64, min: u64) -> Option<f64> {
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
    role: Role,
    win: bool,
    /// Sous-équipe Arena (ou équipe), pour contrôler la cohérence des placements.
    team: u32,
    /// Placement de la sous-équipe en Arena (1 = première) ; nul si absent ou incohérent.
    placement: Option<u32>,
}
/// Issue de l'équipe bleue ; nulle sans les deux camps ou si les deux ont le même résultat.
fn blue_side_won(participants: &[Participant]) -> Option<bool> {
    let outcome = |team| participants.iter().find(|p| p.team == team).map(|p| p.win);
    let (blue, red) = (outcome(context::BLUE_TEAM)?, outcome(context::RED_TEAM)?);
    (blue != red).then_some(blue)
}
fn is_arena(game: &StoredMatch) -> bool {
    game.detail["info"]["gameMode"].as_str() == Some("CHERRY")
        || [1700, 1710, 1740, 1750].contains(&game.queue_id)
}
/// Placement de sous-équipe : `subteamPlacement`, à défaut `placement` ; 0 signifie absent.
fn placement_of(raw: &Value) -> Option<u32> {
    ["subteamPlacement", "placement"]
        .iter()
        .find_map(|key| number(raw, key).filter(|v| *v > 0))
}
/// Les placements ne servent que si chaque sous-équipe a une valeur unique et que les
/// sous-équipes occupent des places distinctes de 1 au nombre de sous-équipes ; sinon
/// aucune n'est conservée (la partie reste comptée, sans classement inventé).
fn placements_are_consistent(participants: &[Participant]) -> bool {
    let mut by_team = BTreeMap::<u32, u32>::new();
    for p in participants {
        let Some(placement) = p.placement else {
            return false;
        };
        if by_team
            .insert(p.team, placement)
            .is_some_and(|old| old != placement)
        {
            return false;
        }
    }
    let distinct: BTreeSet<_> = by_team.values().collect();
    distinct.len() == by_team.len()
        && by_team
            .values()
            .all(|v| (1..=by_team.len() as u32).contains(v))
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
    let arena = is_arena(game);
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
            role,
            win,
            team,
            placement: arena.then(|| placement_of(p)).flatten(),
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
    if arena && !placements_are_consistent(&participants) {
        for p in &mut participants {
            p.placement = None;
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
