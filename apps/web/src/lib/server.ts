import "server-only";
import { displayPatch } from "@olc/shared";
import { cache } from "react";
import { createApiClient } from "./api";
import { indexChampions, type ChampionIndex } from "./champions";
import { staticLocale, type Locale } from "./i18n";
import { indexItems, indexRunes, indexSpells } from "./staticData";

/**
 * Accès à l'API depuis le serveur du site uniquement : `OLC_API_TOKEN` n'est jamais
 * préfixé `NEXT_PUBLIC_`, ne part pas dans le HTML et n'est jamais journalisé.
 */
export const getApi = cache(() =>
  createApiClient({
    baseUrl: process.env.OLC_API_URL?.trim() || "http://127.0.0.1:3030",
    token: process.env.OLC_API_TOKEN?.trim() || null,
  }),
);

export interface StaticContext {
  /** Version Data Dragon publiée (`16.19.1`) et patch de jeu associé (`16.19`). */
  version: string;
  patch: string;
  champions: ChampionIndex;
}

/** Version en ligne et champions de la langue ; `null` si l'API ne sert pas encore de statiques. */
export const loadStatic = cache(async (locale: Locale): Promise<StaticContext | null> => {
  const api = getApi();
  const manifest = await api.manifest();
  if (!manifest.ok) return null;
  const version = manifest.data.live_version;
  const champions = await api.staticDocument(version, staticLocale(locale), "champion.json");
  return {
    version,
    patch: displayPatch(version),
    champions: champions.ok ? indexChampions(champions.data) : new Map(),
  };
});

/** Objets, sorts et runes nécessaires aux pages champion et profil. */
export const loadGameData = cache(async (locale: Locale, version: string) => {
  const api = getApi();
  const [items, spells, runes] = await Promise.all([
    api.staticDocument(version, staticLocale(locale), "item.json"),
    api.staticDocument(version, staticLocale(locale), "summoner.json"),
    api.staticDocument(version, staticLocale(locale), "runesReforged.json"),
  ]);
  return {
    items: items.ok ? indexItems(items.data) : new Map(),
    spells: spells.ok ? indexSpells(spells.data) : new Map(),
    runes: runes.ok ? indexRunes(runes.data) : new Map(),
  };
});
