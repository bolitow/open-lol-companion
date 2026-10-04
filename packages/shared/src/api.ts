/** Contrats JSON de l'API interne (#19), consommés par l'app et le site. */
export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

/** Rôle observé dans les données Riot, miroir de `aggregation::Role`. */
export type Role =
  | "TOP"
  | "JUNGLE"
  | "MIDDLE"
  | "BOTTOM"
  | "UTILITY"
  | "UNKNOWN";

/** Périmètre de calcul du rapport ; les listes vides acceptent toutes les valeurs stockées. */
export interface AggregationOptions {
  patches: string[];
  platforms: string[];
  queues: number[];
  start_ms: number | null;
  end_ms: number | null;
}

export interface ScopeKey {
  patch: string;
  platform_id: string;
  queue_id: number;
}

/** `ALL` représente une population distincte des rangs observés. */
export interface GroupKey extends ScopeKey {
  role: Role;
  rank: string;
  champion_id: number;
}

/** Les taux et le classement restent nuls lorsque l'échantillon est insuffisant. */
export interface ChampionStats extends GroupKey {
  games: number;
  wins: number;
  losses: number;
  population: number;
  /** Nul en Arena : le booléen de victoire n'y désigne pas une première place. */
  win_rate: number | null;
  pick_rate: number | null;
  /** Nul en Arena, comme `win_rate`. */
  win_rate_lower_bound: number | null;
  position: number | null;
  tier: string | null;
  most_picked_rank: string | null;
  /** Arena : participations au placement de sous-équipe valide ; 0 hors Arena. */
  placement_games: number;
  /** Arena : placement moyen de la sous-équipe (1 = première) ; nul sous le seuil et hors Arena. */
  average_placement: number | null;
  /** Arena : part (%) des participations classées première ; nulle sous le seuil et hors Arena. */
  top1_rate: number | null;
  /** Arena : part (%) des participations classées première ou deuxième ; nulle sous le seuil et hors Arena. */
  top2_rate: number | null;
}

/** Les bans concernent la draft entière, sans rang ni rôle individuel. */
export interface BanStats extends ScopeKey {
  champion_id: number;
  banned_matches: number;
  draft_matches: number;
  ban_rate: number | null;
}

/**
 * Étapes d'achat (#81), chacune agrégée séparément avec sa propre population :
 * départ (achats nets avant 1 min 30, multiensemble trié), bottes (première paire,
 * vide si aucune), core (3 premiers objets complets, dans l'ordre d'achat) et
 * objets complets suivants (4e, 5e, 6e). Absentes sans catalogue d'objets du patch.
 */
export type BuildStageCategory =
  | "starter"
  | "boots"
  | "core"
  | "item_slot_4"
  | "item_slot_5"
  | "item_slot_6";

export interface BuildStats extends GroupKey {
  /** Empreintes exactes (`final_items`, `purchase_order`…) ou `BuildStageCategory`. */
  category: string;
  selection: number[];
  games: number;
  wins: number | null;
  performance_available: boolean;
  population: number;
  pick_rate: number | null;
  win_rate: number | null;
  /** Borne inférieure de Wilson à 95 % ; nulle sous le seuil ou sans performance publiable. */
  win_rate_lower_bound: number | null;
}

export interface SkillStats extends GroupKey {
  /** Rang du point investi, distinct du niveau du champion. */
  point: number;
  slot: number;
  games: number;
  mean_timestamp_ms: number;
}

export interface ItemEventStats extends GroupKey {
  event: string;
  item_id: number;
  minute: number;
  events: number;
}

/** Axe d'une ligne `SplitStats`, miroir de `aggregation::SplitDimension`. */
export type SplitDimension = "duration" | "side";

/**
 * Tranche de durée de partie (début inclus, fin exclue : `20_25` couvre de 20 min 00 s
 * à 24 min 59 s) ou côté. Miroir de `aggregation::SplitBucket`.
 */
export type SplitBucket =
  | "lt_20"
  | "20_25"
  | "25_30"
  | "30_35"
  | "35_40"
  | "gte_40"
  | "blue"
  | "red";

/**
 * Winrate d'un champion selon la durée de la partie ou le côté (#119). Absent pour Arena
 * et les modes sans équipes 100/200 ; le côté n'est publié que pour le rang `ALL`.
 */
export interface SplitStats extends GroupKey {
  dimension: SplitDimension;
  bucket: SplitBucket;
  games: number;
  wins: number;
  /** Nul sous le seuil minimal de l'instantané. */
  win_rate: number | null;
  /** Borne inférieure de Wilson à 95 % ; nulle sous le seuil. */
  win_rate_lower_bound: number | null;
}

/**
 * Issue des parties où une équipe a pris un premier objectif. Une partie n'est comptée
 * que si exactement une équipe est marquée première ; la victoire rouge se déduit par
 * différence (`wins - blue_wins` sur `matches - blue_matches`).
 */
export interface FirstObjectiveStats {
  /** Parties où l'objectif a été pris en premier par une équipe identifiée. */
  matches: number;
  /** Victoires de l'équipe qui l'a pris en premier. */
  wins: number;
  /** Parties où l'équipe bleue l'a pris en premier. */
  blue_matches: number;
  /** Victoires de l'équipe bleue parmi ces parties. */
  blue_wins: number;
  /** Winrate (%) de l'équipe ayant pris l'objectif en premier ; nul sous le seuil. */
  win_rate: number | null;
  /** Winrate (%) de l'équipe bleue quand elle le prend en premier ; nul sous le seuil. */
  blue_win_rate: number | null;
}

export interface Coverage {
  matches: number;
  participations: number;
  excluded_bot_participations: number;
  ranked_participations: number;
  unranked_participations: number;
  unknown_rank_participations: number;
  unranked_mode_participations: number;
  unknown_role_participations: number;
  timeline_matches: number;
  timeline_participations: number;
  invalid_timeline_participations: number;
  unidentified_item_undos: number;
  draft_matches: number;
  /** Part (%) des participations Solo/Flex sans rang attribuable ; null hors files classées. */
  unknown_rank_rate: number | null;
  /** Écart médian (heures) entre début de partie et observation de rang retenue. */
  rank_gap_median_hours: number | null;
  /** Écart maximal retenu (heures), au plus `rank_max_age_hours`. */
  rank_gap_max_hours: number | null;
  /** Début (ms Unix) de la plus ancienne partie incluse du périmètre ; null pour un instantané antérieur. */
  first_game_start_ms: number | null;
  /** Début (ms Unix) de la plus récente partie incluse : la vraie fraîcheur, distincte du calcul. */
  last_game_start_ms: number | null;
  /** Participations dont les étapes d'achat ont été dérivées du catalogue du patch. */
  item_stage_participations: number;
  /** Participations à achats nets connus mais sans catalogue d'objets pour leur patch. */
  missing_item_catalog_participations: number;
  /** Participations Arena sans placement de sous-équipe valide, comptées dans `games`. */
  unknown_placement_participations: number;
  /** Parties à deux camps (hors Arena) dont le côté est compté (#119). */
  blue_side_matches: number;
  /** Victoires de l'équipe bleue parmi ces parties. */
  blue_side_wins: number;
  /** Winrate (%) du côté bleu ; nul sous le seuil. */
  blue_side_win_rate: number | null;
  /** Issue selon l'équipe ayant pris le premier sang. */
  first_blood: FirstObjectiveStats;
  /** Issue selon l'équipe ayant pris le premier dragon. */
  first_dragon: FirstObjectiveStats;
  /** Issue selon l'équipe ayant pris la première tour. */
  first_tower: FirstObjectiveStats;
}

export interface ScopeCoverage extends ScopeKey, Coverage {}

/** Fraîcheur réelle des périmètres lus (#103) ; dates de parties nulles pour un instantané antérieur. */
export interface Freshness {
  /** Date du calcul, identique à `source_snapshot_at`. */
  computed_at: string;
  /** Début (ms Unix) de la plus ancienne partie incluse des périmètres lus. */
  first_game_start_ms: number | null;
  /** Début (ms Unix) de la plus récente partie incluse des périmètres lus. */
  last_game_start_ms: number | null;
}

/** Métadonnées et contenu sont lus depuis le même instantané publié. */
export interface SnapshotMeta {
  source_snapshot_at: string;
  published_at: string;
  schema_version: number;
  min_games: number;
  rank_scope: string;
  rank_max_age_hours: number;
  pick_rate_definition: string;
  tier_method: string;
  filters: AggregationOptions;
  freshness: Freshness;
  coverage: ScopeCoverage[];
}

/** Filtres normalisés, tous présents dans la réponse, pagination comprise. */
export interface StatsQuery {
  patch: string;
  platform: string;
  queue: number;
  role: string;
  rank: string;
  offset: number;
  limit: number;
}

export interface TierlistResponse {
  meta: SnapshotMeta;
  query: StatsQuery;
  total: number;
  entries: ChampionStats[];
  bans: BanStats[];
}

export interface BuildsResponse {
  meta: SnapshotMeta;
  query: StatsQuery;
  champion_id: number;
  summary: ChampionStats | null;
  total: number;
  builds: BuildStats[];
  skill_levels: SkillStats[];
  item_events: ItemEventStats[];
  /** Tranches de durée puis côtés du champion (#119) ; vide pour un instantané antérieur. */
  splits: SplitStats[];
  max_build_variants_per_category: number;
  omitted_build_variants: number;
  /** Règles des étapes d'achat ; vide pour un instantané antérieur à #81. */
  build_stage_method: string;
  /** Version du catalogue d'objets jointe au patch demandé ; null sans étapes. */
  item_catalog_version: string | null;
}

/** Population d'une série entre patchs : le patch n'en fait pas partie, il est l'axe. */
export interface TrendsQuery {
  platform: string;
  queue: number;
  role: string;
  rank: string;
}

/**
 * Un patch publié de la série d'un champion. Pourcentages nuls sous le seuil de
 * l'instantané ; un patch observé sans ligne du champion est un point à 0 partie.
 */
export interface TrendPoint {
  patch: string;
  games: number;
  wins: number;
  /** Participations de tous les champions du même patch, rôle et rang. */
  population: number;
  /** Nul en Arena, comme dans la tierlist. */
  win_rate: number | null;
  pick_rate: number | null;
  banned_matches: number;
  draft_matches: number;
  ban_rate: number | null;
  /** Écart en points de pourcentage avec le patch publié précédent ; nul si une valeur est inconnue. */
  delta_win_rate: number | null;
  delta_pick_rate: number | null;
  delta_ban_rate: number | null;
}

/** Du plus ancien au plus récent patch de l'instantané ; couverture de tous ces patchs. */
export interface TrendsResponse {
  meta: SnapshotMeta;
  query: TrendsQuery;
  champion_id: number;
  points: TrendPoint[];
}

export interface ProfileRank {
  queue_id: number;
  status: "ranked" | "unranked";
  tier: string | null;
  division: string | null;
  league_points: number | null;
}

export interface Profile {
  platform: string;
  game_name: string;
  tag_line: string;
  puuid: string;
  profile_icon_id: number | null;
  summoner_level: number | null;
  ranks: ProfileRank[];
  /** Horodatage Unix en secondes de l'acquisition du profil. */
  fetched_at: number;
}

export interface PlayerMatch {
  match_id: string;
  queue_id: number;
  patch: string;
  game_start_ms: number;
  duration_s: number;
  champion_id: number;
  win: boolean;
  kills: number | null;
  deaths: number | null;
  assists: number | null;
  items: number[];
  role: string | null;
}

export interface ProfileMatches {
  platform: string;
  game_name: string;
  tag_line: string;
  /** Horodatage Unix en secondes de l'acquisition de cette page. */
  fetched_at: number;
  start: number;
  count: number;
  next_start: number | null;
  omitted_matches: number;
  matches: PlayerMatch[];
}

export interface StaticManifest {
  live_version: string;
  checked_at: string;
  versions: string[];
  catalogs: JsonValue;
}

/** Premier message WebSocket serveur, puis annonce des publications suivantes. */
export interface Publication {
  type: "data.updated";
  stats_version: string | null;
  static_version: string | null;
  available: boolean;
}

/** Message initial du client WebSocket ; le serveur répond directement par `Publication`. */
export interface WsAuthenticate {
  type: "authenticate";
  token: string;
}

export type ApiErrorCode =
  | "invalid_request"
  | "unauthorized"
  | "not_found"
  | "unavailable"
  | "rate_limited";

export interface ApiErrorResponse {
  error: {
    code: ApiErrorCode;
  };
}
