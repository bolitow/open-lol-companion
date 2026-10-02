import { describe, expect, it } from "vitest";
import { importErrorMessage, type ImportError } from "./imports";

describe("erreurs des imports", () => {
  it("guide le joueur vers la sélection des champions dans les deux langues", () => {
    expect(importErrorMessage("notInChampSelect", "fr")).toContain("sélection des champions");
    expect(importErrorMessage("notInChampSelect", "en")).toContain("champion select");
  });

  it("fournit un message pour chaque code Rust dans les deux langues", () => {
    const errors: ImportError[] = [
      "clientUnavailable",
      "clientRejected",
      "invalidClientData",
      "invalidRunes",
      "runePageUnavailable",
      "notInChampSelect",
      "invalidSpells",
      "invalidItems",
      "itemSetPriorityUnavailable",
    ];
    for (const error of errors) {
      const french = importErrorMessage(error, "fr");
      const english = importErrorMessage(error, "en");
      expect(french.length).toBeGreaterThan(10);
      expect(english.length).toBeGreaterThan(10);
      expect(french).not.toBe(english);
    }
  });

  it("masque les erreurs inconnues au lieu de transmettre un détail interne", () => {
    for (const error of ["détail interne", null, { message: "détail interne" }, "constructor", "__proto__"]) {
      for (const locale of ["fr", "en"] as const) {
        const message = importErrorMessage(error, locale);
        expect(message).toBe(importErrorMessage(undefined, locale));
        expect(message).not.toContain("détail interne");
        expect(typeof message).toBe("string");
        expect(message.length).toBeGreaterThan(10);
      }
    }
  });
});
