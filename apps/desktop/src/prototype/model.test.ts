import { describe, expect, it } from "vitest";
import { initialSession, sessionReducer, parsePreferences, searchCatalog, filterMatches } from "./model";

describe("accueil de démonstration", () => {
  it("conserve le compte et son historique à la fermeture du client", () => {
    const state = { ...initialSession, account: "orbite" as const, selectedMatch: "o-1" };
    const closed = sessionReducer(state, { type: "connection", connected: false });
    expect(closed.account).toBe("orbite");
    expect(closed.selectedMatch).toBe("o-1");
    expect(closed.connected).toBe(false);
  });
  it("bascule vers un autre compte sans conserver le détail du précédent", () => {
    const state = { ...initialSession, selectedMatch: "n-1" };
    const switched = sessionReducer(state, { type: "account", account: "orbite" });
    expect(switched.account).toBe("orbite");
    expect(switched.selectedMatch).toBeNull();
    expect(switched.connected).toBe(true);
  });
  it("filtre les parties par compte avant le mode, même après déconnexion", () => {
    const rows = [
      { account: "nebuleuse", queue: "ranked" },
      { account: "orbite", queue: "ranked" },
      { account: "nebuleuse", queue: "aram" },
    ];
    expect(filterMatches(rows, "nebuleuse", "ranked")).toEqual([rows[0]]);
    expect(filterMatches(rows, "orbite", "all")).toEqual([rows[1]]);
  });
});

describe("préférences de l’aperçu", () => {
  it("démarre en sombre, sans reprendre les données d’une session réelle", () => {
    expect(parsePreferences(null)).toEqual({ theme: "dark", locale: "fr", motion: true });
  });
  it("restaure les choix valides et tolère un stockage corrompu", () => {
    expect(parsePreferences('{"theme":"light","locale":"en","motion":false}'))
      .toEqual({ theme: "light", locale: "en", motion: false });
    expect(parsePreferences("not-json").theme).toBe("dark");
    expect(parsePreferences('{"theme":"unknown","locale":"xx","motion":"false"}'))
      .toEqual({ theme: "dark", locale: "fr", motion: true });
    expect(parsePreferences("null").theme).toBe("dark");
  });
});

describe("recherche globale", () => {
  const entries = [
    { id: "a", label: "Ahri", keywords: "champion mid mage" },
    { id: "n", label: "Nébuleuse#EUW", keywords: "joueur player" },
    { id: "c", label: "Clips", keywords: "enregistrement recording" },
  ];
  it("cherche par nom ou intention sans tenir compte des accents et de la casse", () => {
    expect(searchCatalog(entries, " NEBULEUSE ")).toEqual([entries[1]]);
    expect(searchCatalog(entries, "enregistrement")).toEqual([entries[2]]);
    expect(searchCatalog(entries, "AhRi")).toEqual([entries[0]]);
  });
  it("retourne un état vide pour une recherche sans correspondance", () => {
    expect(searchCatalog(entries, "inexistant")).toEqual([]);
    expect(searchCatalog(entries, "")).toEqual(entries);
  });
});

it("ouvre les liens de partage de draft et de maquettes, sinon l’accueil", async()=>{
  const {pageFromHash}=await import("./model");
  expect(pageFromHash("#layouts")).toBe("layouts");
  expect(pageFromHash("#draft")).toBe("draft");
  expect(pageFromHash("#unknown")).toBe("home");
});
