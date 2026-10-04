import { describe, expect, it } from "vitest";
import { championBySlug, championSlug, indexChampions } from "./champions";
import { indexItems, indexRunes, indexSpells } from "./staticData";

describe("championSlug", () => {
  it("utilise l'identifiant Data Dragon en minuscules pour l'URL", () => {
    const index = indexChampions({ data: { MonkeyKing: { id: "MonkeyKing", key: "62", name: "Wukong" } } });
    expect(championSlug({ id: "MonkeyKing" })).toBe("monkeyking");
    expect(championBySlug(index, "monkeyking")?.key).toBe(62);
    expect(championBySlug(index, "MonkeyKing")?.key).toBe(62);
    expect(championBySlug(index, "wukong")).toBeUndefined();
  });
});

describe("indexItems", () => {
  it("indexe les objets par identifiant numérique", () => {
    const items = indexItems({ data: { "3157": { name: "Sablier de Zhonya" }, bad: { name: "x" }, "1001": {} } });
    expect([...items.entries()]).toEqual([[3157, { name: "Sablier de Zhonya" }]]);
  });
});

describe("indexSpells", () => {
  it("indexe les sorts d'invocateur par clé numérique avec leur image", () => {
    const spells = indexSpells({ data: { SummonerFlash: { id: "SummonerFlash", key: "4", name: "Saut éclair" } } });
    expect(spells.get(4)).toEqual({ id: "SummonerFlash", name: "Saut éclair" });
  });
});

describe("indexRunes", () => {
  it("indexe les arbres et les runes de runesReforged.json", () => {
    const runes = indexRunes([
      {
        id: 8100,
        name: "Domination",
        icon: "perk-images/Styles/7200_Domination.png",
        slots: [{ runes: [{ id: 8112, name: "Électrocution", icon: "perk-images/Styles/Domination/Electrocute/Electrocute.png" }] }],
      },
      { id: "x" },
    ]);
    expect(runes.get(8100)).toEqual({ name: "Domination", icon: "perk-images/Styles/7200_Domination.png" });
    expect(runes.get(8112)?.name).toBe("Électrocution");
    expect(runes.size).toBe(2);
    expect(indexRunes(null).size).toBe(0);
  });
});
