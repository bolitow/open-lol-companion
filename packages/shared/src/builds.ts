import type {BuildStats, Role} from './api';

/** Population explicite demandée au backend par le cœur Rust du desktop. */
export interface BuildRequest {
    champion_id: number;
    patch: string;
    platform: string;
    queue: number;
    role: Role;
    rank: string;
}

/** Publication complète des variantes ; chaque catégorie garde ses propres effectifs. */
export interface BuildReport {
    request: BuildRequest;
    meta: {
        source_snapshot_at: string;
        published_at: string;
        min_games: number;
    };
    builds: BuildStats[];
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
