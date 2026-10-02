/** Contrats des commandes d’import du client LoL (miroirs Rust). */
export interface ImportRunesRequest {
  championName: string;
  primaryStyleId: number;
  subStyleId: number;
  /** Quatre runes primaires, deux secondaires, puis trois fragments, dans l'ordre du client. */
  selectedPerkIds: number[];
}

export type FlashSlot = "D" | "F";

export interface ImportSpellsRequest {
  spellIds: [number, number];
  flashSlot: FlashSlot;
}

export interface ItemStack {
  id: number;
  count: number;
}

export interface ItemBlock {
  label: string;
  items: ItemStack[];
}

export interface ImportItemsRequest {
  championId: number;
  championName: string;
  mapId: number;
  blocks: ItemBlock[];
}

/** Un succès Tauri vaut `null` ; un échec rejette avec l'un de ces codes. */
export type ImportResult = null;
export type ImportError =
  | "clientUnavailable"
  | "clientRejected"
  | "invalidClientData"
  | "notInChampSelect"
  | "invalidRunes"
  | "invalidSpells"
  | "invalidItems"
  | "runePageUnavailable"
  | "itemSetPriorityUnavailable";

const IMPORT_ERROR_MESSAGES: Record<"fr" | "en", Record<ImportError, string>> = {
  fr: {
    clientUnavailable: "Le client League of Legends est indisponible. Ouvrez-le puis réessayez.",
    clientRejected: "Le client a refusé l'import. Vérifiez les choix et, pour les runes, les places disponibles.",
    invalidClientData: "Les données du client sont incompatibles. Mettez l'application à jour puis réessayez.",
    notInChampSelect: "L'import des sorts est disponible pendant la sélection des champions.",
    invalidRunes: "La page de runes est invalide. Vérifiez les arbres, les lignes et les fragments.",
    invalidSpells: "Choisissez deux sorts d'invocateur différents.",
    invalidItems: "Le set d'items est invalide. Vérifiez le champion, la carte et les blocs d'objets.",
    runePageUnavailable: "La page de runes de l'application est verrouillée ou existe en plusieurs exemplaires. Vérifiez les pages dans le client.",
    itemSetPriorityUnavailable: "Un set existant utilise la priorité maximale. L'import ne peut pas être placé en premier.",
  },
  en: {
    clientUnavailable: "The League of Legends client is unavailable. Open it and try again.",
    clientRejected: "The client rejected the import. Check your choices and, for runes, the available page slots.",
    invalidClientData: "The client data is incompatible. Update the application and try again.",
    notInChampSelect: "Summoner spells can only be imported during champion select.",
    invalidRunes: "The rune page is invalid. Check the trees, rows, and stat shards.",
    invalidSpells: "Choose two different summoner spells.",
    invalidItems: "The item set is invalid. Check the champion, map, and item blocks.",
    runePageUnavailable: "The application's rune page is locked or has multiple copies. Check your pages in the client.",
    itemSetPriorityUnavailable: "An existing item set uses the highest priority. The import cannot be placed first.",
  },
};

/** Traduit les codes connus et masque toute erreur brute inattendue de l'IPC. */
export function importErrorMessage(error: unknown, locale: "fr" | "en"): string {
  const messages = IMPORT_ERROR_MESSAGES[locale];
  if (typeof error === "string" && Object.prototype.hasOwnProperty.call(messages, error)) {
    return messages[error as ImportError];
  }
  return locale === "fr" ? "L'import a échoué. Réessayez." : "The import failed. Try again.";
}
