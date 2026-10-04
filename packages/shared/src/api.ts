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
  win_rate: number | null;
  pick_rate: number | null;
  win_rate_lower_bound: number | null;
  position: number | null;
  tier: string | null;
  most_picked_rank: string | null;
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
  rank_scope: string;
  rank_max_age_hours: number;
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
  /** Victoires et défaites de la saison (league-v4) ; absents d'un profil lu dans le client LoL. */
  wins?: number | null;
  losses?: number | null;
  /** Drapeaux factuels league-v4 : série de victoires, vétéran, nouveau dans le palier, inactif. */
  hot_streak?: boolean | null;
  veteran?: boolean | null;
  fresh_blood?: boolean | null;
  inactive?: boolean | null;
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

/**
 * Corps de `POST /v1/privacy/export` et `POST /v1/privacy/erase` (#99), réservés aux
 * sujets de jeton listés dans `OLC_API_PRIVACY_OPERATORS`.
 */
export interface PrivacyRequest {
  puuid: string;
}

/** Classement d'un joueur de départ relevé par une exécution du collecteur. */
export interface SeedEntry {
  run_id: number;
  platform_id: string;
  tier: string;
  division: string;
  league_points: number | null;
  observed_at_ms: number;
}

/** Rang Solo/Flex observé pour un participant. */
export interface RankObservation {
  platform_id: string;
  queue_id: number;
  status: "ranked" | "unranked";
  tier: string | null;
  division: string | null;
  league_points: number | null;
  observed_at_ms: number;
}

/** Partie trouvée ou retenue via l'historique du joueur. */
export interface RunMatchLink {
  run_id: number;
  match_id: string;
  recorded_at_ms: number;
}

/** Partie stockée où le joueur apparaît, avec sa seule fiche de participant match-v5. */
export interface MatchParticipation {
  match_id: string;
  platform_id: string;
  queue_id: number;
  patch: string;
  game_start_ms: number;
  participant: JsonValue | null;
}

/** Données détenues pour un PUUID, sans celles des autres joueurs. */
export interface SubjectExport {
  puuid: string;
  seed_entries: SeedEntry[];
  rank_observations: RankObservation[];
  discoveries: RunMatchLink[];
  sampled_matches: RunMatchLink[];
  matches: MatchParticipation[];
  timeline_match_ids: string[];
  /** Travaux de collecte, quel que soit leur état, qui portent encore ce PUUID. */
  collection_jobs: number;
}

/** Bilan d'un effacement ; `jobs_in_flight > 0` impose de relancer après la collecte. */
export interface SubjectErasure {
  seed_entries: number;
  rank_observations: number;
  discoveries: number;
  sampled_matches: number;
  jobs: number;
  jobs_in_flight: number;
  matches: number;
  timelines: number;
}

export type ApiErrorCode =
  | "invalid_request"
  | "unauthorized"
  | "forbidden"
  | "not_found"
  | "unavailable"
  | "rate_limited"
  | "riot_busy";

export interface ApiErrorResponse {
  error: {
    code: ApiErrorCode;
  };
}
