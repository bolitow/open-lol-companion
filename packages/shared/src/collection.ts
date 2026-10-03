import type {LcuAccount} from './gameflow';

export type SkinRarity = 'kNoRarity' | 'kRare' | 'kEpic' | 'kLegendary' | 'kMythic' | 'kUltimate' | 'kExalted' | 'kTranscendent';

/** Possession telle que connue du client : inconnue et temporaire ne sont pas « manquante ». */
export interface CollectionSkin {
    id: number;
    champion_id: number;
    name: string;
    ownership: 'owned' | 'temporary' | 'missing' | 'unknown';
    tile_url: string | null;
    splash_url: string | null;
    obtainable: boolean | null;
    rarity: SkinRarity | null;
    series_ids: number[];
}

/** Instantané versionné du compte local ; aucun identifiant privé ni secret LCU. */
export interface CollectionState {
    revision: number;
    status: 'disconnected' | 'loading' | 'ready' | 'unavailable';
    account: LcuAccount | null;
    skins: CollectionSkin[];
    wishes: number[];
    storage_error: boolean;
    stale: boolean;
}
