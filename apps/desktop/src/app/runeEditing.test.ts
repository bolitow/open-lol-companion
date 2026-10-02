import {describe, expect, it} from 'vitest';
import type {CatalogRecord, CatalogValue, JsonValue, RunePage} from '@olc/shared';
import {changeRuneStyle, chooseRune, runeDraftFromPage, runeDraftKey, runePagesMatch, validRuneDraft, type RuneDraft} from './runeEditing';

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
function shard(id: number, slots: number[], styles = [8000, 8200, 8400]): CatalogRecord {
    return record(id, 'rune_shard', {listed_in_perk_styles: value(true, 'derived'), rune_page_slots: value(styles.flatMap(style =>
        slots.map(slot_index => ({style_id: String(style), slot_type: 'kStatMod', slot_index}))), 'derived')});
}
function catalog(): CatalogRecord[] {
    return [8000, 8200, 8400].map(id => record(id, 'rune', {rune_kind: value('style')})).concat([
        rune(8005, 8000, 0), rune(9111, 8000, 1), rune(9104, 8000, 2), rune(8014, 8000, 3), rune(9101, 8000, 1),
        rune(8214, 8200, 0), rune(8224, 8200, 1), rune(8226, 8200, 1), rune(8234, 8200, 2), rune(8237, 8200, 3),
        rune(8446, 8400, 1), shard(5005, [4], [8000]), shard(5008, [4, 5]), shard(5011, [6]),
    ]);
}
function draft(): RuneDraft {
    return {primaryStyleId: 8000, subStyleId: 8200, selectedPerkIds: [8005, 9111, 9104, 8014, 8224, 8234, 5005, 5008, 5011]};
}
function page(changes: Partial<RunePage> = {}): RunePage {
    return {...draft(), isValid: true, isTemporary: false, autoModifiedSelections: [], ...changes};
}

describe('brouillon de runes explicite', () => {
    it('laisse neuf places vides sans page et ne devine aucun arbre', () => {
        expect(runeDraftFromPage(null)).toEqual({primaryStyleId: 0, subStyleId: 0, selectedPerkIds: [0, 0, 0, 0, 0, 0, 0, 0, 0]});
    });
    it('copie les choix et conserve les places absentes sans les compresser', () => {
        const source = page({selectedPerkIds: [8005, 0, 9104, NaN, 8224]});
        const result = runeDraftFromPage(source);
        expect(result).toEqual({primaryStyleId: 8000, subStyleId: 8200, selectedPerkIds: [8005, 0, 9104, 0, 8224, 0, 0, 0, 0]});
        result.selectedPerkIds[0] = 0;
        expect(source.selectedPerkIds[0]).toBe(8005);
        expect(runeDraftFromPage(page({primaryStyleId: -1, subStyleId: Infinity, selectedPerkIds: Array(10).fill(5008)})))
            .toEqual({primaryStyleId: 0, subStyleId: 0, selectedPerkIds: Array(9).fill(5008)});
    });
    it('distingue chaque choix et chaque arbre dans la clé sans dépendre de l’ordre des propriétés', () => {
        const source = draft(), key = runeDraftKey(source);
        expect(runeDraftKey({selectedPerkIds: [...source.selectedPerkIds], subStyleId: 8200, primaryStyleId: 8000})).toBe(key);
        expect(runeDraftKey({...source, primaryStyleId: 8400})).not.toBe(key);
        expect(runeDraftKey({...source, subStyleId: 8400})).not.toBe(key);
        for (let index = 0; index < 9; index++) {
            const changed = draft(); changed.selectedPerkIds[index] = 0;
            expect(runeDraftKey(changed)).not.toBe(key);
        }
    });
});

describe('changement d’arbre', () => {
    it('vide les primaires et ne conserve que les fragments compatibles avec le nouvel arbre', () => {
        const source = draft();
        expect(changeRuneStyle(source, 'primary', 8400, catalog())).toEqual({primaryStyleId: 8400, subStyleId: 8200,
            selectedPerkIds: [0, 0, 0, 0, 8224, 8234, 0, 5008, 5011]});
        expect(source).toEqual(draft());
    });
    it('vide le secondaire devenu identique au primaire et n’en invente pas un autre', () => {
        expect(changeRuneStyle(draft(), 'primary', 8200, catalog())).toEqual({primaryStyleId: 8200, subStyleId: 0,
            selectedPerkIds: [0, 0, 0, 0, 0, 0, 0, 5008, 5011]});
        expect(changeRuneStyle(draft(), 'secondary', 8000, catalog())).toEqual(draft());
    });
    it('ne vide que les deux choix lors d’un changement secondaire', () => {
        expect(changeRuneStyle(draft(), 'secondary', 8400, catalog())).toEqual({primaryStyleId: 8000, subStyleId: 8400,
            selectedPerkIds: [8005, 9111, 9104, 8014, 0, 0, 5005, 5008, 5011]});
        expect(changeRuneStyle(draft(), 'primary', 8000, catalog())).toEqual(draft());
        expect(changeRuneStyle(draft(), 'secondary', 8200, catalog())).toEqual(draft());
    });
    it('refuse les arbres inconnus, ordinaires, ambigus ou non vérifiés', () => {
        for (const id of [0, -1, 1.5, NaN, 4294967296, 9999, 9111]) expect(changeRuneStyle(draft(), 'primary', id, catalog())).toEqual(draft());
        for (const change of [{namespace: 'other'}, {locale: 'en_US'}, {fields: {rune_kind: value('style', 'descriptive')}}]) {
            const records = catalog(); Object.assign(records.find(row => row.id === '8400')!, change);
            expect(changeRuneStyle(draft(), 'secondary', 8400, records)).toEqual(draft());
        }
        const records = catalog(); records.push({...records.find(row => row.id === '8400')!});
        expect(changeRuneStyle(draft(), 'primary', 8400, records)).toEqual(draft());
    });
});

describe('choix manuel des runes', () => {
    it('remplace uniquement la bonne ligne primaire et refuse les choix hors ligne ou arbre', () => {
        const source = draft();
        expect(chooseRune(source, 'primary', 1, 9101, catalog()).selectedPerkIds).toEqual([8005, 9101, 9104, 8014, 8224, 8234, 5005, 5008, 5011]);
        expect(source).toEqual(draft());
        for (const [slot, id] of [[1, 9104], [1, 8224], [-1, 8005], [4, 8014], [1.5, 9101], [1, 9999]]) {
            expect(chooseRune(source, 'primary', slot!, id!, catalog())).toEqual(source);
        }
    });
    it('remplace une secondaire de la même ligne puis évince la plus ancienne au troisième choix', () => {
        const changed = chooseRune(draft(), 'secondary', 1, 8226, catalog());
        expect(changed.selectedPerkIds.slice(4, 6)).toEqual([8234, 8226]);
        const third = chooseRune(changed, 'secondary', 3, 8237, catalog());
        expect(third.selectedPerkIds.slice(4, 6)).toEqual([8226, 8237]);
        expect(validRuneDraft(third, catalog())).toBe(true);
        expect(chooseRune(third, 'secondary', 1, 8226, catalog())).toEqual(third);
    });
    it('remplit les secondaires vides et refuse la fondamentale et les lignes incohérentes', () => {
        const empty = {...draft(), selectedPerkIds: [8005, 9111, 9104, 8014, 0, 0, 5005, 5008, 5011]};
        const first = chooseRune(empty, 'secondary', 3, 8237, catalog());
        expect(first.selectedPerkIds.slice(4, 6)).toEqual([8237, 0]);
        expect(chooseRune(first, 'secondary', 2, 8234, catalog()).selectedPerkIds.slice(4, 6)).toEqual([8237, 8234]);
        for (const [slot, id] of [[0, 8214], [1, 8234], [4, 8237], [2, 9104]]) {
            expect(chooseRune(empty, 'secondary', slot!, id!, catalog())).toEqual(empty);
        }
    });
    it('autorise les fragments répétés uniquement aux emplacements présents dans le catalogue', () => {
        const repeated = chooseRune(draft(), 'shard', 0, 5008, catalog());
        expect(repeated.selectedPerkIds.slice(6)).toEqual([5008, 5008, 5011]);
        expect(validRuneDraft(repeated, catalog())).toBe(true);
        for (const [slot, id] of [[0, 5011], [1, 5005], [2, 5008], [3, 5011], [-1, 5008]]) {
            expect(chooseRune(draft(), 'shard', slot!, id!, catalog())).toEqual(draft());
        }
    });
    it('refuse les choix ambigus, non structurels et d’une autre langue', () => {
        for (const change of [{namespace: 'other'}, {locale: 'en_US'}, {fields: {rune_kind: value('rune'), style_id: value('8000'), slot: value(1, 'descriptive')}}]) {
            const records = catalog(); Object.assign(records.find(row => row.id === '9101')!, change);
            expect(chooseRune(draft(), 'primary', 1, 9101, records)).toEqual(draft());
        }
        const records = catalog(); records.push({...records.find(row => row.id === '9101')!});
        expect(chooseRune(draft(), 'primary', 1, 9101, records)).toEqual(draft());
    });
});

describe('page applicable et synchronisation avec le client', () => {
    it('exige neuf choix compatibles vérifiés avant de permettre l’import', () => {
        expect(validRuneDraft(draft(), catalog())).toBe(true);
        for (const selectedPerkIds of [[8005], [...draft().selectedPerkIds, 5008], [8005, 9111, 9104, 8014, 8224, 8226, 5005, 5008, 5011]]) {
            expect(validRuneDraft({...draft(), selectedPerkIds}, catalog())).toBe(false);
        }
        expect(validRuneDraft(runeDraftFromPage(null), catalog())).toBe(false);
        expect(validRuneDraft({...draft(), subStyleId: 8000}, catalog())).toBe(false);
        const records = catalog(); records.find(row => row.id === '5005')!.fields.listed_in_perk_styles = value(false);
        expect(validRuneDraft(draft(), records)).toBe(false);
    });
    it('reconnaît une page client valide même si les deux secondaires sont renvoyées dans l’autre ordre', () => {
        expect(runePagesMatch(page(), draft())).toBe(true);
        expect(runePagesMatch(page({selectedPerkIds: [8005, 9111, 9104, 8014, 8234, 8224, 5005, 5008, 5011]}), draft())).toBe(true);
        expect(runePagesMatch(page({isValid: false}), draft())).toBe(false);
        expect(runePagesMatch(null, draft())).toBe(false);
        expect(runePagesMatch(page(), {...draft(), subStyleId: 8400})).toBe(false);
        for (let index = 0; index < 9; index++) {
            const changed = draft(); changed.selectedPerkIds[index] = 0;
            expect(runePagesMatch(page(), changed)).toBe(false);
        }
    });
    it('ne confond pas deux pages invalides identiques avec une application réussie', () => {
        for (const invalid of [runeDraftFromPage(null), {...draft(), subStyleId: 8000}, {...draft(), selectedPerkIds: [8005]},
            {...draft(), selectedPerkIds: [8005, 9111, 9104, 8014, 8224, 8224, 5005, 5008, 5011]}]) {
            expect(runePagesMatch({...page(), ...invalid}, invalid)).toBe(false);
        }
    });
});
