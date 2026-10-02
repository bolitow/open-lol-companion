import type { AccountId } from "./model";

// Jeux de démonstration : aucune donnée personnelle ou issue d’une API de partie.
export const assets = "/prototype/assets/";
export const accounts = {
  nebuleuse: { name: "Nébuleuse", tag: "EUW", champion: "Ahri", lp: 68, level: 178, rank: "emerald" as const },
  orbite: { name: "Orbite", tag: "EUW", champion: "Orianna", lp: 124, level: 242, rank: "master" as const },
};
export interface Match {
  id: string; account: AccountId; champion: string; win: boolean; queue: "ranked" | "aram";
  kda: [number, number, number]; minutes: number; seconds: number; cs: number; lp: number; items: number[];
}
const seeds: Omit<Match, "id" | "account">[] = [
  { champion: "Ahri", win: true, queue: "ranked", kda: [12, 3, 8], minutes: 28, seconds: 14, cs: 202, lp: 24, items: [6655, 3020, 3118, 3089, 3157, 3135] },
  { champion: "Jinx", win: false, queue: "ranked", kda: [6, 7, 4], minutes: 31, seconds: 2, cs: 210, lp: -19, items: [3006, 3031, 3085, 3072] },
  { champion: "Ahri", win: true, queue: "ranked", kda: [8, 2, 11], minutes: 24, seconds: 18, cs: 184, lp: 23, items: [6655, 3020, 3118, 3089, 3157] },
  { champion: "Lux", win: true, queue: "aram", kda: [10, 6, 24], minutes: 19, seconds: 46, cs: 66, lp: 0, items: [6655, 3020, 3089, 3135] },
  { champion: "Orianna", win: false, queue: "ranked", kda: [4, 5, 9], minutes: 29, seconds: 8, cs: 203, lp: -18, items: [6655, 3020, 3118, 3157] },
  { champion: "Ahri", win: true, queue: "ranked", kda: [7, 2, 14], minutes: 32, seconds: 44, cs: 228, lp: 24, items: [6655, 3020, 3118, 3089, 3157, 3135] },
  { champion: "Thresh", win: false, queue: "aram", kda: [3, 9, 21], minutes: 22, seconds: 31, cs: 30, lp: 0, items: [3020, 3157] },
  { champion: "Ahri", win: true, queue: "ranked", kda: [9, 1, 12], minutes: 26, seconds: 10, cs: 194, lp: 22, items: [6655, 3020, 3118, 3089, 3157] },
];
export const matches: Match[] = [
  ...seeds.map((m, i) => ({ ...m, id: `n-${i + 1}`, account: "nebuleuse" as const })),
  ...seeds.slice(2, 7).map((m, i) => ({ ...m, champion: i % 2 ? "Lux" : "Orianna", id: `o-${i + 1}`, account: "orbite" as const })),
];
export const friends = [
  { name: "LuneBleue", champion: "Jinx", time: 18, followed: false },
  { name: "Kiro", champion: "LeeSin", time: 23, followed: false },
  { name: "Orion", champion: "Yasuo", time: 0, followed: true },
  { name: "Lynae", champion: "Lux", time: 12, followed: false },
  { name: "Mistral", champion: "Vi", time: 31, followed: true },
  { name: "Tokage", champion: "Thresh", time: 8, followed: false },
];
