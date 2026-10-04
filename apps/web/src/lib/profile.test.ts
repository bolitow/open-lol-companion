import type { ProfileRank } from "@olc/shared";
import { describe, expect, it } from "vitest";
import { dictionaries } from "./i18n";
import { parseHistoryStart, rankLabel, riotIdFromSegments } from "./profile";

describe("parseHistoryStart", () => {
  it("lit un index de page borné comme l'API", () => {
    expect(parseHistoryStart(undefined)).toBe(0);
    expect(parseHistoryStart("20")).toBe(20);
    expect(parseHistoryStart(["30", "40"])).toBe(30);
    for (const invalid of ["-1", "abc", "1.5", "10001"]) {
      expect(parseHistoryStart(invalid), invalid).toBe(0);
    }
  });
});

describe("riotIdFromSegments", () => {
  it("décode chaque segment puis valide le Riot ID", () => {
    expect(riotIdFromSegments("Le%20Joueur%20%C3%89lu", "FR42")).toEqual({ gameName: "Le Joueur Élu", tagLine: "FR42" });
    expect(riotIdFromSegments("Le Joueur", "EUW")).toEqual({ gameName: "Le Joueur", tagLine: "EUW" });
  });

  it("refuse un segment invalide ou mal encodé", () => {
    expect(riotIdFromSegments("%E0%A4%A", "EUW")).toBeNull();
    expect(riotIdFromSegments("ab", "EUW")).toBeNull();
  });
});

describe("rankLabel", () => {
  const rank = (value: Partial<ProfileRank>): ProfileRank => ({
    queue_id: 420,
    status: "ranked",
    tier: "GOLD",
    division: "II",
    league_points: 45,
    ...value,
  });

  it("affiche le palier traduit, la division et les LP fournis par Riot", () => {
    expect(rankLabel(rank({}), dictionaries.fr)).toBe("Or II · 45 LP");
    expect(rankLabel(rank({}), dictionaries.en)).toBe("Gold II · 45 LP");
  });

  it("n'affiche pas de division au-delà de Diamant", () => {
    expect(rankLabel(rank({ tier: "MASTER", division: "I", league_points: 120 }), dictionaries.en)).toBe("Master · 120 LP");
  });

  it("n'invente aucun rang pour un joueur non classé", () => {
    expect(rankLabel(rank({ status: "unranked", tier: null, division: null, league_points: null }), dictionaries.fr)).toBe("Non classé");
  });

  it("garde un palier inconnu tel que publié", () => {
    expect(rankLabel(rank({ tier: "NEWTIER", division: null, league_points: null }), dictionaries.en)).toBe("NEWTIER");
  });
});
