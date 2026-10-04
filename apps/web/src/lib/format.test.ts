import { describe, expect, it } from "vitest";
import { formatCount, formatDate, formatDuration, formatRate } from "./format";

// Intl insère des espaces insécables : on les normalise pour comparer.
const plain = (value: string) => value.replace(/[  ]/g, " ");

describe("formatRate", () => {
  it("affiche à une décimale un taux publié en points de pourcentage (0 à 100)", () => {
    expect(plain(formatRate(52.34, "fr"))).toBe("52,3 %");
    expect(formatRate(52.34, "en")).toBe("52.3%");
    expect(formatRate(100, "en")).toBe("100.0%");
    expect(formatRate(0.0156, "en")).toBe("0.0%");
  });

  it("n'invente pas de valeur sous le seuil d'échantillon", () => {
    expect(formatRate(null, "fr")).toBe("—");
    expect(formatRate(undefined, "en")).toBe("—");
  });
});

describe("formatCount", () => {
  it("sépare les milliers selon la langue", () => {
    expect(plain(formatCount(12345, "fr"))).toBe("12 345");
    expect(formatCount(12345, "en")).toBe("12,345");
  });
});

describe("formatDuration", () => {
  it("affiche minutes et secondes", () => {
    expect(formatDuration(1865)).toBe("31:05");
    expect(formatDuration(59)).toBe("0:59");
  });
});

describe("formatDate", () => {
  it("affiche la date en UTC, indépendamment du fuseau du serveur", () => {
    expect(plain(formatDate("2026-10-03 23:19:56.341176+00", "fr"))).toBe("3 oct. 2026, 23:19 UTC");
    expect(plain(formatDate(Date.UTC(2026, 9, 3, 23, 19), "en"))).toBe("Oct 3, 2026, 11:19 PM UTC");
  });

  it("rend tel quel un horodatage illisible au lieu d'échouer", () => {
    expect(formatDate("hier", "fr")).toBe("hier");
  });
});
