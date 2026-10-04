import type {AutoImportContext, AutoImportReceipt, AutoImportRequest, AutoImportSelection, BuildReport, BuildRequest, BuildStats, CatalogRecord, FlashSlot, LcuSession, Role} from '@olc/shared';
import {buildRequestKey, runePageFromBuild, variantsFor} from './buildModel';
import {validSpellPair} from './spellEditing';
import {itemAdjustments,itemSetPlan,type ItemAdjustments} from './itemImport';
import {championDetails} from './draft';
import type {PreparationCatalog} from './catalog';
import type {Locale} from './state';

export interface AutoImportPreferences {runes: boolean; items: boolean; spells: boolean; minGames: number; customRole?: Role}
export function parseAutoImportPreferences(raw: string | null, development: boolean): AutoImportPreferences {
    const defaults = {runes: false, items: false, spells: false, minGames: development ? 1 : 100};
    try {
        const value: unknown = JSON.parse(raw ?? 'null');
        if (!value || typeof value !== 'object') return defaults;
        const v = value as Record<string, unknown>;
        return {...(typeof v.customRole === 'string' && ['TOP','JUNGLE','MIDDLE','BOTTOM','UTILITY'].includes(v.customRole) ? {customRole:v.customRole as Role} : {}),runes: v.runes === true, items: v.items === true, spells: v.spells === true,
            minGames: typeof v.minGames === 'number' && Number.isInteger(v.minGames) && v.minGames >= 1 && v.minGames <= 1000 ? v.minGames : defaults.minGames};
    } catch {return defaults;}
}
export interface AutoImportTarget {context: AutoImportContext; request: BuildRequest; championName: string; itemLabel: string}
export function autoImportTarget(session: LcuSession, catalog: PreparationCatalog, locale: Locale, customRole?: Role): AutoImportTarget | null {
    const draft = session.draft;
    if (!session.connected || session.phase !== 'ChampSelect' || !session.account || !draft?.supported
        || !session.draftId || draft.queueId === undefined || (!draft.customGame && ![400,420,440].includes(draft.queueId))) return null;
    const locals = draft.allies.filter(player => player.local), player = locals[0];
    if (locals.length !== 1 || !player?.championId) return null;
    const role = (player.position?.toUpperCase() ?? (draft.customGame ? customRole : undefined)) as Role | undefined;
    if (!role || !['TOP','JUNGLE','MIDDLE','BOTTOM','UTILITY'].includes(role)) return null;
    const champion = championDetails(player.championId, locale);
    if (!champion) return null;
    return {context: {draftId: session.draftId, gameId: draft.gameId, championId: player.championId, queueId: draft.queueId, role},
        request: {champion_id: player.championId, patch: catalog.version.split('.').slice(0,2).join('.'), platform: session.account.platform, queue: draft.customGame ? 420 : draft.queueId, role, rank: 'ALL'},
        championName: champion.name, itemLabel: locale === 'fr' ? 'Inventaire final observé' : 'Observed final inventory'};
}
export interface ObservedMetrics {games: number; wins: number | null; observedWinRate: number | null; lowSample: boolean}
/** `adjustments` : objets du set remplacés ou retirés faute d'être achetables (#88), propre à la catégorie objets. */
export interface AutoImportCandidate {selection: AutoImportSelection; metrics: ObservedMetrics; adjustments?: ItemAdjustments}
export type AutoImportCandidates = Record<'runes' | 'items', AutoImportCandidate | null> & {spells: AutoImportCandidate | 'flashPreferenceRequired' | null};
/** Les catégories restent indépendantes ; le tri porte sur l'effectif, jamais sur le taux. */
export function chooseAutoImports(report: BuildReport, target: AutoImportTarget, minGames: number, records: readonly CatalogRecord[], flashSlot: FlashSlot | null = null): AutoImportCandidates {
    const empty = {runes: null, items: null, spells: null};
    if (!Number.isInteger(minGames) || minGames < 1 || minGames > 1000 || buildRequestKey(report.request) !== buildRequestKey(target.request)) return empty;
    const eligible = (variant: BuildStats) => Number.isSafeInteger(variant.games) && variant.games >= minGames
        && variant.champion_id === target.request.champion_id && variant.role === target.request.role
        && variant.patch === target.request.patch && variant.platform_id === target.request.platform
        && variant.queue_id === target.request.queue && variant.rank === target.request.rank;
    const metrics = (variant: BuildStats): ObservedMetrics => {
        const wins = variant.performance_available && variant.wins !== null && Number.isSafeInteger(variant.wins) && variant.wins >= 0 && variant.wins <= variant.games ? variant.wins : null;
        return {games: variant.games, wins, observedWinRate: wins === null ? null : 100 * wins / variant.games, lowSample: variant.games < report.meta.min_games};
    };
    let runes: AutoImportCandidate | null = null, items: AutoImportCandidate | null = null;
    for (const variant of variantsFor(report, 'runes').filter(eligible)) {
        const page = runePageFromBuild(variant.selection, records);
        if (page) {runes = {selection: {kind: 'runes', request: {championName: target.championName, primaryStyleId: page.primaryStyleId, subStyleId: page.subStyleId, selectedPerkIds: page.selectedPerkIds}}, metrics: metrics(variant)}; break;}
    }
    for (const variant of variantsFor(report, 'final_items').filter(eligible)) {
        const plan = itemSetPlan(target.context.championId, target.championName, target.itemLabel, variant.selection, records);
        if (plan) {items = {selection: {kind: 'items', request: plan.request}, metrics: metrics(variant), adjustments: itemAdjustments(plan)}; break;}
    }
    const variant = variantsFor(report, 'summoner_spells').find(v=>eligible(v) && validSpellPair(v.selection,records));
    let spells: AutoImportCandidates['spells'] = null;
    if (variant && validSpellPair(variant.selection,records)) {
        spells = variant.selection.includes(4) && flashSlot === null ? 'flashPreferenceRequired'
            : {selection:{kind:'spells',request:{spellIds:variant.selection,flashSlot:flashSlot??'D'}},metrics:metrics(variant)};
    }
    return {runes, items, spells};
}
export type AutoImportKind = AutoImportSelection['kind'];
export interface AutoImportProgress {status: 'waiting' | 'disabled' | 'loading' | 'importing' | 'empty' | 'confirmed' | 'accepted' | 'error' | 'needsFlash'; metrics?: ObservedMetrics; adjustments?: ItemAdjustments; error?: unknown}
export interface AutoImportSnapshot {runes: AutoImportProgress; items: AutoImportProgress; spells: AutoImportProgress}
interface Dependencies {
    prepare: (target: AutoImportTarget, minGames: number, flashSlot: FlashSlot | null) => Promise<AutoImportCandidates>;
    send: (request: AutoImportRequest) => Promise<AutoImportReceipt>;
}
const kinds: readonly AutoImportKind[] = ['runes', 'items', 'spells'];
const targetKey = (target: AutoImportTarget) => JSON.stringify([target.context.draftId, target.request.platform, target.context.championId, target.context.role, target.context.queueId]);
/** Un envoi par draft/champion/poste/catégorie, même après une lecture LCU manquante.
 * Une modification manuelle ultérieure dans LoL n'entraîne jamais de nouvel import. */
export function createAutoImportController(dependencies: Dependencies) {
    let snapshot: AutoImportSnapshot = {runes: {status:'disabled'}, items:{status:'disabled'}, spells:{status:'disabled'}};
    let target: AutoImportTarget | null = null, preferences: AutoImportPreferences = {runes:false,items:false,spells:false,minGames:100};
    let flashPreference: FlashSlot | null = null;
    let fingerprint = '', generation = 0, contextEpoch = 0, lastContext: string | null = null;
    const inFlight = new Set<AutoImportKind>();
    const ledger = new Map<string, AutoImportProgress>(), listeners = new Set<()=>void>();
    const publish = (next: AutoImportSnapshot) => {snapshot = next; listeners.forEach(fn=>fn());};
    const slotKey = (key: string, kind: AutoImportKind) => `${key}:${kind}`;
    async function run(current: AutoImportTarget | null, settings: AutoImportPreferences, flashSlot: FlashSlot | null) {
        const started = ++generation;
        const key = current ? JSON.stringify([contextEpoch,targetKey(current)]) : null;
        const progress = (kind: AutoImportKind): AutoImportProgress => !settings[kind] ? {status:'disabled'} : !key ? {status:'waiting'} : ledger.get(slotKey(key,kind)) ?? {status:'loading'};
        publish({runes:progress('runes'),items:progress('items'),spells:progress('spells')});
        if (!current || !key || kinds.every(kind=>!settings[kind] || ledger.has(slotKey(key,kind)))) return;
        let candidates: AutoImportCandidates;
        try {candidates = await dependencies.prepare(current, settings.minGames, flashSlot);}
        catch (error) {
            if (started === generation) publish(Object.fromEntries(kinds.map(kind=>[kind,settings[kind] && !ledger.has(slotKey(key,kind)) ? {status:'error',error} : snapshot[kind]])) as unknown as AutoImportSnapshot);
            return;
        }
        if (started !== generation) return;
        // Chaque catégorie aboutit séparément : l'absence de runes ne bloque pas les autres imports.
        await Promise.all(kinds.map(async kind => {
            if (!settings[kind] || ledger.has(slotKey(key,kind)) || inFlight.has(kind) || started !== generation) return;
            const candidate = candidates[kind];
            if (candidate === 'flashPreferenceRequired') {publish({...snapshot,[kind]:{status:'needsFlash'}}); return;}
            if (!candidate) {publish({...snapshot,[kind]:{status:'empty'}}); return;}
            const entryKey = slotKey(key,kind);
            const detail = {metrics:candidate.metrics,...(candidate.adjustments?{adjustments:candidate.adjustments}:{})};
            const importing: AutoImportProgress = {status:'importing',...detail};
            inFlight.add(kind); ledger.set(entryKey, importing); publish({...snapshot,[kind]:importing});
            let result: AutoImportProgress;
            try {const receipt = await dependencies.send({context:current.context,selection:candidate.selection});
                result = {status:receipt.confirmed?'confirmed':'accepted',...detail};}
            catch (error) {result = {status:'error',error,...detail};}
            inFlight.delete(kind);
            const stillSameContext = key === JSON.stringify([contextEpoch,lastContext]);
            if (stillSameContext) ledger.set(entryKey,result);
            if (stillSameContext && target && preferences[kind]) publish({...snapshot,[kind]:result});
            // Un échange intervenu pendant l'envoi attend sa fin avant de traiter le nouveau pick.
            if (started !== generation && target) void run(target,preferences,flashPreference);
        }));
    }
    const update = (next: AutoImportTarget | null, settings: AutoImportPreferences, flashSlot: FlashSlot | null = null) => {
        if (next && targetKey(next) !== lastContext) {lastContext=targetKey(next);contextEpoch++;ledger.clear();}
        target = next; preferences = settings; flashPreference = flashSlot;
        const nextFingerprint = JSON.stringify([next && targetKey(next), next?.request.patch, settings.runes, settings.items, settings.spells, settings.minGames, flashSlot]);
        if (nextFingerprint === fingerprint) return;
        fingerprint = nextFingerprint; void run(next,settings,flashSlot);
    };
    return {getSnapshot:()=>snapshot, subscribe:(fn:()=>void)=>{listeners.add(fn);return()=>{listeners.delete(fn);};}, update,
        suspend:()=>{generation++;fingerprint='';target=null;},
        retry:()=>{
            if (!target) return;
            const key = JSON.stringify([contextEpoch,targetKey(target)]);
            for (const kind of kinds) if (ledger.get(slotKey(key,kind))?.status === 'error') ledger.delete(slotKey(key,kind));
            fingerprint='';update(target,preferences,flashPreference);
        }};
}
