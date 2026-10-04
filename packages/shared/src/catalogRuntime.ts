/** État du catalogue local publié par le cœur natif. */
export interface CatalogRuntimeState {
 status:'embedded'|'ready'|'updating'|'error';
 version:string|null;
 snapshotId:string|null;
 assetBase:string|null;
 error:string|null;
}
