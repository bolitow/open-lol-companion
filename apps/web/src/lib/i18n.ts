import type { ApiErrorCode } from "@olc/shared";

export const LOCALES = ["fr", "en"] as const;
export type Locale = (typeof LOCALES)[number];
export const DEFAULT_LOCALE: Locale = "fr";

/** Erreurs affichables : codes de l'API (#19) et API non configurée côté serveur du site. */
export type SiteErrorCode = ApiErrorCode | "not_configured";

export function isLocale(value: string): value is Locale {
  return (LOCALES as readonly string[]).includes(value);
}

/** Langue des documents statiques servis par l'API (`fr_FR`, `en_US`). */
export function staticLocale(locale: Locale): "fr_FR" | "en_US" {
  return locale === "fr" ? "fr_FR" : "en_US";
}

/**
 * Même page dans la langue `target` : seul le segment de langue en tête du chemin est
 * remplacé, la chaîne de requête (filtres) est conservée telle quelle.
 */
export function localeHref(pathname: string, locale: string, target: string, search: string): string {
  return `${pathname.replace(new RegExp(`^/${locale}(?=/|$)`), `/${target}`)}${search}`;
}

/** Remplace `{nom}` par sa valeur ; une variable absente reste visible pour être repérée. */
export function interpolate(template: string, values: Record<string, string>): string {
  return template.replace(/\{(\w+)\}/g, (match, name: string) => values[name] ?? match);
}

const fr = {
  site: {
    name: "Open LoL Companion",
    description: "Tierlist, pages champion et profils League of Legends, gratuits et open source.",
    navigation: "Navigation principale",
    tierlist: "Tierlist",
    otherLanguage: "English",
    otherLanguageLabel: "Lire le site en anglais",
    legal:
      "Open LoL Companion n’est pas approuvé par Riot Games et ne reflète pas les opinions de Riot Games ou de toute personne officiellement impliquée dans la production ou la gestion de ses propriétés. Riot Games et toutes les propriétés associées sont des marques de Riot Games, Inc.",
    sources:
      "Statistiques agrégées par Open LoL Companion à partir des parties publiques de l’API Riot Games. Noms et images : Data Dragon.",
  },
  filters: {
    legend: "Filtres",
    role: "Rôle",
    rank: "Rang",
    platform: "Région",
    queue: "File",
    patch: "Patch",
    apply: "Appliquer",
    roles: { TOP: "Haut", JUNGLE: "Jungle", MIDDLE: "Milieu", BOTTOM: "Bas", UTILITY: "Support" },
    ranks: {
      ALL: "Tous les rangs",
      IRON: "Fer",
      BRONZE: "Bronze",
      SILVER: "Argent",
      GOLD: "Or",
      PLATINUM: "Platine",
      EMERALD: "Émeraude",
      DIAMOND: "Diamant",
      MASTER: "Maître",
      GRANDMASTER: "Grand Maître",
      CHALLENGER: "Challenger",
    },
    queues: { "420": "Classée Solo/Duo", "440": "Classée Flex" },
  },
  stats: {
    published: "Données publiées le {date}.",
    threshold: "Seuil d’échantillon par taux : {count} ; « — » en dessous.",
    tierMethod: "Méthode du tier (texte technique publié par l’API) :",
    pickRate: "Définition du taux de sélection :",
    bans: "Les bans portent sur toute la draft, sans distinction de rôle ni de rang.",
    noPatch: "Aucune version des données n’est encore publiée.",
    otherPatches: "Patches disponibles dans cette publication :",
  },
  tierlist: {
    title: "Tierlist",
    intro: "Champions du rôle choisi, classés selon la publication la plus récente.",
    caption: "Tierlist {role}, {rank}, {platform}, patch {patch}",
    position: "Position",
    champion: "Champion",
    tier: "Tier",
    winRate: "Victoires",
    pickRate: "Sélection",
    banRate: "Bans",
    games: "Parties",
    empty: "Aucune donnée pour ce périmètre.",
    unknownChampion: "Champion {id}",
  },
  champion: {
    titleSuffix: "builds, runes et compétences",
    back: "Retour à la tierlist",
    summary: "Résumé",
    games: "Parties : {count}",
    runes: "Runes",
    spells: "Sorts d’invocateur",
    items: "Objets finaux",
    trinket: "Totem",
    popularItems: "Objets les plus fréquents",
    skillOrder: "Ordre des compétences",
    noData: "Aucune donnée pour ce champion dans ce rôle et ce périmètre. Essayez un autre rôle.",
    noVariants: "Aucune variante publiée.",
    notFound: "Champion introuvable.",
    variantShare: "{share} des parties",
    variantWin: "{rate} de victoires",
  },
  profile: {
    titleSuffix: "profil",
    level: "Niveau {level}",
    ranks: "Classements",
    queues: { "420": "Classée Solo/Duo", "440": "Classée Flex" },
    otherQueue: "File {id}",
    unranked: "Non classé",
    leaguePoints: "{lp} LP",
    history: "Historique",
    win: "Victoire",
    loss: "Défaite",
    kda: "{kills} / {deaths} / {assists}",
    duration: "Durée",
    next: "Parties suivantes",
    first: "Parties les plus récentes",
    empty: "Aucune partie sur cette page.",
    omitted: "Parties personnalisées ou d’une autre plateforme masquées : {count}.",
    fetchedAt: "Lu le {date}.",
    notProvided: "Statistiques par champion, pic de rang, suivi des LP et parties en direct ne sont pas encore fournis par l’API.",
    invalid: "Riot ID ou région invalide.",
  },
  search: {
    open: "Rechercher",
    shortcut: "Ctrl K",
    title: "Recherche",
    label: "Champion ou Riot ID",
    placeholder: "Ahri, Nom#TAG…",
    champions: "Champions",
    player: "Joueur",
    platform: "Région du joueur",
    openProfile: "Voir le profil de {id} ({platform})",
    hint: "Saisissez un nom de champion, ou un Riot ID complet (Nom#TAG) pour ouvrir un profil.",
    noResult: "Aucun champion trouvé.",
    close: "Fermer",
  },
  errors: {
    invalid_request: "Filtres refusés par l’API.",
    unauthorized: "Le site n’est pas autorisé à lire l’API.",
    forbidden: "Accès refusé par l’API.",
    not_found: "Introuvable.",
    unavailable: "Données momentanément indisponibles. Réessayez plus tard.",
    rate_limited: "Trop de demandes en cours. Réessayez dans quelques instants.",
    riot_busy: "Riot est momentanément saturé. Réessayez dans quelques instants.",
    not_configured: "L’accès à l’API n’est pas configuré sur ce serveur.",
  },
};

export type Dictionary = typeof fr;

const en: Dictionary = {
  site: {
    name: "Open LoL Companion",
    description: "League of Legends tier list, champion pages and profiles, free and open source.",
    navigation: "Main navigation",
    tierlist: "Tier list",
    otherLanguage: "Français",
    otherLanguageLabel: "Read the site in French",
    legal:
      "Open LoL Companion is not endorsed by Riot Games and does not reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games and all associated properties are trademarks or registered trademarks of Riot Games, Inc.",
    sources:
      "Statistics aggregated by Open LoL Companion from public games of the Riot Games API. Names and images: Data Dragon.",
  },
  filters: {
    legend: "Filters",
    role: "Role",
    rank: "Rank",
    platform: "Region",
    queue: "Queue",
    patch: "Patch",
    apply: "Apply",
    roles: { TOP: "Top", JUNGLE: "Jungle", MIDDLE: "Mid", BOTTOM: "Bot", UTILITY: "Support" },
    ranks: {
      ALL: "All ranks",
      IRON: "Iron",
      BRONZE: "Bronze",
      SILVER: "Silver",
      GOLD: "Gold",
      PLATINUM: "Platinum",
      EMERALD: "Emerald",
      DIAMOND: "Diamond",
      MASTER: "Master",
      GRANDMASTER: "Grandmaster",
      CHALLENGER: "Challenger",
    },
    queues: { "420": "Ranked Solo/Duo", "440": "Ranked Flex" },
  },
  stats: {
    published: "Data published on {date}.",
    threshold: "Sample threshold per rate: {count}; “—” below it.",
    tierMethod: "Tier method (technical text published by the API):",
    pickRate: "Pick rate definition:",
    bans: "Bans cover the whole draft, regardless of role or rank.",
    noPatch: "No data version has been published yet.",
    otherPatches: "Patches available in this publication:",
  },
  tierlist: {
    title: "Tier list",
    intro: "Champions of the selected role, ranked by the latest publication.",
    caption: "Tier list {role}, {rank}, {platform}, patch {patch}",
    position: "Position",
    champion: "Champion",
    tier: "Tier",
    winRate: "Win rate",
    pickRate: "Pick rate",
    banRate: "Ban rate",
    games: "Games",
    empty: "No data for this selection.",
    unknownChampion: "Champion {id}",
  },
  champion: {
    titleSuffix: "builds, runes and skills",
    back: "Back to the tier list",
    summary: "Summary",
    games: "Games: {count}",
    runes: "Runes",
    spells: "Summoner spells",
    items: "Final items",
    trinket: "Trinket",
    popularItems: "Most common items",
    skillOrder: "Skill order",
    noData: "No data for this champion in this role and selection. Try another role.",
    noVariants: "No published variant.",
    notFound: "Champion not found.",
    variantShare: "{share} of games",
    variantWin: "{rate} win rate",
  },
  profile: {
    titleSuffix: "profile",
    level: "Level {level}",
    ranks: "Rankings",
    queues: { "420": "Ranked Solo/Duo", "440": "Ranked Flex" },
    otherQueue: "Queue {id}",
    unranked: "Unranked",
    leaguePoints: "{lp} LP",
    history: "Match history",
    win: "Victory",
    loss: "Defeat",
    kda: "{kills} / {deaths} / {assists}",
    duration: "Duration",
    next: "Next games",
    first: "Most recent games",
    empty: "No games on this page.",
    omitted: "Custom or other-platform games hidden: {count}.",
    fetchedAt: "Read on {date}.",
    notProvided: "Per-champion stats, peak rank, LP tracking and live games are not provided by the API yet.",
    invalid: "Invalid Riot ID or region.",
  },
  search: {
    open: "Search",
    shortcut: "Ctrl K",
    title: "Search",
    label: "Champion or Riot ID",
    placeholder: "Ahri, Name#TAG…",
    champions: "Champions",
    player: "Player",
    platform: "Player region",
    openProfile: "Open {id}'s profile ({platform})",
    hint: "Type a champion name, or a full Riot ID (Name#TAG) to open a profile.",
    noResult: "No champion found.",
    close: "Close",
  },
  errors: {
    invalid_request: "Filters rejected by the API.",
    unauthorized: "The site is not allowed to read the API.",
    forbidden: "Access denied by the API.",
    not_found: "Not found.",
    unavailable: "Data temporarily unavailable. Please try again later.",
    rate_limited: "Too many requests. Please try again in a moment.",
    riot_busy: "Riot is temporarily busy. Please try again in a moment.",
    not_configured: "API access is not configured on this server.",
  },
};

export const dictionaries: Record<Locale, Dictionary> = { fr, en };

export function dictionary(locale: Locale): Dictionary {
  return dictionaries[locale];
}

export function errorMessage(locale: Locale, code: SiteErrorCode): string {
  return dictionaries[locale].errors[code];
}
