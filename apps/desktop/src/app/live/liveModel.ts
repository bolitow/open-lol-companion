import type {BuildRequest, LiveSession} from '@olc/shared';
import championIndex from '../../../public/game-data/champions.json';
import type {Locale} from '../state';

export function liveChampion(session: LiveSession | null, locale: Locale): {id: number; name: string; image: string} | null {
  if (session?.status !== 'ready' || !session.game) return null;
  const key = session.game.player.championKey.replace(/^game_character_displayname_/, '');
  const champion = Object.entries(championIndex).find(([, entry]) => entry.key === key);
  return champion ? {id: Number(champion[0]), name: champion[1][locale], image: `/game-data/champions/${champion[0]}.jpg`} : null;
}

/** Le contexte provient de la draft réelle ; aucun poste ni file par défaut. */
export type LiveBuildContext = Omit<BuildRequest, 'patch'>;

export function liveBuildContext(session: LiveSession | null): LiveBuildContext | null {
  const champion = liveChampion(session, 'en');
  const context = session?.context;
  if (!champion || !context || context.championId !== champion.id
    || session?.game?.mapNumber !== 11 || session.game.gameMode !== 'CLASSIC'
    || !context.role || context.role === 'UNKNOWN' || !context.platform?.trim()
    || (!context.customGame && (context.queue === null || !Number.isSafeInteger(context.queue) || context.queue <= 0))) return null;
  return {
    champion_id: champion.id,
    platform: context.platform,
    queue: context.customGame ? 420 : context.queue!,
    role: context.role,
    rank: 'ALL',
  };
}

export function liveBuildRequest(session: LiveSession | null, version: string): BuildRequest | null {
  const context = liveBuildContext(session);
  return context && /^\d+\.\d+\.\d+$/.test(version)
    ? {...context, patch: version.split('.').slice(0, 2).join('.')}
    : null;
}

export function formatLiveTime(seconds: number, locale: Locale = 'en'): string {
  if (!Number.isFinite(seconds) || seconds < 0) return '—';
  const minutes = new Intl.NumberFormat(locale, {useGrouping: false}).format(Math.floor(seconds / 60));
  const remainder = new Intl.NumberFormat(locale, {minimumIntegerDigits: 2}).format(Math.floor(seconds % 60));
  return `${minutes}:${remainder}`;
}
