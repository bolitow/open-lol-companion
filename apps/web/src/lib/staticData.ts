/** Lecture défensive des documents Data Dragon servis par `/v1/static/{version}/{locale}/…`. */

export interface NamedEntry {
  name: string;
}

export interface SpellEntry extends NamedEntry {
  /** Identifiant d'image Data Dragon (`SummonerFlash`). */
  id: string;
}

export interface RuneEntry extends NamedEntry {
  /** Chemin relatif à `https://ddragon.leagueoflegends.com/cdn/img/`. */
  icon: string;
}

function dataEntries(document: unknown): Array<[string, Record<string, unknown>]> {
  const data = (document as { data?: unknown } | null)?.data;
  if (typeof data !== "object" || data === null) return [];
  return Object.entries(data).filter((entry): entry is [string, Record<string, unknown>] => typeof entry[1] === "object" && entry[1] !== null);
}

function positiveInteger(value: unknown): number | null {
  const number = Number(value);
  return Number.isInteger(number) && number > 0 ? number : null;
}

/** `item.json` : la clé de `data` est l'identifiant numérique de l'objet. */
export function indexItems(document: unknown): Map<number, NamedEntry> {
  const index = new Map<number, NamedEntry>();
  for (const [key, entry] of dataEntries(document)) {
    const id = positiveInteger(key);
    if (id !== null && typeof entry.name === "string") index.set(id, { name: entry.name });
  }
  return index;
}

/** `summoner.json` : `key` porte l'identifiant numérique utilisé par match-v5. */
export function indexSpells(document: unknown): Map<number, SpellEntry> {
  const index = new Map<number, SpellEntry>();
  for (const [, entry] of dataEntries(document)) {
    const key = positiveInteger(entry.key);
    if (key !== null && typeof entry.id === "string" && typeof entry.name === "string") {
      index.set(key, { id: entry.id, name: entry.name });
    }
  }
  return index;
}

function runeEntry(value: unknown): [number, RuneEntry] | null {
  const entry = value as Record<string, unknown> | null;
  const id = positiveInteger(entry?.id);
  if (id === null || typeof entry?.name !== "string" || typeof entry.icon !== "string") return null;
  return [id, { name: entry.name, icon: entry.icon }];
}

/** `runesReforged.json` : arbres puis runes de chaque emplacement ; les fragments n'y figurent pas. */
export function indexRunes(document: unknown): Map<number, RuneEntry> {
  const index = new Map<number, RuneEntry>();
  if (!Array.isArray(document)) return index;
  for (const style of document) {
    const styleEntry = runeEntry(style);
    if (styleEntry === null) continue;
    index.set(...styleEntry);
    const slots = (style as { slots?: unknown }).slots;
    if (!Array.isArray(slots)) continue;
    for (const slot of slots) {
      const runes = (slot as { runes?: unknown } | null)?.runes;
      if (!Array.isArray(runes)) continue;
      for (const rune of runes) {
        const runeIndexEntry = runeEntry(rune);
        if (runeIndexEntry !== null) index.set(...runeIndexEntry);
      }
    }
  }
  return index;
}
