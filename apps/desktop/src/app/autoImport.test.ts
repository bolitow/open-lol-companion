import {describe, expect, it, vi} from 'vitest';
import type {AutoImportReceipt, AutoImportRequest, BuildReport, BuildStats, LcuSession} from '@olc/shared';
import catalog from '../../public/game-data/catalog/en_US.json';
import {autoImportTarget, chooseAutoImports, createAutoImportController, parseAutoImportPreferences, type AutoImportTarget} from './autoImport';
import type {PreparationCatalog} from './catalog';
const data = catalog as unknown as PreparationCatalog;
const session: LcuSession = {revision: 1, draftId:'draft-1', connected: true, phase: 'ChampSelect', runePage: null,
    account: {game_name: 'Player', tag_line: 'EUW', platform: 'EUW1'},
    draft: {supported: true, gameId: '123', queueId: 420, allySide: 'blue', allies: [{cellId: 0, championId: 432, locked: true, local: true, position: 'utility', acting: false}], enemies: [], allyBans: [], enemyBans: [], timer: null, localSpells: [4, 14]}};
const target = (): AutoImportTarget => autoImportTarget(session, data, 'en')!;
const variant = (change: Partial<BuildStats> = {}): BuildStats => ({...target().request, platform_id:'EUW1', queue_id:420, category:'final_items',selection:[1001],games:50,wins:40,population:80,performance_available:true,pick_rate:null,win_rate:null,...change});
const report = (builds = [variant()]): BuildReport => ({request:target().request,meta:{min_games:100,source_snapshot_at:'2026-10-02T08:00:00Z',published_at:'2026-10-02T08:01:00Z'},builds});
const preferences = {runes:true,items:true,minGames:1};
function deferred<T>() {let resolve!:(value:T)=>void; const promise=new Promise<T>(r=>{resolve=r});return {promise,resolve};}
const flush = async () => {for(let i=0;i<8;i++)await Promise.resolve();};

describe('imports au prépick',()=>{
    it('suit uniquement le champion local sélectionné, son poste et sa vraie file',()=>{
        expect(target()).toMatchObject({context:{gameId:'123',championId:432,role:'UTILITY',queueId:420},request:{champion_id:432,role:'UTILITY',platform:'EUW1',queue:420,rank:'ALL'}});
        for(const patch of [{connected:false},{phase:'InProgress' as const},{account:null},{draftId:undefined},{draft:{...session.draft!,queueId:undefined}},{draft:{...session.draft!,allies:[{...session.draft!.allies[0]!,position:null}]}}])expect(autoImportTarget({...session,...patch},data,'en')).toBeNull();
    });
    it('accepte le prépick avant gameId et conserve le même contexte au verrouillage',()=>{
        const pre={...session,draft:{...session.draft!,gameId:undefined,allies:[{...session.draft!.allies[0]!,locked:false}]}};
        const before=autoImportTarget(pre,data,'en');
        expect(before).toMatchObject({context:{draftId:'draft-1',championId:432,role:'UTILITY',queueId:420}});
        expect(autoImportTarget({...pre,draft:{...pre.draft,allies:[{...pre.draft.allies[0]!,locked:true}]}},data,'en')).toEqual(before);
    });
    it('autorise la personnalisée identifiée avec poste explicite et données Solo/Duo',()=>{
        const custom={...session,draft:{...session.draft!,customGame:true,queueId:3100,gameId:undefined,allies:[{...session.draft!.allies[0]!,locked:false,position:null}]}};
        expect(autoImportTarget(custom,data,'fr')).toBeNull();
        expect(autoImportTarget(custom,data,'fr','UTILITY')).toMatchObject({context:{draftId:'draft-1',championId:432,queueId:3100,role:'UTILITY'},request:{queue:420,role:'UTILITY'}});
        expect(autoImportTarget({...custom,draft:{...custom.draft,customGame:false}},data,'fr','UTILITY')).toBeNull();
    });
    it('active zéro import par défaut et adapte uniquement le seuil au développement',()=>{
        expect(parseAutoImportPreferences(null,true)).toEqual({...preferences,runes:false,items:false});
        expect(parseAutoImportPreferences(null,false).minGames).toBe(100);
        expect(parseAutoImportPreferences('{"runes":true,"items":true,"minGames":0}',false)).toEqual({runes:true,items:true,minGames:100});
        expect(parseAutoImportPreferences('{"customRole":"UTILITY"}',true).customRole).toBe('UTILITY');
        expect(parseAutoImportPreferences('{"customRole":"UNKNOWN"}',true).customRole).toBeUndefined();
    });
    it('choisit la variante valide la plus jouée, même sous le seuil de publication',()=>{
        const choices=chooseAutoImports(report([variant({games:10,wins:10,selection:[1004]}),variant(),variant({games:60,selection:[999999]})]),target(),1,data.records);
        expect(choices.items?.selection).toMatchObject({kind:'items',request:{blocks:[{label:'Observed final inventory',items:[{id:1001,count:1}]}]}});
        expect(choices.items?.metrics).toEqual({games:50,wins:40,observedWinRate:80,lowSample:true});
        expect(chooseAutoImports(report(),target(),100,data.records).items).toBeNull();
        expect(choices.runes).toBeNull();
    });
    it('importe une vraie page complète et ignore entièrement les statistiques de sorts',()=>{
        const runes=variant({category:'runes',games:4,wins:3,selection:[8100,8112,8126,8141,8135,8300,8304,8316,5005,5010,5001]});
        const choices=chooseAutoImports(report([runes,variant(),variant({category:'summoner_spells',selection:[4,14],games:1000})]),target(),1,data.records);
        expect(choices.runes?.selection).toMatchObject({kind:'runes',request:{primaryStyleId:8100,subStyleId:8300,selectedPerkIds:[8112,8126,8141,8135,8304,8316,5005,5010,5001]}});
        expect(choices.runes?.metrics).toMatchObject({games:4,wins:3,observedWinRate:75,lowSample:true});
        expect(Object.keys(choices)).toEqual(['runes','items']);
    });
    it('ne mélange ni les populations ni les performances manquantes',()=>{
        expect(chooseAutoImports(report([variant({role:'TOP'})]),target(),1,data.records).items).toBeNull();
        expect(chooseAutoImports(report([variant({performance_available:false,wins:null})]),target(),1,data.records).items?.metrics.observedWinRate).toBeNull();
    });
    it('importe une fois, indépendamment des runes absentes et des événements répétés',async()=>{
        const send=vi.fn(async(_request:AutoImportRequest)=>({confirmed:true}));
        const prepare=vi.fn(async()=>chooseAutoImports(report(),target(),1,data.records));
        const controller=createAutoImportController({prepare,send});
        controller.update(target(),preferences); await flush();
        controller.update(target(),preferences); await flush();
        controller.update(null,preferences); controller.update(target(),preferences); await flush();
        expect(send).toHaveBeenCalledTimes(1);
        expect(send.mock.calls[0]?.[0]).toMatchObject({context:target().context,selection:{kind:'items'}});
        expect(controller.getSnapshot().items.status).toBe('confirmed');
        expect(controller.getSnapshot().runes.status).toBe('empty');
    });
    it('réimporte après un échange aller-retour, mais attend un envoi précédent encore en vol',async()=>{
        const pending=deferred<AutoImportReceipt>();
        const send=vi.fn().mockImplementationOnce(()=>pending.promise).mockResolvedValue({confirmed:true});
        const other={...target(),context:{...target().context,championId:103},request:{...target().request,champion_id:103}};
        const controller=createAutoImportController({prepare:async(t)=>({runes:null,items:{...chooseAutoImports(report(),target(),1,data.records).items!,selection:{kind:'items',request:{championId:t.context.championId,championName:'Test',mapId:11,blocks:[]}}}}),send});
        controller.update(target(),preferences);await flush();
        controller.update(other,preferences);await flush();
        expect(send).toHaveBeenCalledTimes(1);
        pending.resolve({confirmed:true});await flush();
        expect(send.mock.calls.map(call=>call[0].context.championId)).toEqual([432,103]);
        controller.update(target(),preferences);await flush();
        expect(send.mock.calls.map(call=>call[0].context.championId)).toEqual([432,103,432]);
    });
    it('le passage prépick vers verrouillage ne renvoie pas un second import',async()=>{
        const sent:AutoImportRequest[]=[];
        const controller=createAutoImportController({prepare:async()=>chooseAutoImports(report(),target(),1,data.records),send:async request=>{sent.push(request);return {confirmed:true};}});
        const pre={...session,draft:{...session.draft!,gameId:undefined,allies:[{...session.draft!.allies[0]!,locked:false}]}};
        controller.update(autoImportTarget(pre,data,'en'),preferences);await flush();
        controller.update(autoImportTarget(session,data,'en'),preferences);await flush();
        expect(sent).toHaveLength(1);
    });
    it('annule une réponse de statistiques devenue ancienne ou un import désactivé',async()=>{
        for(const disable of [true,false]){
            const pending=deferred<ReturnType<typeof chooseAutoImports>>();const send=vi.fn(async(_request:AutoImportRequest)=>({confirmed:true}));
            const controller=createAutoImportController({prepare:()=>pending.promise,send});
            controller.update(target(),preferences);
            controller.update(disable?target():null,disable?{...preferences,items:false,runes:false}:preferences);
            pending.resolve(chooseAutoImports(report(),target(),1,data.records)); await flush();
            expect(send).not.toHaveBeenCalled();
        }
    });
    it('ne répète pas un import en vol après une lecture absente ni après confirmation incertaine',async()=>{
        const pending=deferred<AutoImportReceipt>();const send=vi.fn(()=>pending.promise);
        const controller=createAutoImportController({prepare:async()=>chooseAutoImports(report(),target(),1,data.records),send});
        controller.update(target(),preferences); await flush();
        controller.update(null,preferences);controller.update(target(),preferences);await flush();
        expect(send).toHaveBeenCalledTimes(1);
        pending.resolve({confirmed:false});await flush();controller.retry();await flush();
        expect(send).toHaveBeenCalledTimes(1);expect(controller.getSnapshot().items.status).toBe('accepted');
    });
    it('laisse les erreurs visibles sans boucle et permet une reprise explicite',async()=>{
        const send=vi.fn().mockRejectedValueOnce('clientRejected').mockResolvedValue({confirmed:true});
        const controller=createAutoImportController({prepare:async()=>chooseAutoImports(report(),target(),1,data.records),send});
        controller.update(target(),preferences);await flush();controller.update(target(),preferences);await flush();
        expect(send).toHaveBeenCalledTimes(1);expect(controller.getSnapshot().items.status).toBe('error');
        controller.retry();await flush();expect(send).toHaveBeenCalledTimes(2);expect(controller.getSnapshot().items.status).toBe('confirmed');
    });
});
