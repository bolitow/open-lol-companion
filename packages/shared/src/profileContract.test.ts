import { describe, expect, it } from "vitest";
import golden from "./contracts/profile.json";
import type { Profile, ProfileRank } from "./index";

// `Record<keyof T, true>` oblige tsc à refuser tout champ ajouté ou retiré sans mise à jour ici.
const RANK_KEYS: Record<keyof ProfileRank, true> = {
  queue_id: true,
  status: true,
  tier: true,
  division: true,
  league_points: true,
  wins: true,
  losses: true,
  hot_streak: true,
  veteran: true,
  fresh_blood: true,
  inactive: true,
};
const PROFILE_KEYS: Record<keyof Profile, true> = {
  platform: true,
  game_name: true,
  tag_line: true,
  puuid: true,
  profile_icon_id: true,
  summoner_level: true,
  ranks: true,
  fetched_at: true,
};

describe("contrat Profile (partagé avec olc-api et olc-build-client)", () => {
  it("le JSON de référence a exactement les champs du type TypeScript", () => {
    expect(Object.keys(golden).sort()).toEqual(Object.keys(PROFILE_KEYS).sort());
    for (const rank of golden.ranks) {
      expect(Object.keys(rank).sort()).toEqual(Object.keys(RANK_KEYS).sort());
    }
  });
  it("le JSON de référence est assignable à Profile", () => {
    const profile = golden as Profile;
    expect(profile.ranks[0]?.wins).toBe(12);
    expect(profile.ranks[1]?.wins).toBeNull();
  });
});
