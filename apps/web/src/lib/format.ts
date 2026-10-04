import type { Locale } from "./i18n";

const tags: Record<Locale, string> = { fr: "fr-FR", en: "en-US" };

/**
 * Taux publié par l'API en points de pourcentage (`… * 100`, voir `pick_rate_definition`) ;
 * `null` signifie « sous le seuil d'échantillon ».
 */
export function formatRate(value: number | null | undefined, locale: Locale): string {
  if (value === null || value === undefined) return "—";
  return new Intl.NumberFormat(tags[locale], {
    style: "percent",
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(value / 100);
}

export function formatCount(value: number, locale: Locale): string {
  return new Intl.NumberFormat(tags[locale]).format(value);
}

export function formatDuration(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  return `${minutes}:${String(seconds % 60).padStart(2, "0")}`;
}

/** Date en UTC : la page est rendue par le serveur, dont le fuseau n'est pas celui du joueur. */
export function formatDate(value: string | number, locale: Locale): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
  const text = new Intl.DateTimeFormat(tags[locale], { dateStyle: "medium", timeStyle: "short", timeZone: "UTC" }).format(date);
  return `${text} UTC`;
}
