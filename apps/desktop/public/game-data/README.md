# Catalogue de cartes de champions

Données officielles Data Dragon, version 16.19.1, extraites le 1er octobre 2026 de `https://ddragon.leagueoflegends.com/cdn/16.19.1/data/{fr_FR|en_US}/champion.json`. `champions.json` ne conserve que l'identifiant numérique, la clé Riot et les noms officiels FR/EN.

Les 173 cartes de base viennent de `https://ddragon.leagueoflegends.com/cdn/img/champion/loading/{key}_0.jpg` (URL Riot non versionnée, capture à cette date). Les cartes déjà présentes dans le prototype ont été réutilisées. Aucune récupération réseau à l'affichage ; nouveau champion absent du catalogue → libellé de repli, sans panne de la draft. La mise à jour automatique du catalogue relève de #61.

Ces visuels restent la propriété de Riot Games ; ils ne sont pas couverts par la licence du code. Mention de non-affiliation présente dans l'application. Documentation : https://developer.riotgames.com/docs/lol#data-dragon

## Runes, objets, champions et sorts — catalogue local #13

`catalog/` réutilise le normaliseur de #61 : Data Dragon 16.19.1 et complément CommunityDragon 16.19. Il contient 1 496 fiches par langue : 316 objets (carte 11 et fermeture des recettes), 98 runes/arbres, 10 fragments référencés, 173 champions standard, 865 compétences (passif et Q/W/E/R par champion) et 34 sorts d'invocateur. Les variantes `classic` sont exclues. Seuls les fragments actuellement listés dans les arbres sont proposés. Le front filtre les objets explicitement achetables et en boutique ; la carte seule ne garantit pas leur disponibilité dans chaque file, mode ou champion. Les modes disponibles pour chaque sort restent dans `fields.modes` : la présence au catalogue ne suffit pas à le rendre sélectionnable pour une partie. Il s'agit d'un catalogue, pas d'un build conseillé ni de la boutique d'une partie.

Les 1 477 PNG sont locaux ; l’ensemble représente 57 085 679 octets après la reprise ciblée du 6 octobre 2026 (#107). Chaque JSON racine ne contient que les 458 fiches objets/runes/fragments/sorts : 5 730 340 octets en FR, 5 634 770 en EN. Les fiches complètes des champions et leurs compétences sont chargées à la demande depuis `champions/{id}/{locale}.json` : six fiches par fichier (un champion, son passif et Q/W/E/R), environ 88 ko en moyenne, de 68 à 110 ko. Le petit index `/game-data/champions.json` suffit à afficher la liste des noms FR/EN sans charger ces détails.

**Évolution de l'export (#116).** La commande d'export garde désormais les objets disponibles sur la Faille (carte 11), l'ARAM (carte 12) ou l'Arena (carte 30), plus la fermeture de leurs recettes ; la disponibilité de chaque objet par carte reste dans `fields.maps`. `item_filter` du manifeste donne `mode` (`maps_11_12_30_with_components`, ou `all_items` si une carte exportée est absente ou illisible), `maps` et `by_map`, le nombre d'objets conservés par carte. Les surcharges de valeurs par mode sont publiées comme effets `cdragon_parameters:{mode}`. **La sélection d’objets actuellement versionnée reste celle de l’export précédent (carte 11 seule) ; la reprise ciblée des tooltips #107 ne la régénère pas** ; la republier est une étape explicite. Le front filtre toujours sur la carte 11 pour la Faille.

`manifest.json`, schéma 2, donne les empreintes et tailles des racines dans `locales` et des 346 documents de champion dans `champions[id][locale]`. `locales[locale].records` compte les 458 fiches racine ; `total_records` compte les 1 496 fiches, détails compris. `sources.json` conserve les 362 sources versionnées et la correspondance des chemins locaux avec les URL d'origine. L'export télécharge les fiches individuelles `data/{locale}/champion/{key}.json`, l'index `champion.json` et `summoner.json` depuis Data Dragon, en plus des sources déjà utilisées pour les runes et objets. Les documents et icônes sont acquis par lots de quatre, avec taille maximale et hôtes autorisés ; toutes les acquisitions réussissent avant l'écriture de la sortie.

Les valeurs manquantes ou conflictuelles ne deviennent pas zéro. Les descriptions et infobulles du normaliseur sont du texte brut : ne jamais les interpréter comme HTML. Les variables non résolues doivent rester masquées à l'affichage. Le patch du catalogue reste visible. Il n'est pas automatiquement comparé au patch du client ni actualisé dans ce lot.

### Contrat pour les consommateurs

Chaque document, racine ou détail, a la forme `{ "version": "16.19.1", "records": [...] }`. Le schéma des fiches reste `CatalogRecord` du normaliseur, avec `namespace: "standard"` et `locale: "fr_FR"` ou `"en_US"`. Les champs gardent `value`, `unit`, `status` et leurs références de provenance `sources`. Par exemple, `/game-data/catalog/champions/103/fr_FR.json` contient Ahri et ses cinq compétences ; aucune fiche `champion` ou `ability` ne reste dans la racine.

| `kind` | `id` | Champs utiles |
| --- | --- | --- |
| `champion` | Identifiant numérique en chaîne, par exemple `"103"` | `technical_id` (`Ahri`), `name`, `icon` local, caractéristiques normalisées |
| `ability` | `"103:passive"`, `"103:Q"`, `"103:W"`, `"103:E"`, `"103:R"` | `fields.champion_id` et `fields.slot` dérivés de la fiche officielle, `description`, `fields.tooltip`, `fields.tooltip_segments` si tooltip présent, `cooldown`, `range`, `cost`, `max_rank` |
| `summoner_spell` | Identifiant numérique en chaîne, par exemple `"4"` | `technical_id` (`SummonerFlash`), `description`, `fields.tooltip`, `fields.tooltip_segments` si tooltip présent, `cooldown`, `modes`, `required_level` |

`name`, `description` et `icon` se lisent directement sur la fiche ; les autres valeurs citées sont dans `fields`. Pour présenter les compétences d'un champion, utiliser explicitement l'ordre Q/W/E/R et les IDs correspondants : le tri des fiches JSON est lexical et ne représente ni cet ordre, ni un ordre de montée conseillé. Les délais sont en secondes, les portées en unités du jeu, les coûts en points de ressource et les rangs en `rank`. Un passif peut ne pas définir ces valeurs.

**Augments (#118).** L'export garde aussi les fiches `augment` Arena et Mayhem (noms FR/EN, description pour les 225 augments couverts par `cdragon/arena`, absente pour les autres augments Mayhem, rareté, icône, modes, sans statistique), à la racine du document de chaque langue, icônes PNG comprises ; `loadCatalog` du desktop accepte ce kind (test `catalog.test.ts`), les autres familles restent lisibles et l'interface ne présente pas encore les augments. Après régénération, la racine ne compte donc plus 458 fiches mais 458 plus le nombre d'augments par langue (`locales[locale].records` du manifeste et les tailles de racine sont à relire à ce moment-là, pas avant). **La reprise ciblée #107 conserve la publication sans augments ; cette famille n’a pas été régénérée** ; la republier est une étape explicite.

Régénération (sortie absente ou vide, aucun écrasement) :

```sh
cargo run -p olc-collector --example export_desktop_catalog --release -- \
  --version 16.19.1 --output apps/desktop/public/game-data/catalog
```

Pour une mise à jour, exporter dans un dossier de travail vierge, contrôler le manifeste, puis remplacer explicitement l'ancien catalogue. Aucun PostgreSQL, jeton API ou accès réseau à l'exécution du front n'est nécessaire. Les valeurs `observed_at` datent l'acquisition ; la commande est reproductible, mais deux acquisitions ne donnent pas des métadonnées identiques octet pour octet.

### Reprise ciblée des tooltips — #107

Le 6 octobre 2026, le normaliseur 3 a reprojeté uniquement `tooltip_segments` depuis les 348 raw Data Dragon 16.19.1 originaux, dont les IDs ont été revérifiés. Les autres champs/records du normaliseur 1 sont conservés : ce n’est pas un export complet des nouvelles familles. Le périmètre est explicite dans `manifest.json` et `sources.json` (`reprojection`). Les 726 tooltips de compétences/sorts par langue gagnent des segments `{text, damage_type}` de statut `derived`, même provenance, dont la concaténation reste exactement `tooltip`. Aucun segment n’est inventé sur un passif sans tooltip.

Surcoût du seul champ : 995 454 octets (FR 522 594, EN 472 860) ; export complet +995 719 octets (+1,78 %). 316 objets, 98 runes, 10 fragments, 173 champions, 865 compétences, 34 sorts et 1 477 icônes conservés. Un nouveau snapshot scratch validé existe ; le snapshot d’origine et sa tête sont intacts, aucun service n’est déployé. [Procédure bornée de reprise du raw](../../../../docs/catalogue-desktop.md#reprise-ciblée-des-segments-107).
