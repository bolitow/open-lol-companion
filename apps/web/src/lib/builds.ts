import type { BuildStats } from "@olc/shared";

/** Sélection `runes` du collecteur : style principal + 4, style secondaire + 2, 3 fragments. */
export interface RuneSelection {
  primaryStyle: number;
  primary: number[];
  subStyle: number;
  secondary: number[];
  shards: number[];
}

export function decodeRunes(selection: number[]): RuneSelection | null {
  if (selection.length !== 11) return null;
  const [primaryStyle, p1, p2, p3, p4, subStyle, s1, s2, f1, f2, f3] = selection as [
    number, number, number, number, number, number, number, number, number, number, number,
  ];
  return { primaryStyle, primary: [p1, p2, p3, p4], subStyle, secondary: [s1, s2], shards: [f1, f2, f3] };
}

const KEYS = ["Q", "W", "E", "R"] as const;

/** Emplacements de compétence 1 à 4 (timeline match-v5) traduits en touches. */
export function skillLetters(selection: number[]): string[] {
  return selection.map((slot) => KEYS[slot - 1] ?? "?");
}

/** Variantes d'une catégorie, de la plus jouée à la moins jouée. */
export function topVariants(builds: BuildStats[], category: string, limit: number): BuildStats[] {
  return builds
    .filter((build) => build.category === category)
    .sort((a, b) => b.games - a.games)
    .slice(0, limit);
}
