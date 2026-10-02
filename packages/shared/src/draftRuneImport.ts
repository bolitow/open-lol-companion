import type {ImportError, ImportRunesRequest} from './imports';
/** Miroir Rust du parcours de draft : contexte réel contrôlé avant écriture. */
export interface ImportDraftRunesRequest {championId:number;runes:ImportRunesRequest}
export type DraftRuneImportError=ImportError|'draftContextChanged'|'unsupportedMode'|'importBusy';
