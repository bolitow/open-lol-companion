/** Riot ID saisi par le joueur : seule entrée possible d'une recherche de profil. */
export interface RiotId {
  gameName: string;
  tagLine: string;
}

// Longueurs publiées par Riot : nom de 3 à 16 caractères, tag de 3 à 5 caractères alphanumériques.
const TAG = /^[\p{L}\p{N}]{3,5}$/u;

export function parseRiotId(input: string): RiotId | null {
  const parts = input.trim().split("#");
  if (parts.length !== 2) return null;
  const gameName = (parts[0] ?? "").trim();
  const tagLine = (parts[1] ?? "").trim();
  const nameLength = [...gameName].length;
  if (nameLength < 3 || nameLength > 16 || !TAG.test(tagLine)) return null;
  return { gameName, tagLine };
}

export function profilePath(locale: string, platform: string, id: RiotId): string {
  return `/${locale}/profile/${platform}/${encodeURIComponent(id.gameName)}/${encodeURIComponent(id.tagLine)}`;
}
