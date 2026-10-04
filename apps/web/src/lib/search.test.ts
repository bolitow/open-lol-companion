import { describe, expect, it } from "vitest";
import type { ChampionInfo } from "./champions";
import { isSearchShortcut, normalizeSearch, searchChampions, searchTarget } from "./search";

const champions: ChampionInfo[] = [
  { key: 103, id: "Ahri", name: "Ahri", title: "" },
  { key: 166, id: "Akshan", name: "Akshan", title: "" },
  { key: 145, id: "Kaisa", name: "Kai'Sa", title: "" },
  { key: 62, id: "MonkeyKing", name: "Wukong", title: "" },
  { key: 875, id: "Sett", name: "Sett", title: "" },
];

describe("normalizeSearch", () => {
  it("ignore casse, accents, espaces et apostrophes", () => {
    expect(normalizeSearch("  Kai'Sa ")).toBe("kaisa");
    expect(normalizeSearch("Évelynn")).toBe("evelynn");
  });
});

describe("searchChampions", () => {
  it("place les débuts de nom avant les correspondances internes", () => {
    expect(searchChampions("a", champions, 10).map((c) => c.name)).toEqual(["Ahri", "Akshan", "Kai'Sa"]);
  });

  it("trouve un champion par son nom affiché ou son identifiant Data Dragon", () => {
    expect(searchChampions("kaisa", champions, 10).map((c) => c.key)).toEqual([145]);
    expect(searchChampions("monkey", champions, 10).map((c) => c.key)).toEqual([62]);
  });

  it("borne le nombre de résultats et ne renvoie rien pour une saisie vide", () => {
    expect(searchChampions("a", champions, 1)).toHaveLength(1);
    expect(searchChampions("   ", champions, 10)).toEqual([]);
  });
});

describe("isSearchShortcut", () => {
  it("ouvre la recherche avec Ctrl+K (Windows) ou Cmd+K (macOS), majuscule comprise", () => {
    expect(isSearchShortcut({ ctrlKey: true, metaKey: false, key: "k" })).toBe(true);
    expect(isSearchShortcut({ ctrlKey: false, metaKey: true, key: "k" })).toBe(true);
    expect(isSearchShortcut({ ctrlKey: true, metaKey: false, key: "K" })).toBe(true);
  });

  it("ignore la touche K seule et les autres raccourcis", () => {
    expect(isSearchShortcut({ ctrlKey: false, metaKey: false, key: "k" })).toBe(false);
    expect(isSearchShortcut({ ctrlKey: true, metaKey: false, key: "j" })).toBe(false);
  });
});

describe("searchTarget", () => {
  const ahri = champions.slice(0, 1);

  it("ouvre le profil du Riot ID sur la plateforme choisie", () => {
    expect(searchTarget("Le Joueur#FR42", "KR", [], "en")).toBe("/en/profile/KR/Le%20Joueur/FR42");
  });

  it("préfère le Riot ID complet aux champions trouvés", () => {
    expect(searchTarget("Ahri Main#EUW", "EUW1", ahri, "fr")).toBe("/fr/profile/EUW1/Ahri%20Main/EUW");
  });

  it("ouvre sinon la page du premier champion trouvé", () => {
    expect(searchTarget("monkey", "EUW1", [champions[3]!, champions[0]!], "fr")).toBe("/fr/champions/monkeyking");
  });

  it("ne navigue pas pour une saisie vide ou un Riot ID invalide sans champion", () => {
    expect(searchTarget("   ", "EUW1", [], "fr")).toBeNull();
    expect(searchTarget("ab#EUW", "EUW1", [], "fr")).toBeNull();
    expect(searchTarget("Faker#", "EUW1", [], "fr")).toBeNull();
  });
});
