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
  /** Participations de tous les champions du même compartiment (patch, plateforme, file, rôle, rang). */
  population: number;
  /** Parties distinctes du compartiment ; 0 pour un instantané antérieur à #84. */
  bucket_matches: number;
  win_rate: number | null;
  /** Parties où le champion apparaît / `bucket_matches` × 100, comparable au ban rate. */
  pick_rate: number | null;
  /** Part des sélections : participations du champion / `population` × 100 ; nulle avant #84. */
  selection_share: number | null;
  win_rate_lower_bound: number | null;
  position: number | null;
  tier: string | null;
  most_picked_rank: string | null;
}

/**
 * Les bans concernent la draft entière, sans rôle individuel. Leur rang est le palier de la
 * partie (#109) : médiane des paliers observés de ses joueurs, jamais un MMR estimé.
 */
export interface BanStats extends ScopeKey {
  /**
   * `ALL` (toutes les drafts du périmètre) ou palier de partie (`IRON` … `CHALLENGER`) ;
   * `UNKNOWN` sans palier calculable, `UNRANKED_MODE` hors Solo/Flex. `ALL` avant #109.
   * Ne jamais additionner `ALL` et les paliers : une draft compte sous les deux.
   */
  rank: string;
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

export interface Coverage {
  matches: number;
  participations: number;
  excluded_bot_participations: number;
  ranked_participations: number;
  /**
   * Participations classées par palier observé (`IRON` … `CHALLENGER`), figé à la partie (#82) :
   * décrit l'échantillon de `ALL`, non repondéré sur le ladder. Somme égale à
   * `ranked_participations` ; vide hors Solo/Flex ou avant #82.
   */
  tier_participations: Record<string, number>;
  /**
   * Part des participations classées en Master, Grandmaster et Challenger parmi
   * `tier_participations` (#82). Fraction entre 0 et 1, pas un pourcentage (contrairement à
   * `unknown_rank_rate`, en %). Null sans participation classée ou avant l'indicateur de biais
   * quand aucune répartition ne permet de la recalculer : l'API la recalcule sinon.
   */
  apex_share: number | null;
  /** Vrai quand `apex_share` dépasse strictement 0,5 : l'échantillon de `ALL` est dominé par le haut du ladder. */
  high_elo_biased: boolean;
  unranked_participations: number;
  unknown_rank_participations: number;
  unranked_mode_participations: number;
  unknown_role_participations: number;
  timeline_matches: number;
  timeline_participations: number;
  invalid_timeline_participations: number;
  unidentified_item_undos: number;
  draft_matches: number;
  /** Parties Solo/Flex retenues dont le palier de partie est calculable ; 0 avant #109. */
  match_tier_matches: number;
  /** Parties Solo/Flex retenues sans palier de partie (joueurs connus insuffisants) ; 0 avant #109. */
  unknown_match_tier_matches: number;
  /** Part (%) des participations Solo/Flex sans rang attribuable ; null hors files classées. */
  unknown_rank_rate: number | null;
  /** Écart médian (heures) entre début de partie et observation de rang retenue. */
  rank_gap_median_hours: number | null;
  /** Écart maximal retenu (heures), au plus `rank_max_age_hours`. */
  rank_gap_max_hours: number | null;
  /** Participations dont les étapes d'achat ont été dérivées du catalogue du patch. */
  item_stage_participations: number;
  /** Participations à achats nets connus mais sans catalogue d'objets pour leur patch. */
  missing_item_catalog_participations: number;
}

export interface ScopeCoverage extends ScopeKey, Coverage {}

/**
 * Nature de la population servie pour le `rank` demandé (#82), miroir de `PopulationLabel`.
 * `collected_sample` (`ALL`) : échantillon collecté non repondéré, jamais « tous les rangs » ;
 * `observed_tier` : palier du joueur figé à la partie ; `match_tier` : palier de la partie
 * (bans) ; `unknown_rank`, `unranked`, `unranked_mode` : rangs homonymes.
 */
export type PopulationLabel =
  | "collected_sample"
  | "observed_tier"
  | "match_tier"
  | "unknown_rank"
  | "unranked"
  | "unranked_mode";

/** Métadonnées et contenu sont lus depuis le même instantané publié. */
export interface SnapshotMeta {
  source_snapshot_at: string;
  published_at: string;
  schema_version: number;
  min_games: number;
  rank_scope: string;
  rank_max_age_hours: number;
  /** Durée minimale (s) d'une partie classée ; 0 pour un instantané antérieur à #111. */
  min_game_duration_s: number;
  /** Part minimale (%) de la durée jouée par chaque participant ; 0 avant #111. */
  min_played_percent: number;
  /** Parties classées avec un participant `wasAfk` écartées (`afk`) ; `false` avant #111. */
  exclude_afk: boolean;
  /** Origine du rang des bans : `match_median` (médiane des paliers de la partie) ; vide avant #109. */
  ban_rank_basis: string;
  /** Joueurs connus minimaux (sur dix) pour qu'une partie reçoive un palier ; 0 avant #109. */
  ban_rank_min_known_players: number;
  /** Parties sources écartées par motif (`remake`, `short_game`, `afk`, `early_departure`, `invalid_match`). */
  exclusions: Record<string, number>;
  pick_rate_definition: string;
  tier_method: string;
  filters: AggregationOptions;
  /** Nature de la population du `rank` demandé ; sa répartition est `coverage[].tier_participations`. */
  population_label: PopulationLabel;
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

/** Filtres de `/v1/bans` : périmètre et palier de partie, sans rôle ni décalage. */
export interface BansQuery {
  patch: string;
  platform: string;
  queue: number;
  rank: string;
  limit: number;
}

/** Bans du palier demandé, du plus au moins banni ; indépendants de la page de tierlist. */
export interface BansResponse {
  meta: SnapshotMeta;
  query: BansQuery;
  /** Bans publiés pour ce palier, avant `limit`. */
  total: number;
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
  max_build_variants_per_category: number;
  omitted_build_variants: number;
  /** Règles des étapes d'achat ; vide pour un instantané antérieur à #81. */
  build_stage_method: string;
  /** Version du catalogue d'objets jointe au patch demandé ; null sans étapes. */
  item_catalog_version: string | null;
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
