import { describe, expect, expectTypeOf, it } from "vitest";
import type {
  CatalogChange, CatalogDetail, CatalogEffect, CatalogManifest,
  CatalogPage, CatalogRecord, CatalogValue, CatalogValueStatus, CatalogLocale, CatalogNamespace, SourceInventory, RawBranch, JsonValue,
  CatalogModeEffectId, DesktopCatalogItemFilter, CatalogItemMapId,
  CatalogAugmentRarity, CatalogAugmentRecord, CatalogAugmentField,
  CatalogDamageType, CatalogTooltipSegment,
} from "./index";
import {
  CATALOG_MODE_EFFECT_PREFIX, CATALOG_AUGMENT_KIND, CATALOG_AUGMENT_RARITIES,
  CATALOG_AUGMENT_FIELDS, CATALOG_DAMAGE_TYPES, CATALOG_TOOLTIP_SEGMENTS_FIELD,
} from "./index";

describe("contrats JSON du catalogue", () => {
  it("préserve les valeurs absentes, zéro, faux et les sources multiples", () => {
    const values = [null, 0, false].map((value): CatalogValue => ({
      value, unit: null, status: value === null ? "missing" : "verified", sources: [],
    }));
    const conflict: CatalogValue = {
      value: [15, 20], unit: "points", status: "conflict",
      sources: [{ source_id: "a", pointer: "/stats/haste" }, { source_id: "b", pointer: "/mAbilityHasteMod" }],
    };
    expect(JSON.parse(JSON.stringify(values)).map((v: CatalogValue) => v.value)).toEqual([null, 0, false]);
    expect(conflict.sources).toHaveLength(2);
    expectTypeOf<CatalogValue["value"]>().toEqualTypeOf<JsonValue>();
    expectTypeOf<CatalogValueStatus>().toEqualTypeOf<"verified" | "derived" | "descriptive" | "missing" | "unsupported" | "conflict">();
  });

  it("garde les champs nullables présents comme dans les contrats Rust", () => {
    expectTypeOf<CatalogManifest["source_inventory"]>().toEqualTypeOf<SourceInventory[]>();
    expectTypeOf<SourceInventory["raw_branches"]>().toEqualTypeOf<RawBranch[]>();
    expectTypeOf<CatalogLocale>().toEqualTypeOf<"fr_FR" | "en_US" | "und">();
    expectTypeOf<CatalogNamespace>().toEqualTypeOf<"standard" | "classic" | "global">();
    expectTypeOf<CatalogRecord["description"]>().toEqualTypeOf<string | null>();
    expectTypeOf<CatalogEffect["calculation"]>().toEqualTypeOf<CatalogValue | null>();
    expectTypeOf<CatalogManifest["sources"][number]["locale"]>().toEqualTypeOf<string | null>();
    expectTypeOf<CatalogPage["records"][number]>().toEqualTypeOf<CatalogDetail["record"]>();
    expectTypeOf<CatalogChange["change"]>().toEqualTypeOf<"added" | "removed" | "modified">();
    expectTypeOf<CatalogChange["sections"][number]>().toEqualTypeOf<"fields" | "stats" | "effects" | "text" | "source" | "coverage">();
  });

  it("décrit les objets par carte et les surcharges de valeurs par mode (#116)", () => {
    const filter: DesktopCatalogItemFilter = {
      mode: "maps_11_12_30_with_components", reason: null,
      maps: ["11", "12", "30"], by_map: { "11": 316, "12": 404, "30": 232 },
    };
    expect(JSON.parse(JSON.stringify(filter)).by_map["12"]).toBe(404);
    expectTypeOf<DesktopCatalogItemFilter["maps"][number]>().toEqualTypeOf<CatalogItemMapId>();
    expectTypeOf<CatalogItemMapId>().toEqualTypeOf<"11" | "12" | "30">();
    const aram: CatalogModeEffectId = `${CATALOG_MODE_EFFECT_PREFIX}ARAM`;
    expect(aram).toBe("cdragon_parameters:ARAM");
    // @ts-expect-error l'effet de base n'est pas un effet de mode
    const base: CatalogModeEffectId = "cdragon_parameters";
    expect(base).toBe("cdragon_parameters");
  });

  it("décrit les augments Arena et Mayhem comme catalogue statique sans statistique (#118)", () => {
    expect(CATALOG_AUGMENT_KIND).toBe("augment");
    expect(CATALOG_AUGMENT_RARITIES).toEqual(["kSilver", "kGold", "kPrismatic", "kEventChoice"]);
    expectTypeOf<CatalogAugmentRarity>().toEqualTypeOf<"kSilver" | "kGold" | "kPrismatic" | "kEventChoice">();
    expectTypeOf<CatalogAugmentRecord["kind"]>().toEqualTypeOf<"augment">();
    expectTypeOf<CatalogAugmentRecord["description"]>().toEqualTypeOf<string | null>();
    expectTypeOf<CatalogAugmentRecord["stats"]>().toEqualTypeOf<Record<string, never>>();
    expectTypeOf<CatalogAugmentRecord["effects"]>().toEqualTypeOf<never[]>();
    expectTypeOf<keyof CatalogAugmentRecord["fields"]>().toEqualTypeOf<CatalogAugmentField>();
    // Riot : aucun taux de victoire, de sélection, de popularité ni tier d'augment.
    for (const field of CATALOG_AUGMENT_FIELDS) {
      expect(field).not.toMatch(/win|pick|tier|popular|rate|games/i);
    }
    expect(CATALOG_AUGMENT_FIELDS).toContain("community_description");
    // Description présente (export Arena) ou absente (Mayhem) : jamais une chaîne inventée.
    const described: CatalogAugmentRecord = {
      kind: "augment", id: "93", namespace: "standard", locale: "fr_FR", name: "Échauffement",
      description: "Vous obtenez le sort d'invocateur Échauffement . (jusqu'à @MaxStacks@% max).",
      icon: null,
      fields: {
        community_description: { value: "Vous obtenez le sort d'invocateur Échauffement . (jusqu'à @MaxStacks@% max).", unit: null, status: "descriptive", sources: [] },
      },
      stats: {}, effects: [],
      coverage: { source_fields: 20, normalized_fields: 10, unmapped_fields: [], issues: ["unresolved_placeholder:description"] },
    };
    expect(described.coverage.issues).not.toContain("missing:description");
    const augment: CatalogAugmentRecord = {
      kind: "augment", id: "1205", namespace: "standard", locale: "fr_FR", name: "Adaptation",
      description: null, icon: "/game-data/catalog/icons/a.png",
      fields: {
        technical_id: { value: "ARAM_ADAPt", unit: null, status: "verified", sources: [] },
        rarity: { value: "kSilver", unit: null, status: "verified", sources: [] },
        modes: { value: ["KIWI", "KIWI_JADE"], unit: null, status: "derived", sources: [] },
      },
      stats: {}, effects: [],
      coverage: { source_fields: 6, normalized_fields: 6, unmapped_fields: [], issues: ["missing:description"] },
    };
    expect(JSON.parse(JSON.stringify(augment)).fields.modes.value).toEqual(["KIWI", "KIWI_JADE"]);
    // @ts-expect-error un augment n'a ni statistique ni champ de popularité
    const forbidden: CatalogAugmentRecord["fields"] = { pick_rate: augment.fields.rarity };
    expect(forbidden).toBeDefined();
  });

  it("décrit les segments typés d'infobulle, miroir du JSON Rust (#107)", () => {
    expect(CATALOG_TOOLTIP_SEGMENTS_FIELD).toBe("tooltip_segments");
    expect(CATALOG_DAMAGE_TYPES).toEqual(["physical", "magic", "true"]);
    expectTypeOf<CatalogDamageType>().toEqualTypeOf<"physical" | "magic" | "true">();
    expectTypeOf<CatalogTooltipSegment["damage_type"]>().toEqualTypeOf<CatalogDamageType | null>();
    expectTypeOf<CatalogTooltipSegment["text"]>().toEqualTypeOf<string>();
    const segments: CatalogTooltipSegment[] = [
      { text: "Inflige ", damage_type: null },
      { text: "{{ e1 }} dégâts magiques", damage_type: "magic" },
    ];
    const value: CatalogValue = {
      value: segments as unknown as JsonValue, unit: null, status: "derived",
      sources: [{ source_id: "source:fr_FR/champion/Ahri.json", pointer: "/data/Ahri/spells/0/tooltip" }],
    };
    expect(JSON.parse(JSON.stringify(value)).value[1].damage_type).toBe("magic");
    expect(JSON.parse(JSON.stringify(value)).value[0].damage_type).toBeNull();
  });
});
