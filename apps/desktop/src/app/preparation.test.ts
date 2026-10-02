import {describe, expect, it} from 'vitest';
import type {CatalogRecord, CatalogValue, JsonValue} from '@olc/shared';
import {activeRuneExploration,displayedRuneStyles,catalogDescription,itemComponents, itemStats, readableValue, runeRows, runeStyles, searchableItems, shardRows} from './preparation';

function value(data: JsonValue, status: CatalogValue['status'] = 'verified', unit: string | null = null): CatalogValue {
    return {value: data, status, unit, sources: []};
}
function record(id: string, kind: string, fields: CatalogRecord['fields'] = {}): CatalogRecord {
    return {id, kind, namespace: 'standard', locale: 'fr_FR', name: id, description: null, icon: null,
        fields, stats: {}, effects: [], coverage: {source_fields: 0, normalized_fields: 0, unmapped_fields: [], issues: []}};
}
function rune(id: string, slot: number, style = '8000'): CatalogRecord {
    return record(id, 'rune', {rune_kind: value('rune'), style_id: value(style, 'derived'), slot: value(slot, 'derived')});
}
function item(id: string): CatalogRecord {
    return record(id, 'item', {maps: value({'11': true}), purchasable: value(true), in_store: value(true)});
}
function shard(id: string, slots: number[], style = '8000'): CatalogRecord {
    return record(id, 'rune_shard', {listed_in_perk_styles: value(true, 'derived'),
        rune_page_slots: value(slots.map(slot_index => ({style_id: style, slot_index, slot_type: 'kStatMod'})), 'derived')});
}

describe('valeurs du référentiel utilisables', () => {
    it('préserve zéro et faux et ne publie pas les valeurs non résolues', () => {
        const r = record('1', 'item', {zero: value(0), no: value(false, 'derived'), text: value('description', 'descriptive'),
            absent: value(12, 'missing'), conflict: value(40, 'conflict'), unsupported: value(50, 'unsupported')});
        expect(['zero', 'no', 'text', 'absent', 'conflict', 'unsupported', 'unknown'].map(key => readableValue(r, key)))
            .toEqual([0, false, 'description', null, null, null, null]);
    });
});

describe('arbres et fragments de runes', () => {
    it('sépare les styles des runes et ignore un type conflictuel', () => {
        const style = record('8000', 'rune', {rune_kind: value('style', 'derived')});
        const conflict = record('8100', 'rune', {rune_kind: value('style', 'conflict')});
        expect(runeStyles([rune('8005', 0), style, conflict, record('x', 'item', style.fields)])).toEqual([style]);
    });
    it('conserve toutes les alternatives, les rangées vides et exclut la fondamentale en secondaire', () => {
        const records = [rune('8005', 0), rune('8008', 0), rune('9101', 1), rune('9104', 2), rune('9111', 2), rune('8014', 3),
            rune('other', 1, '8100'), rune('invalid', 4), rune('fraction', 1.5)];
        records.push({...rune('conflict', 1), fields: {style_id: value('8000', 'conflict'), slot: value(1)}});
        expect(runeRows(records, '8000', false).map(row => row.map(r => r.id)))
            .toEqual([['8005', '8008'], ['9101'], ['9104', '9111'], ['8014']]);
        expect(runeRows(records, '8000', true).map(row => row.map(r => r.id)))
            .toEqual([['9101'], ['9104', '9111'], ['8014']]);
        expect(runeRows([], 'unknown', false)).toEqual([[], [], [], []]);
        expect(runeRows([rune('only', 2)], '8000', true).map(row => row.map(r => r.id))).toEqual([[], ['only'], []]);
    });
    it('retrouve l’ordre Data Dragon depuis la provenance du slot et non depuis l’identifiant de rune', () => {
        const conqueror = rune('8010', 0);
        conqueror.fields.slot = {...value(0, 'derived'), sources: [{source_id: 'fr/runes', pointer: '/0/slots/0/runes/3'}]};
        const fleet = rune('8021', 0);
        fleet.fields.slot = {...value(0, 'verified'), sources: [{source_id: 'fr/runes', pointer: '/0/slots/0/runes/2'}]};
        const press = rune('8005', 0);
        press.fields.slot = {...value(0, 'derived'), sources: [{source_id: 'fr/runes', pointer: '/0/slots/0/runes/0'}]};
        const records = [conqueror, fleet, press];
        expect(runeRows(records, '8000', false)[0]?.map(r => r.id)).toEqual(['8005', '8021', '8010']);
        expect(records.map(r => r.id)).toEqual(['8010', '8021', '8005']);
    });
    it('garde l’ordre entrant si la provenance du slot manque, contredit la rangée ou est ambiguë', () => {
        const later = rune('later', 1);
        later.fields.slot = {...value(1, 'derived'), sources: [{source_id: 'fr/runes', pointer: '/0/slots/1/runes/2'}]};
        const fallbackFields: CatalogValue[] = [
            value(1, 'derived'),
            {...value(1, 'descriptive'), sources: [{source_id: 'fr/runes', pointer: '/0/slots/1/runes/0'}]},
            {...value(1, 'derived'), sources: [{source_id: 'fr/runes', pointer: '/0/slots/0/runes/0'}]},
            {...value(1, 'derived'), sources: [{source_id: 'fr/runes', pointer: '/0/slots/1/runes/0/extra'}]},
            {...value(1, 'derived'), sources: [{source_id: 'fr/runes', pointer: '/0/slots/1/runes/0'}, {source_id: 'other', pointer: '/0/slots/1/runes/1'}]},
            {...value(1, 'derived'), sources: [{source_id: 'fr/runes', pointer: '/0/slots/1/runes/2'}]},
        ];
        for (const field of fallbackFields) {
            const uncertain = rune('uncertain', 1);
            uncertain.fields.slot = field;
            uncertain.fields.style_id = {...value('8000', 'derived'), sources: [{source_id: 'fr/runes', pointer: '/0/slots/1/runes/0'}]};
            expect(runeRows([later, uncertain], '8000', true)[0]?.map(r => r.id)).toEqual(['later', 'uncertain']);
        }
    });
    it('garde un fragment dans chaque rangée autorisée sans ressusciter les fragments historiques', () => {
        const repeated = shard('5008', [4, 5, 5]);
        const historical = shard('5002', [6]);
        historical.fields.listed_in_perk_styles = value(false, 'derived');
        const conflict = shard('conflict', [6]);
        conflict.fields.rune_page_slots = value([{style_id: '8000', slot_index: 6, slot_type: 'kStatMod'}], 'conflict');
        const unknown = shard('unknown', [6]);
        delete unknown.fields.listed_in_perk_styles;
        const wrongType = shard('wrong', [4]);
        wrongType.fields.rune_page_slots = value([{style_id: '8000', slot_index: 4, slot_type: 'kMixedRegularSplashable'}, null,
            {style_id: '8000', slot_index: -1, slot_type: 'kStatMod'}]);
        const records = [shard('5001', [6]), repeated, shard('5005', [4]), historical, conflict, unknown, wrongType, shard('other', [4], '8100')];
        expect(shardRows(records, '8000').map(row => row.map(r => r.id))).toEqual([['5008', '5005'], ['5008'], ['5001']]);
        expect(shardRows(records, 'unknown')).toEqual([]);
    });
});

describe('objets et recettes', () => {
    it('filtre la carte et les permissions exactes puis recherche sans accents ni casse', () => {
        const sword = {...item('1036'), name: 'Épée longue'};
        const potion = {...item('2003'), name: 'Potion de soin'};
        const blocked = ['maps', 'purchasable', 'in_store'].flatMap(key => [false, 'true', null].map(flag => {
            const r = item(`${key}:${flag}`);
            r.fields[key] = value(key === 'maps' ? {'11': flag} : flag);
            return r;
        }));
        const unresolved = item('conflict');
        unresolved.fields.in_store = value(true, 'conflict');
        const missing = item('missing');
        delete missing.fields.purchasable;
        const records = [sword, potion, ...blocked, unresolved, missing, {...item('rune'), kind: 'rune'}];
        expect(searchableItems(records, '  EPEE  ')).toEqual([sword]);
        expect(searchableItems(records, '').map(r => r.id)).toEqual(['1036', '2003']);
        expect(searchableItems(records, 'introuvable')).toEqual([]);
    });
    it('préserve les composants répétés et montre les références manquantes dans la bonne langue', () => {
        const component = item('1036');
        const english = {...component, locale: 'en_US', name: 'Long Sword'};
        const classic = {...component, namespace: 'classic'};
        const recipe = item('3134');
        recipe.fields.builds_from = value(['1036', '1036', '9999', 1001]);
        expect(itemComponents([english, classic, component, record('9999', 'rune')], recipe)).toEqual([
            {id: '1036', record: component}, {id: '1036', record: component}, {id: '9999', record: null}, {id: '1001', record: null},
        ]);
        expect(itemComponents([], item('empty'))).toEqual([]);
        recipe.fields.builds_from = value(['1036'], 'conflict');
        expect(itemComponents([component], recipe)).toEqual([]);
    });
    it('omet les statistiques non confirmées ou non finies sans inventer de zéro', () => {
        const r = item('1');
        r.stats = {damage: value(15, 'verified', 'attack_damage'), zero: value(0, 'derived'), missing: value(null, 'missing'),
            conflict: value(10, 'conflict'), description: value(5, 'descriptive'), string: value('5'), infinite: value(Infinity),
            nan: value(NaN), negative: value(-3, 'derived', 'percent')};
        expect(itemStats(r)).toEqual([{key: 'damage', value: 15, unit: 'attack_damage'}, {key: 'zero', value: 0, unit: null},
            {key: 'negative', value: -3, unit: 'percent'}]);
        expect(itemStats(item('empty'))).toEqual([]);
    });
});

it('ne remplace jamais un arbre équipé inconnu par le premier arbre du catalogue',()=>{
 const styles=[record('8000','rune'),record('8100','rune')];
 const page={primaryStyleId:0,subStyleId:9999,selectedPerkIds:[],isValid:false,isTemporary:false,autoModifiedSelections:[]};
 expect(displayedRuneStyles(styles,page)).toEqual(['','']);
 expect(displayedRuneStyles(styles,null)).toEqual(['8000','8100']);
 expect(displayedRuneStyles(styles,{...page,primaryStyleId:8000})).toEqual(['8000','']);
});
it('ne publie pas de variables de description non résolues',()=>{
 const r=record('9101','rune');
 expect(catalogDescription({...r,description:'Rend @HealAmount@ PV.'})).toBeNull();
 expect(catalogDescription({...r,description:'Rend {{ f1 }} PV.'})).toBeNull();
 expect(catalogDescription({...r,description:'Augmente de 30% la vitesse.'})).toBe('Augmente de 30% la vitesse.');
});

it('ne restaure pas un arbre exploré avant la création d’une page locale', () => {
    const explored: [string,string] = ['8300','8400'];
    expect(activeRuneExploration(explored,false,false)).toEqual(explored);
    expect(activeRuneExploration(explored,false,true)).toBeNull();
    expect(activeRuneExploration(explored,true,true)).toBeNull();
    // Terminer l’édition conserve les arbres de la page créée, pas ceux explorés avant.
    expect(activeRuneExploration(explored,true,false)).toBeNull();
});
