import { championSlug, type ChampionInfo } from "./champions";
import type { Platform } from "./filters";
import { parseRiotId, profilePath } from "./riotId";

/** Forme comparable : minuscules, sans accents ni ponctuation (Kai'Sa → kaisa). */
export function normalizeSearch(value: string): string {
  return value
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]/gu, "");
}

/** Champions dont le nom ou l'identifiant contient la saisie, débuts de nom d'abord. */
export function searchChampions(query: string, champions: ChampionInfo[], limit: number): ChampionInfo[] {
  const needle = normalizeSearch(query);
  if (needle.length === 0) return [];
  const scored: Array<{ champion: ChampionInfo; score: number }> = [];
  for (const champion of champions) {
    const name = normalizeSearch(champion.name);
    const id = normalizeSearch(champion.id);
    const score = name.startsWith(needle) || id.startsWith(needle) ? 0 : name.includes(needle) || id.includes(needle) ? 1 : -1;
    if (score >= 0) scored.push({ champion, score });
  }
  return scored
    .sort((a, b) => a.score - b.score || a.champion.name.localeCompare(b.champion.name))
    .slice(0, limit)
    .map((entry) => entry.champion);
}

/** Raccourci de la recherche globale : Ctrl+K sous Windows, Cmd+K sous macOS. */
export function isSearchShortcut(event: Pick<KeyboardEvent, "ctrlKey" | "metaKey" | "key">): boolean {
  return (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k";
}

/**
 * Page ouverte à la validation de la recherche : profil si la saisie est un Riot ID
 * complet (sur la plateforme choisie), sinon premier champion trouvé, sinon aucune.
 */
export function searchTarget(query: string, platform: Platform, results: ChampionInfo[], locale: string): string | null {
  const riotId = parseRiotId(query);
  if (riotId) return profilePath(locale, platform, riotId);
  const first = results[0];
  return first ? `/${locale}/champions/${championSlug(first)}` : null;
}
