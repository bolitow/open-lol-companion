import {skinLineEntries,validLocalSkinImage} from '../cosmeticsAssets';
import {collectionCopy} from './collectionCopy';
import {championDirectory} from '../championDirectory';
import type {CollectionState, SkinRarity} from '../../../../../packages/shared/src/collection';
import type {Locale} from '../state';

export type CollectionFilter = 'all' | 'owned' | 'missing' | 'wishes';
export interface CollectionView {accountKey: string | null; query: string; filter: CollectionFilter; championId: number | null; rarity: SkinRarity | null; seriesId: number | null; selectedId: number | null; scrollTop: number; visibleCount: number}
export const initialCollectionView: CollectionView = {accountKey: null, query: '', filter: 'all', championId: null, rarity: null, seriesId: null, selectedId: null, scrollTop: 0, visibleCount: 60};
export function collectionViewForAccount(view: CollectionView, accountKey: string | null): CollectionView {
    return view.accountKey === accountKey ? view : {...initialCollectionView, accountKey};
}
export function collectionVisibleCount(count: number, total: number): number {
    return Math.min(Math.max(0, total), Math.max(60, Number.isFinite(count) ? Math.floor(count) : 60));
}
export function nextCollectionCount(count: number, total: number): number {
    return Math.min(total, collectionVisibleCount(count, total) + 60);
}
export function patchCollectionView(view: CollectionView, patch: Partial<CollectionView>): CollectionView {
    const changed = (patch.query !== undefined && patch.query !== view.query)
        || (patch.filter !== undefined && patch.filter !== view.filter)
        || (patch.championId !== undefined && patch.championId !== view.championId)
        || (patch.rarity !== undefined && patch.rarity !== view.rarity)
        || (patch.seriesId !== undefined && patch.seriesId !== view.seriesId);
    return {...view, ...patch, ...(changed ? {visibleCount: 60, scrollTop: 0} : {})};
}
export function initialCollectionState(native: boolean): CollectionState {
    return {revision: 0, status: native ? 'loading' : 'disconnected', account: null, skins: [], wishes: [], storage_error: false, stale: false};
}
const normalized = (value: string) => value.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLocaleLowerCase().trim();
export function filterCollection(state: CollectionState, view: Pick<CollectionView, 'query' | 'filter' | 'championId'> & Partial<Pick<CollectionView, 'rarity' | 'seriesId'>>, locale: Locale) {
    const words = normalized(view.query).split(/\s+/).filter(Boolean), wishes = new Set(state.wishes);
    const seriesNames=new Map(skinLineEntries().map(line=>[line.id,normalized(`${line.names.fr} ${line.names.en}`)]));
    const champions=new Map(championDirectory.champions.map(champion=>[champion.id,normalized(`${champion.names.fr} ${champion.names.en}`)]));
    return state.skins.filter(skin => (view.championId === null || skin.champion_id === view.championId)
        && words.every(word=>normalized([
            skin.name,champions.get(skin.champion_id)??'',
            skin.rarity?`${collectionCopy.fr.rarities[skin.rarity]} ${collectionCopy.en.rarities[skin.rarity]}`:'',
            ...skin.series_ids.map(id=>seriesNames.get(id)??''),
        ].join(' ')).includes(word))
        && (view.rarity == null || skin.rarity === view.rarity)
        && (view.seriesId == null || skin.series_ids.includes(view.seriesId))
        && (view.filter === 'all' || (view.filter === 'wishes' ? wishes.has(skin.id) : skin.ownership === view.filter)))
        .sort((a, b) => a.name.localeCompare(b.name, locale) || a.id - b.id);
}
export function collectionSummary(state: CollectionState) {
    return {owned: state.skins.filter(s => s.ownership === 'owned').length, total: state.skins.length,
        temporary: state.skins.filter(s => s.ownership === 'temporary').length, unknown: state.skins.filter(s => s.ownership === 'unknown').length};
}
export function canEditWishes(state: CollectionState): boolean {
    return state.status === 'ready' && state.account !== null && !state.stale && !state.storage_error;
}
export function collectionAccountKey(state: CollectionState): string | null {
    const account = state.account;
    return account ? `${account.platform}:${account.game_name}#${account.tag_line}` : null;
}
/** N'affiche pas d'URL arbitraire : les images sont des ressources HTTPS transmises par le cœur Rust. */
export function collectionImageUrl(value: string | null): string | null {
    if (!value) return null;
    if(validLocalSkinImage(value))return value;
    try {const url = new URL(value); return url.protocol === 'https:' && !url.username && !url.password ? url.href : null;} catch {return null;}
}
