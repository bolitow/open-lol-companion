import type { ProfileRank } from "@olc/shared";
import { interpolate, type Dictionary } from "./i18n";
import { parseRiotId, type RiotId } from "./riotId";

/** Borne de l'API pour le début de l'historique (`services/api`, profils). */
const MAX_START = 10_000;
/** Paliers sans division affichée par le client. */
const APEX = new Set(["MASTER", "GRANDMASTER", "CHALLENGER"]);

export function parseHistoryStart(value: string | string[] | undefined): number {
  const raw = Array.isArray(value) ? value[0] : value;
  if (raw === undefined || !/^\d+$/.test(raw)) return 0;
  const start = Number(raw);
  return start <= MAX_START ? start : 0;
}

/** Segments d'URL du profil : décodés si besoin, puis validés comme une saisie. */
export function riotIdFromSegments(gameName: string, tagLine: string): RiotId | null {
  try {
    return parseRiotId(`${decodeURIComponent(gameName)}#${decodeURIComponent(tagLine)}`);
  } catch {
    return null;
  }
}

/** Rang tel que Riot le publie (league-v4) ; aucun rang n'est déduit pour un non classé. */
export function rankLabel(rank: ProfileRank, t: Dictionary): string {
  if (rank.status === "unranked" || rank.tier === null) return t.profile.unranked;
  const tiers: Record<string, string> = t.filters.ranks;
  const parts = [tiers[rank.tier] ?? rank.tier];
  if (rank.division !== null && !APEX.has(rank.tier)) parts.push(rank.division);
  const label = parts.join(" ");
  return rank.league_points === null ? label : `${label} · ${interpolate(t.profile.leaguePoints, { lp: String(rank.league_points) })}`;
}
