import type {CatalogRecord} from '@olc/shared';
import {loadCatalog, loadChampionAbilities, type PreparationCatalog} from '../catalog';
import type {Locale} from '../state';
export interface LiveCatalogState {catalog: PreparationCatalog | null; abilities: CatalogRecord[]; error: boolean}
/** Les ressources de l'ancien champion ne sont jamais publiées après son départ. */
export function connectLiveCatalog(locale: Locale, championId: number, receive: (state: LiveCatalogState) => void, loaders = {catalog: loadCatalog, abilities: loadChampionAbilities}): () => void {
  let active = true;
  void (async () => {
    try {
      const catalog = await loaders.catalog(locale);
      if (!active) return;
      receive({catalog, abilities: [], error: false});
      // Une fiche de compétences absente ne masque pas les runes et les objets.
      const abilities = await loaders.abilities(locale, championId, catalog.version,catalog.snapshotId).catch(() => []);
      if (active) receive({catalog, abilities, error: false});
    } catch {
      if (active) receive({catalog: null, abilities: [], error: true});
    }
  })();
  return () => { active = false; };
}
