import { describe, expect, expectTypeOf, it } from "vitest";
import type {
  CatalogChange, CatalogDetail, CatalogEffect, CatalogManifest,
  CatalogPage, CatalogRecord, CatalogValue, CatalogValueStatus, CatalogLocale, CatalogNamespace, SourceInventory, RawBranch, JsonValue,
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
});
