import type {BuildStats, Role, ScopeCoverage, ChampionStats, SkillStats, ItemEventStats, OmittedBuildVariants} from './api';

/** Population explicite demandée au backend par le cœur Rust du desktop. */
export interface BuildRequest {
    champion_id: number;
    patch: string;
    platform: string;
    queue: number;
    role: Role;
    rank: string;
}

/** Projection minimale de la couverture publiée ; ce périmètre couvre tous les rôles. */
export type BuildPopulationCoverage=Pick<ScopeCoverage,'patch'|'platform_id'|'queue_id'|'ranked_participations'|'tier_participations'|'apex_share'|'high_elo_biased'>;

export type BuildSummary=Pick<ChampionStats,'patch'|'platform_id'|'queue_id'|'role'|'rank'|'champion_id'|'games'|'wins'|'losses'|'population'|'win_rate'|'pick_rate'>;

/** Publication complète des variantes ; chaque catégorie garde ses propres effectifs. */
export interface BuildReport {
    request: BuildRequest;
    meta: {
        source_snapshot_at: string;
        published_at: string;
        min_games: number;
        /** Optionnels pour les anciens serveurs ; libellés futurs tolérés. */
        population_label?: string;
        coverage?: BuildPopulationCoverage[];
    };
    builds: BuildStats[];
    /** Absents dans les anciennes versions du transport desktop. */
    summary?: BuildSummary|null;
    skill_levels?: SkillStats[];
    item_events?: ItemEventStats[];
    omitted_build_variants?: number|null;
    omitted_build_variants_by_category?: OmittedBuildVariants[];
    max_build_variants_per_category?:number|null;
    max_item_events?:number|null;
    omitted_item_events?:number|null;
}

/** Codes publics de la commande Rust, sans détails d'authentification. */
export type BuildError =
    | 'not_configured'
    | 'invalid_configuration'
    | 'invalid_request'
    | 'unauthorized'
    | 'unavailable'
    | 'rate_limited'
    | 'invalid_response'
    | 'changed_snapshot'
    | 'desktop_required';
