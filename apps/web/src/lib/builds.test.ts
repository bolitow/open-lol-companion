import type { BuildStats } from "@olc/shared";
import { describe, expect, it } from "vitest";
import { decodeRunes, skillLetters, topVariants } from "./builds";

function build(category: string, selection: number[], games: number): BuildStats {
  return {
    patch: "16.19",
    platform_id: "EUW1",
    queue_id: 420,
    role: "MIDDLE",
    rank: "ALL",
    champion_id: 103,
    category,
    selection,
    games,
    wins: null,
    performance_available: false,
    population: 500,
    pick_rate: null,
    win_rate: null,
  };
}

describe("decodeRunes", () => {
  it("découpe la sélection publiée en arbre principal, secondaire et fragments", () => {
    expect(decodeRunes([8100, 8112, 8139, 8138, 8135, 8200, 8226, 8210, 5008, 5008, 5001])).toEqual({
      primaryStyle: 8100,
      primary: [8112, 8139, 8138, 8135],
      subStyle: 8200,
      secondary: [8226, 8210],
      shards: [5008, 5008, 5001],
    });
  });

  it("refuse une sélection de longueur inattendue", () => {
    expect(decodeRunes([8100, 8112])).toBeNull();
  });
});

describe("skillLetters", () => {
  it("traduit les emplacements 1 à 4 en touches Q W E R", () => {
    expect(skillLetters([1, 2, 3, 1, 1, 4])).toEqual(["Q", "W", "E", "Q", "Q", "R"]);
  });

  it("marque un emplacement inconnu sans l'inventer", () => {
    expect(skillLetters([1, 7])).toEqual(["Q", "?"]);
  });
});

describe("topVariants", () => {
  it("garde la catégorie demandée, triée par effectif décroissant", () => {
    const builds = [
      build("runes", [1], 10),
      build("final_items", [3089, 3020], 40),
      build("final_items", [3157], 90),
      build("final_items", [3165], 5),
    ];
    expect(topVariants(builds, "final_items", 2).map((b) => b.selection)).toEqual([[3157], [3089, 3020]]);
  });
});
