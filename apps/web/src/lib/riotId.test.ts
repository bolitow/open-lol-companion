import { describe, expect, it } from "vitest";
import { parseRiotId, profilePath } from "./riotId";

describe("parseRiotId", () => {
  it("sépare le nom et le tag", () => {
    expect(parseRiotId("  Faker#KR1 ")).toEqual({ gameName: "Faker", tagLine: "KR1" });
  });

  it("accepte les noms avec espaces et accents", () => {
    expect(parseRiotId("Le Joueur Élu#FR42")).toEqual({ gameName: "Le Joueur Élu", tagLine: "FR42" });
  });

  it("refuse une saisie sans tag ou hors des longueurs Riot", () => {
    for (const invalid of ["Faker", "#EUW", "Faker#", "ab#EUW", "Faker#E", "Faker#TOOLONG", "a".repeat(17) + "#EUW", "Faker#E-W", "Fa#ker#EUW"]) {
      expect(parseRiotId(invalid), invalid).toBeNull();
    }
  });
});

describe("profilePath", () => {
  it("encode séparément chaque segment du Riot ID", () => {
    expect(profilePath("fr", "EUW1", { gameName: "Le Joueur/Élu?", tagLine: "FR42" })).toBe(
      "/fr/profile/EUW1/Le%20Joueur%2F%C3%89lu%3F/FR42",
    );
  });
});
