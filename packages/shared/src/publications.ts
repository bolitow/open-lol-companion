import type {Publication} from './api';

/** Événement Tauri émis par le cœur Rust à chaque changement de l'état des publications. */
export const PUBLICATION_STATE_EVENT = 'publication-state' as const;

/** État de la connexion au canal WebSocket `/v1/ws` (miroir Rust `PublicationStatus`). */
export type PublicationStatus =
    | 'not_configured'
    | 'connecting'
    | 'connected'
    | 'reconnecting'
    | 'unauthorized';

/**
 * État relu par la commande `publication_state` et émis avec `PUBLICATION_STATE_EVENT`
 * (miroir Rust `PublicationState`). `revision` augmente à chaque nouvelle publication
 * observée : l'interface relit alors ses builds ; elle n'a jamais à l'interpréter autrement.
 */
export interface PublicationState {
    revision: number;
    status: PublicationStatus;
    /** Dernière publication reçue ; `null` tant qu'aucune n'est arrivée. */
    publication: Publication | null;
}
