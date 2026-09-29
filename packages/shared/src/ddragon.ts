/** Data Dragon : données statiques officielles de Riot (champions, items, runes) par patch. */
export const DDRAGON_BASE = "https://ddragon.leagueoflegends.com";

export function championIconUrl(version: string, championId: string): string {
  return `${DDRAGON_BASE}/cdn/${version}/img/champion/${championId}.png`;
}

export function itemIconUrl(version: string, itemId: number): string {
  return `${DDRAGON_BASE}/cdn/${version}/img/item/${itemId}.png`;
}

/** Patch « 16.19.1 » → patch de jeu affiché « 16.19 ». */
export function displayPatch(version: string): string {
  return version.split(".").slice(0, 2).join(".");
}
