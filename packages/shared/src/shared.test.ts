import { describe, expect, it } from "vitest";
import {
  championIconUrl,
  displayPatch,
  PUBLICATION_STATE_EVENT,
  screenForPhase,
  type PublicationState,
} from "./index";

describe("screenForPhase", () => {
  it("envoie la sélection des champions vers l'écran de draft", () => {
    expect(screenForPhase("ChampSelect")).toBe("champ-select");
  });
  it("affiche les overlays pendant la partie", () => {
    expect(screenForPhase("InProgress")).toBe("in-game");
    expect(screenForPhase("Reconnect")).toBe("in-game");
  });
  it("revient au dashboard par défaut", () => {
    expect(screenForPhase("Lobby")).toBe("dashboard");
    expect(screenForPhase("Unknown")).toBe("dashboard");
  });
});

describe("ddragon", () => {
  it("construit les URLs d'icônes", () => {
    expect(championIconUrl("16.19.1", "Jinx")).toBe(
      "https://ddragon.leagueoflegends.com/cdn/16.19.1/img/champion/Jinx.png",
    );
  });
  it("raccourcit la version en patch", () => {
    expect(displayPatch("16.19.1")).toBe("16.19");
  });
});

describe("publications", () => {
  it("garde le nom d'événement et la forme émis par le cœur Rust", () => {
    expect(PUBLICATION_STATE_EVENT).toBe("publication-state");
    const state: PublicationState = {
      revision: 1,
      status: "connected",
      publication: {
        type: "data.updated",
        stats_version: "2026-10-01 12:05:00+00",
        static_version: null,
        available: true,
      },
    };
    expect(Object.keys(state)).toEqual(["revision", "status", "publication"]);
  });
});
