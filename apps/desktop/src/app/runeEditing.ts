import type {CatalogRecord, JsonValue, RunePage} from '@olc/shared';
import {runePageFromBuild} from './buildModel';

export interface RuneDraft {
    primaryStyleId: number;
    subStyleId: number;
    /** Quatre primaires, deux secondaires ordonnées par ancienneté, puis trois fragments. */
    selectedPerkIds: number[];
}

function validId(id: number): boolean {
    return Number.isSafeInteger(id) && id > 0 && id <= 0xffffffff;
}

export function runeDraftFromPage(page: RunePage | null): RuneDraft {
    const keepId = (id: number | undefined): number => id !== undefined && validId(id) ? id : 0;
    return {primaryStyleId: keepId(page?.primaryStyleId), subStyleId: keepId(page?.subStyleId),
        selectedPerkIds: Array.from({length: 9}, (_, index) => keepId(page?.selectedPerkIds[index]))};
}

function structuralValue(record: CatalogRecord, field: string): JsonValue | null {
    const entry = record.fields[field];
    return entry && (entry.status === 'verified' || entry.status === 'derived') ? entry.value : null;
}

function findRecord(id: number, kind: string, records: readonly CatalogRecord[]): CatalogRecord | null {
    if (!validId(id)) return null;
    const matches = records.filter(record => record.id === String(id) && record.kind === kind && record.namespace === 'standard');
    return matches.length === 1 ? matches[0]! : null;
}

function findStyle(id: number, records: readonly CatalogRecord[]): CatalogRecord | null {
    const record = findRecord(id, 'rune', records);
    return record && structuralValue(record, 'rune_kind') === 'style' ? record : null;
}

function runeSlot(id: number, style: CatalogRecord, records: readonly CatalogRecord[]): number | null {
    const rune = findRecord(id, 'rune', records);
    if (!rune || rune.locale !== style.locale || structuralValue(rune, 'rune_kind') !== 'rune'
        || structuralValue(rune, 'style_id') !== style.id) return null;
    const slot = structuralValue(rune, 'slot');
    return typeof slot === 'number' && Number.isInteger(slot) && slot >= 0 && slot <= 3 ? slot : null;
}

function shardFits(id: number, slot: number, style: CatalogRecord, records: readonly CatalogRecord[]): boolean {
    const shard = findRecord(id, 'rune_shard', records);
    if (!shard || shard.locale !== style.locale || structuralValue(shard, 'listed_in_perk_styles') !== true) return false;
    const positions = structuralValue(shard, 'rune_page_slots');
    return Array.isArray(positions) && positions.some(position => position !== null && typeof position === 'object'
        && !Array.isArray(position) && position.style_id === style.id && position.slot_type === 'kStatMod' && position.slot_index === 4 + slot);
}

export function changeRuneStyle(draft: RuneDraft, branch: 'primary' | 'secondary', styleId: number, records: readonly CatalogRecord[]): RuneDraft {
    const style = findStyle(styleId, records);
    const previous = findStyle(draft.primaryStyleId, records) ?? findStyle(draft.subStyleId, records);
    if (!style || (previous && previous.locale !== style.locale)) return draft;
    if (branch === 'secondary') {
        if (styleId === draft.primaryStyleId || styleId === draft.subStyleId) return draft;
        return {...draft, subStyleId: styleId, selectedPerkIds: [...draft.selectedPerkIds.slice(0, 4), 0, 0, ...draft.selectedPerkIds.slice(6)]};
    }
    if (styleId === draft.primaryStyleId) return draft;
    const secondary = findStyle(draft.subStyleId, records);
    const keepSecondary = secondary !== null && secondary.locale === style.locale && styleId !== draft.subStyleId;
    return {primaryStyleId: styleId, subStyleId: keepSecondary ? draft.subStyleId : 0, selectedPerkIds: [0, 0, 0, 0,
        ...(keepSecondary ? draft.selectedPerkIds.slice(4, 6) : [0, 0]),
        ...Array.from({length: 3}, (_, slot) => {
            const id = draft.selectedPerkIds[6 + slot] ?? 0;
            return shardFits(id, slot, style, records) ? id : 0;
        })]};
}

export function chooseRune(draft: RuneDraft, branch: 'primary' | 'secondary' | 'shard', slot: number, id: number, records: readonly CatalogRecord[]): RuneDraft {
    if (!Number.isInteger(slot)) return draft;
    const style = findStyle(branch === 'secondary' ? draft.subStyleId : draft.primaryStyleId, records);
    if (!style) return draft;
    const choices = [...draft.selectedPerkIds];
    if (branch === 'shard') {
        if (slot < 0 || slot > 2 || !shardFits(id, slot, style, records)) return draft;
        choices[6 + slot] = id;
    } else if (branch === 'primary') {
        if (slot < 0 || slot > 3 || runeSlot(id, style, records) !== slot) return draft;
        choices[slot] = id;
    } else {
        if (draft.primaryStyleId === draft.subStyleId || slot < 1 || slot > 3 || runeSlot(id, style, records) !== slot) return draft;
        if (choices.slice(4, 6).includes(id)) return draft;
        // Le choix remplacé devient le plus récent ; la troisième ligne évince la plus ancienne.
        const retained = choices.slice(4, 6).filter(current => {
            const currentSlot = runeSlot(current, style, records);
            return currentSlot !== null && currentSlot > 0 && currentSlot !== slot;
        });
        const selected = [...retained.slice(-1), id];
        choices.splice(4, 2, selected[0]!, selected[1] ?? 0);
    }
    return {...draft, selectedPerkIds: choices};
}

export function validRuneDraft(draft: RuneDraft, records: readonly CatalogRecord[]): boolean {
    if (draft.selectedPerkIds.length !== 9) return false;
    return runePageFromBuild([draft.primaryStyleId, ...draft.selectedPerkIds.slice(0, 4), draft.subStyleId, ...draft.selectedPerkIds.slice(4)], records) !== null;
}

export function runeDraftKey(draft: RuneDraft): string {
    return JSON.stringify([draft.primaryStyleId, draft.subStyleId, draft.selectedPerkIds]);
}

function completeShape(draft: RuneDraft): boolean {
    return validId(draft.primaryStyleId) && validId(draft.subStyleId) && draft.primaryStyleId !== draft.subStyleId
        && draft.selectedPerkIds.length === 9 && draft.selectedPerkIds.every(validId)
        && new Set(draft.selectedPerkIds.slice(0, 6)).size === 6;
}

/** La validité métier du brouillon est contrôlée avec le catalogue avant l'import ; celle du retour est attestée par le client. */
export function runePagesMatch(page: RunePage | null, draft: RuneDraft): boolean {
    if (!page?.isValid || !completeShape(page) || !completeShape(draft)
        || page.primaryStyleId !== draft.primaryStyleId || page.subStyleId !== draft.subStyleId) return false;
    return draft.selectedPerkIds.every((id, index) => index === 4 || index === 5
        ? page.selectedPerkIds.slice(4, 6).includes(id) : page.selectedPerkIds[index] === id);
}
