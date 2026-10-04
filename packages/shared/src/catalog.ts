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

/**
 * Préfixe de l'effet qui porte les valeurs d'un objet pour un mode (#116) : l'identifiant est
 * `cdragon_parameters:{clé de mode}`, la clé étant celle de la source (`ARAM`, `cherry`…) ou
 * un hachage non résolu (`{bffdf499}`, signalé par `unresolved_mode_key:` dans `coverage.issues`).
 * L'effet `cdragon_parameters` sans suffixe garde les valeurs de base, jamais remplacées.
 */
export const CATALOG_MODE_EFFECT_PREFIX = "cdragon_parameters:";
export type CatalogModeEffectId = `${typeof CATALOG_MODE_EFFECT_PREFIX}${string}`;

/** Cartes dont l'export desktop garde les objets : Faille, ARAM, Arena. */
export type CatalogItemMapId = "11" | "12" | "30";

/**
 * Miroir de `item_filter` dans le manifeste de l'export desktop (#116). `maps` est la liste
 * exportée ; `by_map` compte les objets conservés disponibles sur chaque carte.
 * Le repli `all_items` garde tous les objets quand la disponibilité par carte est incertaine.
 */
export interface DesktopCatalogItemFilter {
  mode: "maps_11_12_30_with_components" | "all_items";
  reason: string | null;
  maps: CatalogItemMapId[];
  by_map: Record<CatalogItemMapId, number>;
  /**
   * Objets dont la disponibilité sur la carte n'a pas pu être lue, par carte. N'apparaît qu'en
   * repli `all_items` ; omis (jamais `{}` ni `null`) quand le filtre par carte a pu s'appliquer.
   */
  unreadable_by_map?: Partial<Record<CatalogItemMapId, number>>;
}

/** Famille des augments Arena et Mayhem (#118) : catalogue statique, jamais une statistique. */
export const CATALOG_AUGMENT_KIND = "augment";

/** Raretés de la source CommunityDragon ; toute autre valeur y reste `unsupported`. */
export const CATALOG_AUGMENT_RARITIES = ["kSilver", "kGold", "kPrismatic", "kEventChoice"] as const;
export type CatalogAugmentRarity = (typeof CATALOG_AUGMENT_RARITIES)[number];

/**
 * Seuls champs de `fields` d'une fiche `augment`. `modes` liste les clés de mode des listes du jeu
 * qui contiennent l'augment (`CHERRY`, `KIWI`, `KIWI_JADE` sur le patch 16.19) telles que la source
 * les donne, sans correspondance inventée avec une file ; une liste vide signifie « aucune liste ».
 * `technical_id` est l'identifiant de jeu (`ARAM_ADAPt`) ; `community_short_name` et
 * `community_description` n'existent que si la source les renseigne. Aucun taux de victoire, de sélection, de popularité ni tier d'augment.
 */
export const CATALOG_AUGMENT_FIELDS = [
  "community_name", "community_id", "technical_id", "community_short_name",
  "community_description", "rarity", "community_icon_path", "modes",
] as const;
export type CatalogAugmentField = (typeof CATALOG_AUGMENT_FIELDS)[number];

/**
 * Fiche `augment` (#118) : `id` est l'identifiant numérique en chaîne, `namespace` vaut `standard`,
 * `icon` est le chemin local exporté (ou l'URL publique côté API). `description` est le texte brut
 * de l'export CommunityDragon `cdragon/arena` (225 augments sur 16.19), dans la langue de la fiche ;
 * les placeholders (`@MaxStacks@`, `{{ clé }}`) et jetons d'icône du client (`%i:Augment%`) restent non résolus dans le texte et sont signalés par
 * `unresolved_placeholder:description` dans `coverage.issues`. Les autres augments, Mayhem, n'ont
 * aucune description publiée : `description` vaut `null`, signalé par `missing:description`.
 */
export interface CatalogAugmentRecord extends CatalogRecord {
  kind: typeof CATALOG_AUGMENT_KIND;
  description: string | null;
  fields: Partial<Record<CatalogAugmentField, CatalogValue>>;
  stats: Record<string, never>;
  effects: never[];
}

/** Nom du champ des fiches `ability` et `summoner_spell` qui porte les segments typés de `tooltip` (#107). */
export const CATALOG_TOOLTIP_SEGMENTS_FIELD = "tooltip_segments";

/** Types de dégâts des balises `physicalDamage`, `magicDamage` et `trueDamage` des infobulles Data Dragon. */
export const CATALOG_DAMAGE_TYPES = ["physical", "magic", "true"] as const;
export type CatalogDamageType = (typeof CATALOG_DAMAGE_TYPES)[number];

/**
 * Fragment d'une infobulle (#107) : texte seul, jamais de balisage. `damage_type` vient de la balise
 * de la source, pas de la formulation du texte, donc il vaut dans toutes les langues ; `null` hors balise
 * de dégâts. Les balises imbriquées (`scaleAP`…) héritent du type qui les entoure, les placeholders
 * (`{{ e1 }}`) restent non résolus. Dans `CatalogValue.value` du champ `tooltip_segments`, ce sont des
 * `CatalogTooltipSegment[]` (statut `derived`, provenance `…/tooltip`) dont les textes concaténés
 * sont exactement le champ `tooltip` ; l'absence d'infobulle donne l'absence du champ.
 */
export interface CatalogTooltipSegment {
  text: string;
  damage_type: CatalogDamageType | null;
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
