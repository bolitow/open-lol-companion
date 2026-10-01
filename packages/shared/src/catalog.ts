import type { JsonValue } from "./api";

/** Langues des routes ; `und` signale un catalogue Riot sans traduction. */
export type CatalogLocale = "fr_FR" | "en_US" | "und";
/** `global` est réservé aux cartes/files/modes/types non versionnés de Riot. */
export type CatalogNamespace = "standard" | "classic" | "global";

/** Référentiel #61 ; les clés et valeurs nulles correspondent au JSON Rust. */
export type CatalogValueStatus =
  | "verified"
  | "derived"
  | "descriptive"
  | "missing"
  | "unsupported"
  | "conflict";

export interface CatalogValueSource {
  source_id: string;
  pointer: string;
}

/** Une absence n'est ni zéro ni faux ; une valeur conflictuelle n'est pas validée. */
export interface CatalogValue {
  value: JsonValue;
  unit: string | null;
  status: CatalogValueStatus;
  sources: CatalogValueSource[];
}

export interface CatalogSourceMeta {
  id: string;
  provider: string;
  key: string;
  version: string;
  locale: string | null;
  url: string;
  observed_at: string;
}

/** Source brute conservée côté collecteur ; les routes publiques exposent sa métadonnée. */
export interface CatalogSource extends CatalogSourceMeta {
  data: JsonValue;
}

export interface CatalogEffect {
  id: string;
  description: string | null;
  parameters: Record<string, CatalogValue>;
  calculation: CatalogValue | null;
}

export interface CatalogRecordCoverage {
  source_fields: number;
  normalized_fields: number;
  unmapped_fields: string[];
  issues: string[];
}

export interface CatalogRecord {
  kind: string;
  id: string;
  /** standard/classic ; global uniquement avec locale und pour les catalogues Riot. */
  namespace: string;
  locale: string;
  name: string;
  /** Texte seul : ne jamais injecter cette description comme HTML. */
  description: string | null;
  icon: string | null;
  fields: Record<string, CatalogValue>;
  stats: Record<string, CatalogValue>;
  effects: CatalogEffect[];
  coverage: CatalogRecordCoverage;
}

export interface CatalogCoverage {
  records: number;
  source_fields: number;
  normalized_fields: number;
  unmapped_fields: number;
  records_with_issues: number;
  by_kind: Record<string, number>;
}

/** Branche sans lien vers une valeur publiée, conservée dans la source brute. */
export interface RawBranch {
  pointer: string;
  leaf_fields: number;
  reason: string;
}

/** Un lien de provenance ne prouve pas que la mécanique a été interprétée. */
export interface SourceInventory {
  source_id: string;
  total_leaf_fields: number;
  branches: number;
  linked_branches: number;
  raw_branches: RawBranch[];
}

export interface CatalogManifest {
  publication_id: string;
  version: string;
  normalizer_version: number;
  published_at: string;
  degraded: boolean;
  warnings: string[];
  sources: CatalogSourceMeta[];
  source_inventory: SourceInventory[];
  coverage: CatalogCoverage;
}

/** Total filtré et lignes appartiennent à la même publication atomique. */
export interface CatalogPage {
  publication_id: string;
  version: string;
  locale: string;
  kind: string;
  namespace: string;
  total: number;
  offset: number;
  limit: number;
  records: CatalogRecord[];
}

export interface CatalogDetail {
  publication_id: string;
  version: string;
  record: CatalogRecord;
}

/** Ces sections décrivent un changement de référentiel, sans conclusion d'équilibrage. */
export interface CatalogChange {
  id: string;
  change: "added" | "removed" | "modified";
  sections: ("fields" | "stats" | "effects" | "text" | "source" | "coverage")[];
}

export interface CatalogDiffPage {
  from: string;
  to: string;
  locale: string;
  kind: string;
  namespace: string;
  total: number;
  offset: number;
  limit: number;
  changes: CatalogChange[];
}
