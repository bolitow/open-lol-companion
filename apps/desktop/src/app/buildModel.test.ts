import {describe, expect, it} from 'vitest';
import type {BuildReport, BuildRequest, BuildStats, CatalogRecord, CatalogValue, JsonValue} from '@olc/shared';
import {buildMetrics, buildRequestKey, runePageFromBuild, skillSequence, variantsFor} from './buildModel';

const request: BuildRequest = {champion_id: 103, patch: '16.19', platform: 'EUW1', queue: 420, role: 'MIDDLE', rank: 'ALL'};
const selection = [8000, 8005, 9111, 9104, 8014, 8200, 8224, 8234, 5005, 5008, 5011];
function value(data: JsonValue, status: CatalogValue['status'] = 'verified'): CatalogValue {
    return {value: data, status, unit: null, sources: []};
}
function record(id: number, kind: string, fields: CatalogRecord['fields']): CatalogRecord {
    return {id: String(id), kind, namespace: 'standard', locale: 'fr_FR', name: String(id), description: null, icon: null,
        fields, stats: {}, effects: [], coverage: {source_fields: 0, normalized_fields: 0, unmapped_fields: [], issues: []}};
}
function rune(id: number, style: number, slot: number): CatalogRecord {
    return record(id, 'rune', {rune_kind: value('rune'), style_id: value(String(style), 'derived'), slot: value(slot, 'derived')});
}
function shard(id: number, slots: number[]): CatalogRecord {
    return record(id, 'rune_shard', {listed_in_perk_styles: value(true, 'derived'),
        rune_page_slots: value(slots.map(slot_index => ({style_id: '8000', slot_type: 'kStatMod', slot_index})), 'derived')});
}
function catalog(): CatalogRecord[] {
    return [record(8000, 'rune', {rune_kind: value('style')}), record(8200, 'rune', {rune_kind: value('style')}),
        rune(8005, 8000, 0), rune(9111, 8000, 1), rune(9104, 8000, 2), rune(8014, 8000, 3),
        rune(8224, 8200, 1), rune(8234, 8200, 2), rune(8226, 8200, 1), rune(8214, 8200, 0),
        shard(5005, [4]), shard(5008, [4, 5]), shard(5011, [6])];
}
function variant(changes: Partial<BuildStats> = {}): BuildStats {
    return {patch: '16.19', platform_id: 'EUW1', queue_id: 420, role: 'MIDDLE', rank: 'ALL', champion_id: 103,
        category: 'runes', selection: [...selection], games: 100, wins: 60, population: 200,
        performance_available: true, pick_rate: 50, win_rate: 60, win_rate_lower_bound: null, placement_games: 0, average_placement: null, ...changes};
}
function report(builds: BuildStats[]): BuildReport {
    return {request, meta: {source_snapshot_at: '2026-10-01T10:00:00Z', published_at: '2026-10-01T10:01:00Z', min_games: 100}, builds};
}

describe('identité de la population demandée', () => {
    it('isole chaque dimension et ignore l’ordre de création des propriétés', () => {
        const key = buildRequestKey(request);
        expect(buildRequestKey({rank: 'ALL', role: 'MIDDLE', queue: 420, platform: 'EUW1', patch: '16.19', champion_id: 103})).toBe(key);
        for (const change of [{champion_id: 1}, {patch: '16.18'}, {platform: 'KR'}, {queue: 440}, {role: 'TOP' as const}, {rank: 'GOLD'}]) {
            expect(buildRequestKey({...request, ...change})).not.toBe(key);
        }
    });
});

describe('page de runes observée', () => {
    it('décode les deux arbres et les neuf choix sans modifier la sélection', () => {
        const source = [...selection];
        expect(runePageFromBuild(source, catalog())).toEqual({primaryStyleId: 8000, subStyleId: 8200,
            selectedPerkIds: [8005, 9111, 9104, 8014, 8224, 8234, 5005, 5008, 5011],
            isValid: true, isTemporary: false, autoModifiedSelections: []});
        expect(source).toEqual(selection);
    });
    it('conserve un fragment répété lorsque ses deux emplacements sont autorisés', () => {
        const repeated = [...selection]; repeated[8] = 5008;
        expect(runePageFromBuild(repeated, catalog())?.selectedPerkIds.slice(6)).toEqual([5008, 5008, 5011]);
    });
    it('refuse une sélection tronquée, surnuméraire ou un identifiant mal formé', () => {
        expect(runePageFromBuild(selection.slice(0, 10), catalog())).toBeNull();
        expect(runePageFromBuild([...selection, 5008], catalog())).toBeNull();
        for (const id of [0, -1, 1.5, NaN, Infinity, 4294967296]) {
            const invalid = [...selection]; invalid[1] = id;
            expect(runePageFromBuild(invalid, catalog())).toBeNull();
        }
    });
    it('refuse un arbre inconnu, identique au primaire ou qui est une rune ordinaire', () => {
        for (const id of [9999, 8000, 8224]) {
            const invalid = [...selection]; invalid[5] = id;
            expect(runePageFromBuild(invalid, catalog())).toBeNull();
        }
    });
    it('refuse les runes primaires dans la mauvaise ligne ou dans un autre arbre', () => {
        for (const id of [9104, 8224, 9999]) {
            const invalid = [...selection]; invalid[2] = id;
            expect(runePageFromBuild(invalid, catalog())).toBeNull();
        }
    });
    it('refuse une fondamentale secondaire et deux runes secondaires de la même ligne', () => {
        for (const id of [8214, 8226, 8224]) {
            const invalid = [...selection]; invalid[7] = id;
            expect(runePageFromBuild(invalid, catalog())).toBeNull();
        }
        const reversed = [...selection]; reversed[6] = 8234; reversed[7] = 8224;
        expect(runePageFromBuild(reversed, catalog())?.selectedPerkIds.slice(4, 6)).toEqual([8234, 8224]);
    });
    it('valide les fragments par emplacement exact sans comprimer les lignes manquantes', () => {
        const invalid = [...selection]; invalid[8] = 5011;
        expect(runePageFromBuild(invalid, catalog())).toBeNull();
        const records = catalog();
        records.find(r => r.id === '5008')!.fields.rune_page_slots = value([{style_id: '8000', slot_type: 'kStatMod', slot_index: 6}]);
        expect(runePageFromBuild(selection, records)).toBeNull();
    });
    it('refuse les fragments historiques et les emplacements du mauvais arbre ou type', () => {
        for (const change of [
            {listed_in_perk_styles: value(false)},
            {rune_page_slots: value([{style_id: '8200', slot_type: 'kStatMod', slot_index: 4}])},
            {rune_page_slots: value([{style_id: '8000', slot_type: 'kMixedRegularSplashable', slot_index: 4}])},
        ]) {
            const records = catalog(); Object.assign(records.find(r => r.id === '5005')!.fields, change);
            expect(runePageFromBuild(selection, records)).toBeNull();
        }
    });
    it('exige des champs structurels vérifiés ou dérivés', () => {
        for (const [id, field] of [['8000', 'rune_kind'], ['9111', 'style_id'], ['9111', 'slot'], ['5005', 'listed_in_perk_styles'], ['5005', 'rune_page_slots']]) {
            for (const status of ['descriptive', 'missing', 'unsupported', 'conflict'] as const) {
                const records = catalog(); const target = records.find(r => r.id === id)!;
                target.fields[field!] = {...target.fields[field!]!, status};
                expect(runePageFromBuild(selection, records)).toBeNull();
            }
        }
    });
    it('ne mélange pas les espaces, langues ou identifiants ambigus du catalogue', () => {
        for (const change of [{namespace: 'classic'}, {locale: 'en_US'}]) {
            const records = catalog(); Object.assign(records.find(r => r.id === '9111')!, change);
            expect(runePageFromBuild(selection, records)).toBeNull();
        }
        expect(runePageFromBuild(selection, [...catalog(), rune(9111, 8000, 2)])).toBeNull();
    });
});

describe('variantes et ordre de compétences', () => {
    it('filtre une catégorie et trie par effectif puis sélection numérique sans muter le rapport', () => {
        const low = variant({games: 80, selection: [1]}), later = variant({selection: [10]}),
            longer = variant({selection: [2, 1]}), first = variant({selection: [2]}), other = variant({category: 'item', selection: [1001]});
        const source = report([low, later, longer, other, first]);
        expect(variantsFor(source, 'runes')).toEqual([first, longer, later, low]);
        expect(source.builds).toEqual([low, later, longer, other, first]);
        expect(variantsFor(source, 'missing')).toEqual([]);
    });
    it('décode uniquement les points investis sans compléter jusqu’à dix-huit niveaux', () => {
        expect(skillSequence([1, 2, 3, 1, 1, 4])).toEqual(['Q', 'W', 'E', 'Q', 'Q', 'R']);
        expect(skillSequence(Array(20).fill(1))).toHaveLength(20);
        for (const slots of [[], [0], [5], [1.5], [NaN], [Infinity], [1, 2, -1], Array(65).fill(1)]) {
            expect(skillSequence(slots)).toBeNull();
        }
    });
});

describe('métriques publiées', () => {
    it('conserve les taux publiés et les effectifs sans recalculer une valeur absente', () => {
        expect(buildMetrics(variant(), 100)).toEqual({pickRate: 50, winRate: 60, games: 100, population: 200});
        expect(buildMetrics(variant({pick_rate: null, win_rate: null}), 100)).toEqual({pickRate: null, winRate: null, games: 100, population: 200});
        expect(buildMetrics(variant({pick_rate: 0, win_rate: 0, wins: 0}), 100).winRate).toBe(0);
        expect(buildMetrics(variant({pick_rate: 100, win_rate: 100, wins: 100}), 100).pickRate).toBe(100);
    });
    it('masque les taux sous le seuil et conserve zéro comme effectif observé', () => {
        expect(buildMetrics(variant({games: 99}), 100)).toEqual({pickRate: null, winRate: null, games: 99, population: 200});
        expect(buildMetrics(variant({games: 0, wins: 0, population: 0}), 100)).toEqual({pickRate: null, winRate: null, games: 0, population: 0});
        for (const min of [0, -1, 1.5, NaN, Infinity]) {
            expect(buildMetrics(variant(), min)).toMatchObject({pickRate: null, winRate: null});
        }
    });
    it('garde la popularité quand la performance est indisponible', () => {
        expect(buildMetrics(variant({performance_available: false, wins: null, win_rate: 60}), 100))
            .toEqual({pickRate: 50, winRate: null, games: 100, population: 200});
        expect(buildMetrics(variant({wins: null}), 100).winRate).toBeNull();
    });
    it('refuse les taux non finis ou hors bornes sans les borner artificiellement', () => {
        for (const rate of [-1, 100.01, NaN, Infinity]) {
            expect(buildMetrics(variant({pick_rate: rate, win_rate: rate}), 100)).toMatchObject({pickRate: null, winRate: null});
        }
        expect(buildMetrics(variant({population: 99}), 100).pickRate).toBeNull();
        expect(buildMetrics(variant({games: 1.5}), 1)).toMatchObject({pickRate: null, winRate: null});
        expect(buildMetrics(variant({wins: 101}), 100).winRate).toBeNull();
    });
});
