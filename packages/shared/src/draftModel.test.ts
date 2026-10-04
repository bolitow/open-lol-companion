import { describe, expect, it } from "vitest";
import {
  roleFromDraftPosition,
  scoreDraft,
  type ChampionStats,
  type DraftModelInput,
  type Role,
} from "./index";

const POPULATION = { patch: "16.19", platform_id: "EUW1", queue_id: 420, rank: "ALL" };

/** Entrée publiée minimale ; le taux et la borne suivent le seuil comme le collecteur. */
function stat(
  champion_id: number,
  role: Role,
  wins: number,
  games: number,
  lower: number | null,
  overrides: Partial<ChampionStats> = {},
): ChampionStats {
  return {
    ...POPULATION,
    role,
    champion_id,
    games,
    wins,
    losses: games - wins,
    population: 1000,
    win_rate: games >= 100 ? (100 * wins) / games : null,
    pick_rate: games >= 100 ? (100 * games) / 1000 : null,
    win_rate_lower_bound: lower,
    position: null,
    tier: null,
    most_picked_rank: null,
    ...overrides,
  };
}

function input(overrides: Partial<DraftModelInput> = {}): DraftModelInput {
  return {
    population: POPULATION,
    min_games: 100,
    role: "MIDDLE",
    stats: [],
    allies: [],
    enemies: [],
    bans: [],
    ...overrides,
  };
}

describe("scoreDraft : score par champion", () => {
  it("note chaque candidat du rôle par la borne basse de Wilson publiée, sans la recalculer", () => {
    const result = scoreDraft(
      input({
        stats: [
          stat(1, "MIDDLE", 55, 100, 45.2),
          stat(2, "MIDDLE", 520, 1000, 48.9),
          stat(3, "MIDDLE", 30, 50, null),
          stat(4, "TOP", 900, 1000, 87.9),
        ],
      }),
    );
    expect(result.candidates.map((c) => [c.champion_id, c.score, c.position])).toEqual([
      [2, 48.9, 1],
      [1, 45.2, 2],
      [3, null, null],
    ]);
    expect(result.candidates[0]).toMatchObject({ role: "MIDDLE", games: 1000, win_rate: 52 });
  });

  it("départage les égalités par effectif puis par identifiant", () => {
    const result = scoreDraft(
      input({
        stats: [
          stat(9, "MIDDLE", 60, 120, 40),
          stat(8, "MIDDLE", 60, 120, 40),
          stat(7, "MIDDLE", 100, 200, 40),
        ],
      }),
    );
    expect(result.candidates.map((c) => c.champion_id)).toEqual([7, 8, 9]);
  });

  it("exclut les bans, les champions adverses et ceux des autres alliés, pas le choix local", () => {
    const result = scoreDraft(
      input({
        stats: [1, 2, 3, 4, 5].map((id) => stat(id, "MIDDLE", 60, 100, 40 + id)),
        bans: [1],
        enemies: [2],
        allies: [
          { champion_id: 3, role: "TOP", local: false },
          { champion_id: 4, role: "MIDDLE", local: true },
        ],
      }),
    );
    expect(result.candidates.map((c) => c.champion_id)).toEqual([5, 4]);
  });

  it("donne la part du rôle parmi les cinq postes connus du champion", () => {
    const result = scoreDraft(
      input({
        stats: [
          stat(1, "MIDDLE", 300, 600, 46),
          stat(1, "TOP", 200, 400, 45),
          stat(1, "UNKNOWN", 50, 100, 40),
        ],
      }),
    );
    expect(result.candidates[0]?.role_share).toBe(60);
  });

  it("laisse la part du rôle nulle pour le poste UNKNOWN, hors des cinq postes connus", () => {
    const result = scoreDraft(
      input({
        role: "UNKNOWN",
        stats: [stat(1, "UNKNOWN", 100, 200, 43), stat(1, "TOP", 25, 50, null)],
      }),
    );
    expect(result.candidates.map((c) => [c.champion_id, c.score])).toEqual([[1, 43]]);
    expect(result.candidates[0]?.role_share).toBeNull();
  });

  it("ignore les entrées d'une autre population (rang, patch, plateforme ou file)", () => {
    const result = scoreDraft(
      input({
        stats: [
          stat(1, "MIDDLE", 60, 100, 41),
          stat(2, "MIDDLE", 60, 100, 99, { rank: "GOLD" }),
          stat(3, "MIDDLE", 60, 100, 99, { patch: "16.18" }),
          stat(4, "MIDDLE", 60, 100, 99, { platform_id: "KR" }),
          stat(5, "MIDDLE", 60, 100, 99, { queue_id: 440 }),
          stat(1, "TOP", 900, 1000, 99, { rank: "GOLD" }),
        ],
      }),
    );
    expect(result.candidates.map((c) => c.champion_id)).toEqual([1]);
    expect(result.candidates[0]?.role_share).toBe(100);
  });

  it("déclare les matchups et synergies absents tant qu'aucun agrégat n'est publié", () => {
    const result = scoreDraft(input());
    expect(result.matchups_available).toBe(false);
    expect(result.synergies_available).toBe(false);
    expect(result.method).toMatch(/matchups and synergies not included/);
  });
});

describe("scoreDraft : estimation descriptive du draft", () => {
  it("reste nulle tant qu'un camp n'a aucun champion éligible", () => {
    const result = scoreDraft(
      input({
        stats: [stat(1, "TOP", 55, 100, 45)],
        allies: [{ champion_id: 1, role: "TOP", local: false }],
        enemies: [2],
      }),
    );
    expect(result.teams.ally).toEqual({
      champions: 1,
      eligible: 1,
      mean_win_rate: 55,
      estimated_win_rate: null,
    });
    expect(result.teams.enemy).toEqual({
      champions: 1,
      eligible: 0,
      mean_win_rate: null,
      estimated_win_rate: null,
    });
  });

  it("partage 100 % au prorata des taux moyens publiés des deux camps", () => {
    const result = scoreDraft(
      input({
        stats: [
          stat(1, "TOP", 52, 100, 42),
          stat(2, "MIDDLE", 56, 100, 46),
          stat(3, "JUNGLE", 48, 100, 38),
        ],
        allies: [
          { champion_id: 1, role: "TOP", local: false },
          { champion_id: 2, role: "MIDDLE", local: true },
        ],
        enemies: [3],
      }),
    );
    expect(result.teams.ally.mean_win_rate).toBe(54);
    expect(result.teams.enemy.mean_win_rate).toBe(48);
    expect(result.teams.ally.estimated_win_rate).toBeCloseTo((100 * 54) / 102, 10);
    expect(result.teams.enemy.estimated_win_rate).toBeCloseTo((100 * 48) / 102, 10);
    expect(
      (result.teams.ally.estimated_win_rate ?? 0) + (result.teams.enemy.estimated_win_rate ?? 0),
    ).toBeCloseTo(100, 10);
  });

  it("n'attribue aucun poste aux adversaires : taux tous rôles confondus, sous le seuil exclus", () => {
    const result = scoreDraft(
      input({
        stats: [
          stat(1, "MIDDLE", 30, 60, null),
          stat(1, "TOP", 20, 40, null),
          stat(1, "UNKNOWN", 10, 20, null),
          stat(2, "MIDDLE", 40, 80, null),
        ],
        enemies: [1, 2],
      }),
    );
    expect(result.teams.enemy).toMatchObject({ champions: 2, eligible: 1, mean_win_rate: 50 });
  });

  it("utilise le rôle visible d'un allié, sans repli tous rôles s'il joue hors de ses postes publiés", () => {
    const result = scoreDraft(
      input({
        stats: [stat(1, "TOP", 60, 100, 50), stat(1, "MIDDLE", 400, 1000, 37), stat(2, "TOP", 50, 100, 40)],
        allies: [
          { champion_id: 1, role: "MIDDLE", local: false },
          { champion_id: 2, role: "JUNGLE", local: false },
          { champion_id: 3, role: null, local: true },
        ],
      }),
    );
    expect(result.teams.ally).toMatchObject({ champions: 3, eligible: 1, mean_win_rate: 40 });
  });

  it("compte un allié sans poste visible avec son taux tous rôles confondus", () => {
    const result = scoreDraft(
      input({
        stats: [stat(1, "TOP", 60, 100, 50), stat(1, "MIDDLE", 40, 100, 30)],
        allies: [{ champion_id: 1, role: null, local: false }],
      }),
    );
    expect(result.teams.ally.mean_win_rate).toBe(50);
  });
});

describe("scoreDraft : conformité", () => {
  it("ne produit que des agrégats : clés figées, aucune identité ni recommandation unique", () => {
    const result = scoreDraft(
      input({ stats: [stat(1, "MIDDLE", 60, 100, 41)], enemies: [1] }),
    );
    expect(Object.keys(result).sort()).toEqual([
      "candidates",
      "matchups_available",
      "method",
      "min_games",
      "population",
      "role",
      "synergies_available",
      "teams",
    ]);
    expect(Object.keys(result.teams).sort()).toEqual(["ally", "enemy"]);
    expect(Object.keys(result.teams.enemy).sort()).toEqual([
      "champions",
      "eligible",
      "estimated_win_rate",
      "mean_win_rate",
    ]);
    const candidate = scoreDraft(input({ stats: [stat(1, "MIDDLE", 60, 100, 41)] })).candidates[0];
    expect(Object.keys(candidate ?? {}).sort()).toEqual([
      "champion_id",
      "games",
      "pick_rate",
      "position",
      "role",
      "role_share",
      "score",
      "win_rate",
    ]);
  });
});

describe("roleFromDraftPosition", () => {
  it("convertit le poste visible de la projection Rust vers le rôle des statistiques", () => {
    expect(roleFromDraftPosition("top")).toBe("TOP");
    expect(roleFromDraftPosition("jungle")).toBe("JUNGLE");
    expect(roleFromDraftPosition("middle")).toBe("MIDDLE");
    expect(roleFromDraftPosition("bottom")).toBe("BOTTOM");
    expect(roleFromDraftPosition("utility")).toBe("UTILITY");
    expect(roleFromDraftPosition(null)).toBeNull();
  });
});
