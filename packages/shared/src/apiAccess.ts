/** Origine de la configuration active (miroir Rust `ApiAccessSource`). */
export type ApiAccessSource = 'environment' | 'keychain';

/** Codes stables traduits par l'interface (miroir Rust `CredentialError`). */
export type ApiAccessError =
    | 'invalid_configuration'
    | 'too_long'
    | 'storage_unavailable'
    | 'read_failed'
    | 'write_failed';

/**
 * État relu par `api_access_status`, `save_api_access` et `clear_api_access`
 * (miroir Rust `ApiAccessStatus`). Ne contient jamais le jeton : `source` non nul
 * suffit à dire qu'il est présent.
 */
export interface ApiAccessStatus {
    source: ApiAccessSource | null;
    url: string | null;
    error: ApiAccessError | null;
}

/** Saisie envoyée une seule fois au cœur Rust par `save_api_access` (miroir `ApiAccessInput`). */
export interface ApiAccessInput {
    url: string;
    token: string;
}
