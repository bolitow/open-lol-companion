import type { TierlistResponse } from "@olc/shared";

/** Sous-ensemble utilisé de `champion.json` (Data Dragon, servi par `/v1/static`). */
export interface ChampionInfo {
  key: number;
  /** Identifiant Data Dragon (`MonkeyKing`), utilisé pour les images. */
  id: string;
  name: string;
  title: string;
}

export type ChampionIndex = Map<number, ChampionInfo>;

/** Indexe le document par identifiant numérique ; les fiches incomplètes sont ignorées. */
export function indexChampions(document: unknown): ChampionIndex {
  const index: ChampionIndex = new Map();
  const data = (document as { data?: unknown } | null)?.data;
  if (typeof data !== "object" || data === null) return index;
  for (const raw of Object.values(data)) {
    const entry = raw as Record<string, unknown>;
    const key = Number(entry.key);
    if (!Number.isInteger(key) || key <= 0 || typeof entry.id !== "string" || typeof entry.name !== "string") {
      continue;
    }
    index.set(key, {
      key,
      id: entry.id,
      name: entry.name,
      title: typeof entry.title === "string" ? entry.title : "",
    });
  }
  return index;
}

export interface TierlistRow {
  championId: number;
  /** `null` si le champion manque au catalogue statique (nouveau champion, patch décalé). */
  name: string | null;
  iconId: string | null;
  tier: string | null;
  position: number | null;
  games: number;
  winRate: number | null;
  pickRate: number | null;
  banRate: number | null;
}

/** Lignes de la tierlist, dans l'ordre publié par l'API (position puis champion). */
export function tierlistRows(response: Pick<TierlistResponse, "entries" | "bans">, champions: ChampionIndex): TierlistRow[] {
  const bans = new Map(response.bans.map((ban) => [ban.champion_id, ban.ban_rate]));
  return response.entries.map((entry) => {
    const champion = champions.get(entry.champion_id);
    return {
      championId: entry.champion_id,
      name: champion?.name ?? null,
      iconId: champion?.id ?? null,
      tier: entry.tier,
      position: entry.position,
      games: entry.games,
      winRate: entry.win_rate,
      pickRate: entry.pick_rate,
      banRate: bans.get(entry.champion_id) ?? null,
    };
  });
}

/** Segment d'URL d'une page champion : identifiant Data Dragon en minuscules (`monkeyking`). */
export function championSlug(champion: Pick<ChampionInfo, "id">): string {
  return champion.id.toLowerCase();
}

export function championBySlug(index: ChampionIndex, slug: string): ChampionInfo | undefined {
  const wanted = slug.toLowerCase();
  for (const champion of index.values()) {
    if (championSlug(champion) === wanted) return champion;
  }
  return undefined;
}
