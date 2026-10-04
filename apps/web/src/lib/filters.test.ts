import { describe, expect, it } from "vitest";
import { filtersHref, parseStatsFilters, statsSearchParams } from "./filters";

describe("parseStatsFilters", () => {
  it("applique les valeurs par défaut sans inventer de patch", () => {
    expect(parseStatsFilters({}, { patch: "16.19" })).toEqual({
      patch: "16.19",
      platform: "EUW1",
      queue: 420,
      role: "MIDDLE",
      rank: "ALL",
    });
    expect(parseStatsFilters({}, { patch: null }).patch).toBeNull();
  });

  it("conserve les filtres valides du lien", () => {
    expect(
      parseStatsFilters(
        { patch: "16.18", platform: "KR", queue: "440", role: "UTILITY", rank: "EMERALD" },
        { patch: "16.19" },
      ),
    ).toEqual({ patch: "16.18", platform: "KR", queue: 440, role: "UTILITY", rank: "EMERALD" });
  });

  it("remplace chaque valeur invalide par sa valeur par défaut", () => {
    expect(
      parseStatsFilters(
        { patch: "16.19.1", platform: "EUROPE", queue: "1700", role: "MID", rank: "FAKE" },
        { patch: "16.19" },
      ),
    ).toEqual({ patch: "16.19", platform: "EUW1", queue: 420, role: "MIDDLE", rank: "ALL" });
  });

  it("refuse le rôle inconnu et les populations hors rang affiché", () => {
    const parsed = parseStatsFilters({ role: "UNKNOWN", rank: "UNRANKED_MODE" }, { patch: "16.19" });
    expect(parsed.role).toBe("MIDDLE");
    expect(parsed.rank).toBe("ALL");
  });

  it("prend la première valeur d'un paramètre répété", () => {
    expect(parseStatsFilters({ role: ["TOP", "JUNGLE"] }, { patch: "16.19" }).role).toBe("TOP");
  });
});

describe("statsSearchParams", () => {
  it("construit la requête attendue par l'API, sans paramètre inconnu", () => {
    const query = statsSearchParams({
      patch: "16.19",
      platform: "EUW1",
      queue: 420,
      role: "MIDDLE",
      rank: "ALL",
    });
    expect(query.toString()).toBe(
      "patch=16.19&platform=EUW1&queue=420&role=MIDDLE&rank=ALL&offset=0&limit=200",
    );
  });
});

describe("filtersHref", () => {
  it("garde les filtres actifs et applique la modification demandée", () => {
    const filters = { patch: "16.19", platform: "EUW1", queue: 420, role: "MIDDLE", rank: "ALL" } as const;
    expect(filtersHref("/fr/tierlist", filters, { role: "TOP" })).toBe(
      "/fr/tierlist?patch=16.19&platform=EUW1&queue=420&role=TOP&rank=ALL",
    );
  });

  it("omet le patch quand aucun n'est connu", () => {
    const filters = { patch: null, platform: "KR", queue: 440, role: "JUNGLE", rank: "GOLD" } as const;
    expect(filtersHref("/en/tierlist", filters)).toBe(
      "/en/tierlist?platform=KR&queue=440&role=JUNGLE&rank=GOLD",
    );
  });
});

describe("patchOptions", () => {
  it("fusionne les patches connus sans doublon, du plus récent au plus ancien", async () => {
    const { patchOptions } = await import("./filters");
    expect(patchOptions(["16.19", null, "16.9"], [["16.18", "16.19"], ["16.10"]])).toEqual(["16.19", "16.18", "16.10", "16.9"]);
    expect(patchOptions([null], [])).toEqual([]);
  });

  it("ignore une valeur de patch mal formée", async () => {
    const { patchOptions } = await import("./filters");
    expect(patchOptions(["16.19"], [["abc", "16.19.1"]])).toEqual(["16.19"]);
  });
});
