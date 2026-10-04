/** Filtres de statistiques du site, alignés sur la validation de `services/api/src/query.rs`. */

/** Rôles affichés ; `UNKNOWN` reste une population technique, non proposée au joueur. */
export const ROLES = ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"] as const;

/** Rangs affichés par Riot ; `ALL` est une population distincte, jamais une somme. */
export const RANKS = [
  "ALL",
  "IRON",
  "BRONZE",
  "SILVER",
  "GOLD",
  "PLATINUM",
  "EMERALD",
  "DIAMOND",
  "MASTER",
  "GRANDMASTER",
  "CHALLENGER",
] as const;

/** Plateformes Riot acceptées par l'API (miroir de `olc_collector::config::PLATFORMS`). */
export const PLATFORMS = [
  "BR1",
  "EUN1",
  "EUW1",
  "JP1",
  "KR",
  "LA1",
  "LA2",
  "ME1",
  "NA1",
  "OC1",
  "RU",
  "SG2",
  "TR1",
  "TW2",
  "VN2",
] as const;

/** Files classées proposées : Solo/Duo (420) et Flex (440). */
export const QUEUES = [420, 440] as const;

export type SiteRole = (typeof ROLES)[number];
export type SiteRank = (typeof RANKS)[number];
export type Platform = (typeof PLATFORMS)[number];
export type Queue = (typeof QUEUES)[number];

export interface StatsFilters {
  /** Patch technique `16.19` ; `null` tant qu'aucune version statique n'est connue. */
  patch: string | null;
  platform: Platform;
  queue: Queue;
  role: SiteRole;
  rank: SiteRank;
}

export type SearchParams = Record<string, string | string[] | undefined>;

const DEFAULTS = { platform: "EUW1", queue: 420, role: "MIDDLE", rank: "ALL" } as const;
/** Limite haute de l'API : une page couvre tous les champions d'un rôle. */
export const STATS_PAGE_LIMIT = 200;

const PATCH = /^\d{1,3}\.\d{1,3}$/;

function first(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}

function pick<T extends string | number>(allowed: readonly T[], raw: string | undefined, fallback: T): T {
  if (raw === undefined) return fallback;
  return allowed.find((value) => String(value) === raw) ?? fallback;
}

export function isPlatform(value: string): value is Platform {
  return (PLATFORMS as readonly string[]).includes(value);
}

/** Lit les filtres d'une URL ; toute valeur invalide retombe sur sa valeur par défaut. */
export function parseStatsFilters(params: SearchParams, defaults: { patch: string | null }): StatsFilters {
  const patch = first(params.patch);
  return {
    patch: patch !== undefined && PATCH.test(patch) ? patch : defaults.patch,
    platform: pick(PLATFORMS, first(params.platform), DEFAULTS.platform),
    queue: pick(QUEUES, first(params.queue), DEFAULTS.queue),
    role: pick(ROLES, first(params.role), DEFAULTS.role),
    rank: pick(RANKS, first(params.rank), DEFAULTS.rank),
  };
}

/** Paramètres exacts de `/v1/tierlist` et `/v1/builds` (paramètres inconnus refusés par l'API). */
export function statsSearchParams(filters: StatsFilters & { patch: string }): URLSearchParams {
  return new URLSearchParams({
    patch: filters.patch,
    platform: filters.platform,
    queue: String(filters.queue),
    role: filters.role,
    rank: filters.rank,
    offset: "0",
    limit: String(STATS_PAGE_LIMIT),
  });
}

/** Lien de page qui conserve les filtres actifs, avec une modification éventuelle. */
export function filtersHref(path: string, filters: StatsFilters, change: Partial<StatsFilters> = {}): string {
  const next = { ...filters, ...change };
  const query = new URLSearchParams();
  if (next.patch !== null) query.set("patch", next.patch);
  query.set("platform", next.platform);
  query.set("queue", String(next.queue));
  query.set("role", next.role);
  query.set("rank", next.rank);
  return `${path}?${query.toString()}`;
}

/** Patches proposés au filtre : valeurs connues, dédoublonnées, du plus récent au plus ancien. */
export function patchOptions(current: Array<string | null>, published: string[][]): string[] {
  const unique = new Set<string>();
  for (const patch of [...current, ...published.flat()]) {
    if (patch !== null && PATCH.test(patch)) unique.add(patch);
  }
  const parts = (patch: string) => patch.split(".").map(Number) as [number, number];
  return [...unique].sort((a, b) => {
    const [aMajor, aMinor] = parts(a);
    const [bMajor, bMinor] = parts(b);
    return bMajor - aMajor || bMinor - aMinor;
  });
}
