import { DDRAGON_BASE } from "@olc/shared";

/** Images officielles Data Dragon, servies par le CDN de Riot. */
export function spellIconUrl(version: string, spellId: string): string {
  return `${DDRAGON_BASE}/cdn/${version}/img/spell/${spellId}.png`;
}

export function profileIconUrl(version: string, iconId: number): string {
  return `${DDRAGON_BASE}/cdn/${version}/img/profileicon/${iconId}.png`;
}

/** Les chemins d'icônes de `runesReforged.json` ne sont pas versionnés. */
export function runeIconUrl(icon: string): string {
  return `${DDRAGON_BASE}/cdn/img/${icon}`;
}
