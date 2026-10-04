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

/**
 * Paliers cumulés (#83) : « X et plus », regroupement de paliers observés calculé à
 * l'agrégation. Miroir de `CUMULATIVE_RANKS` (collecteur) ; pas de `GRANDMASTER_PLUS` ni de
 * `CHALLENGER_PLUS`, `MASTER_PLUS` couvrant les trois paliers apex.
 */
export const CUMULATIVE_RANKS = [
  "IRON_PLUS",
  "BRONZE_PLUS",
  "SILVER_PLUS",
  "GOLD_PLUS",
  "PLATINUM_PLUS",
  "EMERALD_PLUS",
  "DIAMOND_PLUS",
  "MASTER_PLUS",
] as const;

export type CumulativeRank = (typeof CUMULATIVE_RANKS)[number];

/**
 * `ALL` représente une population distincte des rangs observés. `rank` vaut `ALL`, un palier
 * observé (`IRON` … `CHALLENGER`), `UNKNOWN`, `UNRANKED`, `UNRANKED_MODE` ou un palier cumulé
 * (`CumulativeRank`, #83) : une population déjà calculée, jamais à additionner à `ALL`, à un
 * palier observé ni à un autre palier cumulé.
 */
export interface GroupKey extends ScopeKey {
  role: Role;
  rank: string;
  champion_id: number;
}

/**
 * Fiabilité d'un taux au regard de son effectif (#91) : `low` sous `meta.reliability_floor`
 * observations, quel que soit `meta.min_games`. Jamais un MMR ni une valeur cachée.
 */
export type Reliability = "low" | "sufficient";

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
  /** Borne supérieure de Wilson à 95 % du winrate ; nulle comme la borne basse (#91). */
  win_rate_upper_bound: number | null;
  /** Bornes de Wilson à 95 % du pick rate, nulles quand le pick rate l'est (#91). */
  pick_rate_lower_bound: number | null;
  pick_rate_upper_bound: number | null;
  /** `low` sous `meta.reliability_floor` parties du champion ; null avant #91. */
  reliability: Reliability | null;
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
   * `ALL` (toutes les drafts du périmètre), palier de partie (`IRON` … `CHALLENGER`) ou
   * palier cumulé (`CumulativeRank`, #83, drafts dont le palier de partie est au moins égal) ;
   * `UNKNOWN` sans palier calculable, `UNRANKED_MODE` hors Solo/Flex. `ALL` avant #109.
   * Ne jamais additionner `ALL`, les paliers et les paliers cumulés : une draft compte sous
   * plusieurs d'entre eux.
   */
  rank: string;
  champion_id: number;
  banned_matches: number;
  draft_matches: number;
  ban_rate: number | null;
  /** Bornes de Wilson à 95 % du ban rate, nulles quand le taux l'est (#91). */
  ban_rate_lower_bound: number | null;
  ban_rate_upper_bound: number | null;
  /** `low` sous `meta.reliability_floor` drafts du palier ; null avant #91. */
  reliability: Reliability | null;
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
  /** Borne supérieure de Wilson à 95 % ; nulle sous le seuil ou sans performance publiable (#91). */
  win_rate_upper_bound: number | null;
  /** `low` sous `meta.reliability_floor` parties de la variante ; null avant #91. */
  reliability: Reliability | null;
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

/** Métadonnées et contenu sont lus depuis le même instantané publié. */
export interface SnapshotMeta {
  source_snapshot_at: string;
  published_at: string;
  schema_version: number;
  min_games: number;
  /** Plancher de fiabilité, indépendant de `min_games` ; 0 pour un instantané antérieur à #91. */
  reliability_floor: number;
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
  coverage: ScopeCoverage[];
}

/** Filtres normalisés, tous présents dans la réponse, pagination comprise. */
export interface StatsQuery {
  patch: string;
  platform: string;
  queue: number;
  role: string;
  /** Mêmes valeurs que `GroupKey.rank`, paliers cumulés compris (#83). */
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
