import type { TierlistResponse } from "@olc/shared";
import { describe, expect, it } from "vitest";
import { indexChampions, tierlistRows } from "./champions";

const championDoc = {
  type: "champion",
  version: "16.19.1",
  data: {
    Ahri: { id: "Ahri", key: "103", name: "Ahri", title: "le renard à neuf queues" },
    MonkeyKing: { id: "MonkeyKing", key: "62", name: "Wukong", title: "le roi des singes" },
    Broken: { id: "Broken", name: "Sans clé" },
  },
};

describe("indexChampions", () => {
  it("indexe les champions par identifiant numérique et ignore les fiches incomplètes", () => {
    const index = indexChampions(championDoc);
    expect(index.size).toBe(2);
    expect(index.get(62)).toEqual({ key: 62, id: "MonkeyKing", name: "Wukong", title: "le roi des singes" });
  });

  it("renvoie un index vide pour un document inattendu", () => {
    expect(indexChampions(null).size).toBe(0);
    expect(indexChampions({ data: "x" }).size).toBe(0);
  });
});

function stats(champion_id: number, position: number | null, tier: string | null) {
  return {
    patch: "16.19",
    platform_id: "EUW1",
    queue_id: 420,
    role: "MIDDLE" as const,
    rank: "ALL",
    champion_id,
    games: 120,
    wins: 66,
    losses: 54,
    population: 1000,
    win_rate: position === null ? null : 55,
    pick_rate: position === null ? null : 12,
    win_rate_lower_bound: null,
    position,
    tier,
    most_picked_rank: null,
  };
}

describe("tierlistRows", () => {
  it("associe nom, ban et rang publié sans recalculer le tier", () => {
    const response = {
      entries: [stats(103, 1, "S"), stats(62, null, null), stats(999, 2, "A")],
      bans: [
        { patch: "16.19", platform_id: "EUW1", queue_id: 420, champion_id: 103, banned_matches: 30, draft_matches: 300, ban_rate: 10 },
      ],
    } as unknown as TierlistResponse;
    const rows = tierlistRows(response, indexChampions(championDoc));
    expect(rows.map((r) => [r.championId, r.name, r.tier, r.banRate])).toEqual([
      [103, "Ahri", "S", 10],
      [62, "Wukong", null, null],
      [999, null, "A", null],
    ]);
    expect(rows[0]?.iconId).toBe("Ahri");
    expect(rows[2]?.iconId).toBeNull();
  });
});
