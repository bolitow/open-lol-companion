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

/** Un succès Tauri vaut `null` ; un échec rejette avec l'un de ces codes. */
export type ImportResult = null;
export type ImportError =
  | "clientUnavailable"
  | "clientRejected"
  | "invalidClientData"
  | "notInChampSelect"
  | "invalidRunes"
  | "invalidSpells"
  | "runePageUnavailable";

const IMPORT_ERROR_MESSAGES: Record<"fr" | "en", Record<ImportError, string>> = {
  fr: {
    clientUnavailable: "Le client League of Legends est indisponible. Ouvrez-le puis réessayez.",
    clientRejected: "Le client a refusé l'import. Vérifiez les choix et, pour les runes, les places disponibles.",
    invalidClientData: "Les données du client sont incompatibles. Mettez l'application à jour puis réessayez.",
    notInChampSelect: "L'import des sorts est disponible pendant la sélection des champions.",
    invalidRunes: "La page de runes est invalide. Vérifiez les arbres, les lignes et les fragments.",
    invalidSpells: "Choisissez deux sorts d'invocateur différents.",
    runePageUnavailable: "La page de runes de l'application est verrouillée ou existe en plusieurs exemplaires. Vérifiez les pages dans le client.",
  },
  en: {
    clientUnavailable: "The League of Legends client is unavailable. Open it and try again.",
    clientRejected: "The client rejected the import. Check your choices and, for runes, the available page slots.",
    invalidClientData: "The client data is incompatible. Update the application and try again.",
    notInChampSelect: "Summoner spells can only be imported during champion select.",
    invalidRunes: "The rune page is invalid. Check the trees, rows, and stat shards.",
    invalidSpells: "Choose two different summoner spells.",
    runePageUnavailable: "The application's rune page is locked or has multiple copies. Check your pages in the client.",
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
