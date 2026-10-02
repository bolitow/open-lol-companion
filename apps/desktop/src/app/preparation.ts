import type {CatalogRecord, JsonValue} from '@olc/shared';

export function readableValue(record: CatalogRecord, key: string): JsonValue | null {
    const field = record.fields[key];
    return field && ['verified', 'derived', 'descriptive'].includes(field.status) ? field.value : null;
}

export function runeStyles(records: readonly CatalogRecord[]): CatalogRecord[] {
    return records.filter(record => record.kind === 'rune' && readableValue(record, 'rune_kind') === 'style');
}

function sourceOrderedRow(row: CatalogRecord[], slotIndex: number): CatalogRecord[] {
    const ranked: Array<{record: CatalogRecord; index: number}> = [];
    let styleIndex: string | undefined;
    const indices = new Set<number>();
    for (const record of row) {
        const slot = record.fields.slot;
        if (!slot || (slot.status !== 'derived' && slot.status !== 'verified')) return row;
        const pointers = new Set(slot.sources.map(source => source.pointer));
        if (pointers.size !== 1) return row;
        const pointer = [...pointers][0];
        const match = pointer?.match(/^\/(0|[1-9]\d*)\/slots\/(0|[1-9]\d*)\/runes\/(0|[1-9]\d*)$/);
        if (!match || Number(match[2]) !== slotIndex) return row;
        const index = Number(match[3]);
        if (!Number.isSafeInteger(index) || indices.has(index) || (styleIndex !== undefined && styleIndex !== match[1])) return row;
        styleIndex = match[1];
        indices.add(index);
        ranked.push({record, index});
    }
    // Une provenance incomplète laisse toute la rangée intacte, sans ordre partiellement déduit.
    return ranked.sort((left, right) => left.index - right.index).map(entry => entry.record);
}

export function runeRows(records: readonly CatalogRecord[], styleId: string, secondary: boolean): CatalogRecord[][] {
    const firstSlot = secondary ? 1 : 0;
    const rows: CatalogRecord[][] = Array.from({length: secondary ? 3 : 4}, () => []);
    for (const record of records) {
        if (record.kind !== 'rune' || readableValue(record, 'rune_kind') !== 'rune' || readableValue(record, 'style_id') !== styleId) continue;
        const slot = readableValue(record, 'slot');
        if (typeof slot === 'number' && Number.isInteger(slot) && slot >= firstSlot && slot < 4) rows[slot - firstSlot]?.push(record);
    }
    return rows.map((row, index) => sourceOrderedRow(row, index + firstSlot));
}

function objectValue(value: JsonValue): value is {[key: string]: JsonValue} {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export function shardRows(records: readonly CatalogRecord[], styleId: string): CatalogRecord[][] {
    const rows = new Map<number, CatalogRecord[]>();
    for (const record of records) {
        if (record.kind !== 'rune_shard' || readableValue(record, 'listed_in_perk_styles') !== true) continue;
        const slots = readableValue(record, 'rune_page_slots');
        if (!Array.isArray(slots)) continue;
        // Un fragment peut appartenir à plusieurs rangées, mais une seule fois par rangée.
        const indices = new Set<number>();
        for (const slot of slots) {
            if (objectValue(slot) && slot.style_id === styleId && slot.slot_type === 'kStatMod'
                && typeof slot.slot_index === 'number' && Number.isInteger(slot.slot_index) && slot.slot_index >= 0) indices.add(slot.slot_index);
        }
        for (const index of indices) {
            const row = rows.get(index) ?? [];
            row.push(record);
            rows.set(index, row);
        }
    }
    return [...rows].sort(([left], [right]) => left - right).map(([, row]) => row);
}

function searchText(text: string): string {
    return text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase().trim();
}

export function searchableItems(records: readonly CatalogRecord[], query: string): CatalogRecord[] {
    const search = searchText(query);
    return records.filter(record => {
        const maps = readableValue(record, 'maps');
        return record.kind === 'item' && objectValue(maps) && maps['11'] === true
            && readableValue(record, 'in_store') === true && readableValue(record, 'purchasable') === true
            && searchText(record.name).includes(search);
    });
}

export function itemComponents(records: readonly CatalogRecord[], item: CatalogRecord): Array<{id: string; record: CatalogRecord | null}> {
    const components = readableValue(item, 'builds_from');
    if (!Array.isArray(components)) return [];
    const result: Array<{id: string; record: CatalogRecord | null}> = [];
    for (const component of components) {
        const id = typeof component === 'string' && component.length > 0 ? component
            : typeof component === 'number' && Number.isSafeInteger(component) && component > 0 ? String(component) : null;
        if (id === null) continue;
        const record = records.find(candidate => candidate.kind === 'item' && candidate.id === id
            && candidate.locale === item.locale && candidate.namespace === item.namespace) ?? null;
        result.push({id, record});
    }
    return result;
}

export function itemStats(record: CatalogRecord): Array<{key: string; value: number; unit: string | null}> {
    return Object.entries(record.stats).flatMap(([key, stat]) =>
        (stat.status === 'verified' || stat.status === 'derived') && typeof stat.value === 'number' && Number.isFinite(stat.value)
            ? [{key, value: stat.value, unit: stat.unit}] : []);
}

export function displayedRuneStyles(styles:readonly CatalogRecord[],page:import('@olc/shared').RunePage|null):[string,string]{
    if(!page)return [styles[0]?.id??'',styles[1]?.id??''];
    const find=(id:number)=>styles.find(style=>style.id===String(id))?.id??'';
    return [find(page.primaryStyleId),find(page.subStyleId)];
}

/** Une variable non résolue n'est ni une valeur zéro ni un conseil exploitable. */
export function catalogDescription(record:CatalogRecord):string|null{
    const text=record.description;
    return text&& !/@[^@\n]+@|\{\{[^}]*\}\}/.test(text)?text:null;
}

/** Une page contrôlée garde la priorité sur une exploration locale antérieure. */
export function activeRuneExploration(exploring: [string,string] | null, fixedPage: boolean, editing: boolean): [string,string] | null {
    return editing || fixedPage ? null : exploring;
}
