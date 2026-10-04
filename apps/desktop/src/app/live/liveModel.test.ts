import {describe, expect, it} from 'vitest';
import type {LiveContext, LiveSession} from '@olc/shared';
import {formatLiveTime, liveBuildRequest, liveChampion} from './liveModel';

const ready: LiveSession = {
  revision: 2, generation: 1, status: 'ready',
  context: {championId: 432, role: 'UTILITY', platform: 'EUW1', queue: 420, customGame: false},
  game: {gameTime: 125.8, gameMode: 'CLASSIC', mapNumber: 11, player: {championKey: 'Bard', level: 3, kills: 0, deaths: 1, assists: 2, creepScore: 4, items: [], currentGold: null, wardScore: null, isDead: null, respawnTimer: null, abilityLevels: null, team: null, position: null}, teams: null, events: []}, postgame: null,
};

describe('contexte réel en partie', () => {
  it('associe le champion live au catalogue avant de demander les statistiques', () => {
    expect(liveChampion(ready, 'en')).toEqual({id: 432, name: 'Bard', image: '/game-data/champions/432.jpg'});
    expect(liveBuildRequest(ready, '16.19.1')).toEqual({champion_id: 432, patch: '16.19', platform: 'EUW1', queue: 420, role: 'UTILITY', rank: 'EMERALD_PLUS'});
  });

  it('reconnaît la clé publique brute du champion', () => {
    const game = {...ready.game!, player: {...ready.game!.player, championKey: 'game_character_displayname_Bard'}};
    expect(liveChampion({...ready, game}, 'fr')?.id).toBe(432);
  });

  it.each<Partial<LiveContext>>([{role: null}, {role: 'UNKNOWN'}, {platform: null}, {queue: null}, {championId: 103}])('ne complète pas un contexte absent ou incohérent : %j', patch => {
    expect(liveBuildRequest({...ready, context: {...ready.context!, ...patch}}, '16.19.1')).toBeNull();
  });

  it('ne conserve aucune statistique hors partie prête ou hors Faille', () => {
    for (const status of ['idle', 'waiting', 'unavailable', 'invalid'] as const) {
      expect(liveBuildRequest({...ready, status}, '16.19.1')).toBeNull();
      expect(liveChampion({...ready, status}, 'fr')).toBeNull();
    }
    expect(liveBuildRequest({...ready, game: {...ready.game!, mapNumber: 12}}, '16.19.1')).toBeNull();
    expect(liveBuildRequest({...ready, game: {...ready.game!, gameMode: 'ARAM'}}, '16.19.1')).toBeNull();
    expect(liveBuildRequest({...ready, context: null}, '16.19.1')).toBeNull();
    expect(liveBuildRequest(ready, 'invalid')).toBeNull();
  });

  it('utilise la population Solo/Duo explicite d’une personnalisée', () => {
    const custom = {...ready, context: {...ready.context!, queue: 3100, customGame: true}};
    expect(liveBuildRequest(custom, '16.19.1')?.queue).toBe(420);
    for (const queue of [0, null]) {
      expect(liveBuildRequest({...custom, context: {...custom.context, queue}}, '16.19.1')?.queue).toBe(420);
    }
  });

  it('ne remplace pas une file réelle par Solo/Duo', () => {
    expect(liveBuildRequest({...ready, context: {...ready.context!, queue: 440}}, '16.19.1')?.queue).toBe(440);
  });

  it('ne devine pas un champion absent du catalogue', () => {
    const unknown = {...ready, game: {...ready.game!, player: {...ready.game!.player, championKey: 'UnknownChampion'}}};
    expect(liveChampion(unknown, 'fr')).toBeNull();
    expect(liveBuildRequest(unknown, '16.19.1')).toBeNull();
  });

  it('formate le temps écoulé sans arrondir la seconde suivante', () => {
    expect(formatLiveTime(125.8)).toBe('2:05');
    expect(formatLiveTime(3661)).toBe('61:01');
    expect(formatLiveTime(Number.NaN)).toBe('—');
  });
});

it('respecte le rang explicite et le neutralise hors classé',()=>{
 expect(liveBuildRequest(ready,'16.19.1','GOLD')?.rank).toBe('GOLD');
 expect(liveBuildRequest({...ready,context:{...ready.context!,queue:400}},'16.19.1','GOLD')?.rank).toBe('ALL');
});
