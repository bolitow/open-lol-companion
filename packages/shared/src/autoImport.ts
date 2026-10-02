import type {ImportItemsRequest, ImportRunesRequest} from './imports';

/** Garde Rust d'une partie, d'un champion sélectionné et de son poste. */
export interface AutoImportContext {
    draftId: string;
    gameId?: string;
    championId: number;
    role: string;
    queueId: number;
}
export type AutoImportSelection =
    | {kind: 'runes'; request: ImportRunesRequest}
    | {kind: 'items'; request: ImportItemsRequest};
export interface AutoImportRequest {
    context: AutoImportContext;
    selection: AutoImportSelection;
}
/** Le client a accepté l'écriture ; la relecture peut rester non confirmée. */
export interface AutoImportReceipt {confirmed: boolean}
