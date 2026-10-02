import type {BuildReport, BuildRequest, BuildStats, CatalogRecord, JsonValue, RunePage} from '@olc/shared';

export function buildRequestKey(request: BuildRequest): string {
    return JSON.stringify([request.champion_id, request.patch, request.platform, request.queue, request.role, request.rank]);
}

function structuralValue(record: CatalogRecord, field: string): JsonValue | null {
    const entry = record.fields[field];
    return entry && (entry.status === 'verified' || entry.status === 'derived') ? entry.value : null;
}

function objectValue(value: JsonValue): value is {[key: string]: JsonValue} {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

/** Les onze IDs agrégés ne deviennent une page valide qu'après contrôle du référentiel. */
export function runePageFromBuild(selection: readonly number[], records: readonly CatalogRecord[]): RunePage | null {
    if (selection.length !== 11 || selection.some(id => !Number.isSafeInteger(id) || id <= 0 || id > 0xffffffff)) return null;
    const primaryStyleId = selection[0]!, subStyleId = selection[5]!;
    if (primaryStyleId === subStyleId) return null;
    const find = (id: number, kind: string): CatalogRecord | null => {
        const matches = records.filter(record => record.id === String(id) && record.kind === kind && record.namespace === 'standard');
        return matches.length === 1 ? matches[0]! : null;
    };
    const primary = find(primaryStyleId, 'rune'), secondary = find(subStyleId, 'rune');
    if (!primary || !secondary || structuralValue(primary, 'rune_kind') !== 'style'
        || structuralValue(secondary, 'rune_kind') !== 'style' || secondary.locale !== primary.locale) return null;
    const runeSlot = (id: number, style: number): number | null => {
        const rune = find(id, 'rune');
        if (!rune || rune.locale !== primary.locale || structuralValue(rune, 'rune_kind') !== 'rune'
            || structuralValue(rune, 'style_id') !== String(style)) return null;
        const slot = structuralValue(rune, 'slot');
        return typeof slot === 'number' && Number.isInteger(slot) && slot >= 0 && slot <= 3 ? slot : null;
    };
    for (let slot = 0; slot < 4; slot++) {
        if (runeSlot(selection[1 + slot]!, primaryStyleId) !== slot) return null;
    }
    const secondarySlots = [runeSlot(selection[6]!, subStyleId), runeSlot(selection[7]!, subStyleId)];
    if (secondarySlots.some(slot => slot === null || slot === 0) || secondarySlots[0] === secondarySlots[1]) return null;
    for (let slot = 0; slot < 3; slot++) {
        const shard = find(selection[8 + slot]!, 'rune_shard');
        if (!shard || shard.locale !== primary.locale || structuralValue(shard, 'listed_in_perk_styles') !== true) return null;
        const positions = structuralValue(shard, 'rune_page_slots');
        // Les rangées kStatMod suivent les quatre rangées primaires ; aucune compression des absences.
        if (!Array.isArray(positions) || !positions.some(position => objectValue(position)
            && position.style_id === String(primaryStyleId) && position.slot_type === 'kStatMod' && position.slot_index === 4 + slot)) return null;
    }
    return {primaryStyleId, subStyleId, selectedPerkIds: [...selection.slice(1, 5), ...selection.slice(6)],
        isValid: true, isTemporary: false, autoModifiedSelections: []};
}

function compareSelections(left: readonly number[], right: readonly number[]): number {
    for (let index = 0; index < Math.min(left.length, right.length); index++) {
        const difference = left[index]! - right[index]!;
        if (difference !== 0) return difference;
    }
    return left.length - right.length;
}

export function variantsFor(report: BuildReport, category: string): BuildStats[] {
    return report.builds.filter(build => build.category === category)
        .sort((left, right) => right.games - left.games || compareSelections(left.selection, right.selection));
}

export type SkillKey = 'Q' | 'W' | 'E' | 'R';

/** L'ordre des points investis ne prédit ni le niveau du champion ni les points futurs. */
export function skillSequence(selection: readonly number[]): SkillKey[] | null {
    if (selection.length === 0 || selection.length > 64 || selection.some(slot => !Number.isInteger(slot) || slot < 1 || slot > 4)) return null;
    const keys: readonly SkillKey[] = ['Q', 'W', 'E', 'R'];
    return selection.map(slot => keys[slot - 1]!);
}

export interface BuildMetrics {
    pickRate: number | null;
    winRate: number | null;
    games: number;
    population: number;
}

function publishedRate(rate: number | null): number | null {
    return rate !== null && Number.isFinite(rate) && rate >= 0 && rate <= 100 ? rate : null;
}

export function buildMetrics(variant: BuildStats, minGames: number): BuildMetrics {
    const {games, population, wins} = variant;
    const eligible = Number.isSafeInteger(minGames) && minGames > 0 && Number.isSafeInteger(games) && games >= minGames;
    const populationKnown = Number.isSafeInteger(population) && population >= games;
    const winsKnown = wins !== null && Number.isSafeInteger(wins) && wins >= 0 && wins <= games;
    return {
        pickRate: eligible && populationKnown ? publishedRate(variant.pick_rate) : null,
        winRate: eligible && winsKnown && variant.performance_available ? publishedRate(variant.win_rate) : null,
        games,
        population,
    };
}
