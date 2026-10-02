/** Contrats des commandes d’import du client LoL (miroirs Rust). */
export interface ImportRunesRequest {
  championName: string;
  primaryStyleId: number;
  subStyleId: number;
  /** Quatre runes primaires, deux secondaires, puis trois fragments, dans l'ordre du client. */
  selectedPerkIds: number[];
}

/** Un succès Tauri vaut `null` ; un échec rejette avec l'un de ces codes. */
export type ImportResult = null;
export type ImportError =
  | "clientUnavailable"
  | "clientRejected"
  | "invalidClientData"
  | "invalidRunes"
  | "runePageUnavailable";

const IMPORT_ERROR_MESSAGES: Record<"fr" | "en", Record<ImportError, string>> = {
  fr: {
    clientUnavailable: "Le client League of Legends est indisponible. Ouvrez-le puis réessayez.",
    clientRejected: "Le client a refusé l'import. Vérifiez les choix et, pour les runes, les places disponibles.",
    invalidClientData: "Les données du client sont incompatibles. Mettez l'application à jour puis réessayez.",
    invalidRunes: "La page de runes est invalide. Vérifiez les arbres, les lignes et les fragments.",
    runePageUnavailable: "La page de runes de l'application est verrouillée ou existe en plusieurs exemplaires. Vérifiez les pages dans le client.",
  },
  en: {
    clientUnavailable: "The League of Legends client is unavailable. Open it and try again.",
    clientRejected: "The client rejected the import. Check your choices and, for runes, the available page slots.",
    invalidClientData: "The client data is incompatible. Update the application and try again.",
    invalidRunes: "The rune page is invalid. Check the trees, rows, and stat shards.",
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
