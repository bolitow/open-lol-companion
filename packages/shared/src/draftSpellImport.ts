import type {ImportSpellsRequest} from './imports';
import type {DraftRuneImportError} from './draftRuneImport';
/** Miroir de ImportDraftSpellsRequest ; garde identique aux runes. */
export interface ImportDraftSpellsRequest {championId:number;spells:ImportSpellsRequest}
export type DraftSpellImportError=DraftRuneImportError;
