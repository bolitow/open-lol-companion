import { describe, expect, it } from "vitest";
import { dictionaries, errorMessage, isLocale, localeHref, staticLocale } from "./i18n";

function keys(value: unknown, prefix = ""): string[] {
  if (typeof value !== "object" || value === null) return [prefix];
  return Object.entries(value).flatMap(([key, child]) => keys(child, prefix ? `${prefix}.${key}` : key));
}

describe("i18n", () => {
  it("traduit toutes les chaînes en français et en anglais", () => {
    expect(keys(dictionaries.en)).toEqual(keys(dictionaries.fr));
    for (const locale of ["fr", "en"] as const) {
      for (const key of keys(dictionaries[locale])) {
        const value = key.split(".").reduce<unknown>((node, part) => (node as Record<string, unknown>)[part], dictionaries[locale]);
        expect(typeof value === "string" && value.trim().length > 0, `${locale}.${key}`).toBe(true);
      }
    }
  });

  it("reconnaît uniquement les langues du site", () => {
    expect(isLocale("fr")).toBe(true);
    expect(isLocale("en")).toBe(true);
    expect(isLocale("de")).toBe(false);
  });

  it("associe chaque langue au catalogue statique servi par l'API", () => {
    expect(staticLocale("fr")).toBe("fr_FR");
    expect(staticLocale("en")).toBe("en_US");
  });

  it("traduit chaque code d'erreur de l'API", () => {
    for (const code of ["invalid_request", "unauthorized", "not_found", "unavailable", "rate_limited", "not_configured"] as const) {
      expect(errorMessage("fr", code)).not.toBe(errorMessage("en", code));
      expect(errorMessage("fr", code).length).toBeGreaterThan(0);
    }
  });
});

describe("interpolate", () => {
  it("remplace les variables nommées et laisse les inconnues visibles", async () => {
    const { interpolate } = await import("./i18n");
    expect(interpolate("{count} parties sur {platform}", { count: "12", platform: "EUW1" })).toBe("12 parties sur EUW1");
    expect(interpolate("Champion {id}", {})).toBe("Champion {id}");
  });
});

describe("localeHref", () => {
  it("remplace le préfixe de langue et conserve les filtres de l'URL", () => {
    expect(localeHref("/fr/tierlist", "fr", "en", "?role=TOP&rank=GOLD")).toBe("/en/tierlist?role=TOP&rank=GOLD");
    expect(localeHref("/en/champions/ahri", "en", "fr", "?patch=16.19")).toBe("/fr/champions/ahri?patch=16.19");
  });

  it("traite l'accueil d'une langue et une URL sans filtre", () => {
    expect(localeHref("/fr", "fr", "en", "")).toBe("/en");
    expect(localeHref("/fr/tierlist", "fr", "en", "")).toBe("/en/tierlist");
  });

  it("ne remplace qu'un segment de langue entier en tête de chemin", () => {
    expect(localeHref("/fra/tierlist", "fr", "en", "")).toBe("/fra/tierlist");
    expect(localeHref("/en/profile/EUW1/fr/EUW", "en", "fr", "")).toBe("/fr/profile/EUW1/fr/EUW");
  });
});
