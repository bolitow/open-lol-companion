# Recette du référentiel #61 — 1er octobre 2026

Recette exécutée sur macOS, Rust en mode release et PostgreSQL 17 dans un conteneur
séparé. Le cache public Data Dragon 16.19.1/16.18.1 a été copié en lecture seule
depuis la recette #18 vers une base jetable ; CommunityDragon a été téléchargé
avec le transport de production. Aucune partie, donnée de joueur ou clé Riot n’a
été copiée. La recette ne modifie pas les bases de collecte existantes.

Le [rapport machine](2026-10-01-catalogue-couverture.json) conserve les identifiants
de publication, compteurs par source/famille/langue, tailles et résultats HTTP.
Le manifeste API donne l’inventaire détaillé de toutes les branches par source.

## Volumes réellement publiés

Les valeurs « par langue » sont disponibles en français et en anglais. Les nombres
incluent les entrées historiques, réservées et variantes présentes dans les sources :
ils ne sont pas un nombre d’objets achetables ou de sorts sélectionnables dans chaque mode.

| Famille | 16.18.1 | 16.19.1 |
| --- | ---: | ---: |
| Objets distincts par langue | 868 | 870 |
| Champions standard par langue | 173 | 173 |
| Champions Classic par langue | 68 | 72 |
| Compétences standard par langue | 865 | 865 |
| Compétences Classic par langue | 340 | 360 |
| Runes et arbres par langue | 98 | 98 |
| Fragments par langue | 10 | 10 |
| Sorts d’invocateur par langue | 34 | 34 |
| Icônes de profil par langue | 5 042 | 5 050 |
| Cartes Data Dragon par langue | 5 | 5 |
| Catalogues globaux | 17 cartes, 99 files, 22 modes, 3 types | mêmes effectifs |
| Total de fiches (langues/contextes inclus) | 15 147 | 15 215 |
| Sources archivées par publication | 508 | 516 |

Les 1 020 sources distinctes des deux publications occupent 13,40 Mio ; les quatre
catalogues globaux sont partagés. Les fiches occupent 74,88 Mio, et l’ensemble des
cinq tables de catalogue 89,09 Mio (index/TOAST inclus, cache brut #18 exclu).
Aucune duplication des fiches n’est ajoutée aux agrégats de parties.

## Couverture et limites observées

| Mesure de couverture des fiches | 16.18.1 | 16.19.1 |
| --- | ---: | ---: |
| Feuilles source rattachées aux fiches | 621 805 | 627 535 |
| Feuilles normalisées | 204 349 | 205 877 |
| Feuilles conservées non normalisées | 417 456 | 421 658 |
| Fiches avec au moins une anomalie explicite | 2 999 | 3 033 |

Ces totaux comptent les feuilles BIN dans chaque langue de fiche. Pour les feuilles
source uniques, consulter `unique_source_leaves_by_provider` du rapport et
`source_inventory` du manifeste. L’inventaire couvre aussi les branches auxiliaires
qui ne produisent aucune fiche. Sur 16.19 : 1 966 branches BIN auxiliaires, dont
726 définitions de visuels, 642 objets de sorts, 378 groupes, 139 modificateurs,
10 matériaux et 70 métadonnées de conseils. Elles restent archivées, sans être
présentées comme des mécaniques interprétées.

Le paramètre de l’objet 3078 `mAbilityHasteMod` donne bien **15 points** avec statut
`verified`, provenance `/Items~13078/mAbilityHasteMod`. Les paramètres d’effets sont
présents ; les calculs complexes restent `unsupported`. Les fragments ont noms,
icônes et emplacements, mais leurs effets textuels ne deviennent pas des chiffres
inventés. Les unités non démontrées restent nulles.

## Mesures

Mesures locales indicatives, sans garantie de débit en production ; d’autres
vérifications tournaient sur la machine. Le RSS est celui du processus collecteur.

| Opération | Temps | Pic RSS |
| --- | ---: | ---: |
| Projection/publication des deux patches avec téléchargement CommunityDragon | 30,38 s | 883,09 Mio |
| Réexécution depuis les caches, mêmes manifestes | 4,07 s | 817,52 Mio |

Le catalogue entier et les sources de chaque patch sont encore traités en mémoire.
La pagination de l’API ne supprime pas ce coût côté collecteur : prévoir environ
1 Gio pour cette recette et de la marge si les exports grossissent.

Réponses HTTP mesurées en local, sans compression :

| Réponse | Octets | Temps observé |
| --- | ---: | ---: |
| Manifeste 16.19.1 | 550 158 | 36,01 ms |
| Page de 50 objets FR | 733 143 | 20,89 ms |
| Page maximale de 200 objets EN | 2 708 770 | 37,59 ms |
| Détail de l’objet 3078 | 19 206 | 2,64 ms |
| Fragments FR (10) | 27 647 | 2,10 ms |
| Diff objets, première page | 4 427 | 39,19 ms |

Les fiches incluent une provenance et une couverture détaillées : elles sont plus
volumineuses qu’une simple liste de noms. La borne de 2 Mio par fiche et 200 fiches
par page n’est pas un budget global de réponse de 2 Mio. Le consommateur peut réduire
`limit` ; un format compact relève d’une évolution future.

## Résultats de validation

- `pnpm test` avec `OLC_TEST_DATABASE_URL` dans l’espace de travail initial :
  **318 tests Rust et 10 TypeScript verts**.
  Les deux tests de recette explicitement ignorés par défaut ont aussi été exécutés
  séparément avec leurs données requises, tous deux verts.
- Validation de livraison dans le worktree isolé, basé sur la PR API #60 et sans
  les imports #14–16 : **273 tests Rust et 7 TypeScript verts**, PostgreSQL réel
  inclus ; `pnpm lint` et `git diff --check` également verts.
- `pnpm lint` : typage, formatage Rust et Clippy workspace à zéro avertissement.
- TDD observé pour projection, enrichissement, inventaire, filtres/diff, publication,
  contrôle des pointeurs, reconstruction historique et représentation des nombres.
- HTTP réel : manifeste/liste/détail/diff, FR/EN, Classic et `und/global`, filtres
  accélération/prix/catégories, 400/404, listes vides et routes statiques historiques.
  Les 10 requêtes de la recette ont toutes un ETag et une revalidation 304 sans corps.
- Deux reconstructions hors ligne strictement identiques aux manifestes publiés,
  avec recomputation de toutes les empreintes des sources archivées.
- Nouvelle exécution normale : mêmes identifiants et mêmes manifestes, aucun doublon.
- Tests PostgreSQL : échec au COMMIT sans état partiel, révision historique conservée,
  replay d’un ancien mapping sans recul de tête, lecture cohérente sous 80 changements
  de publication et 40 lectures concurrentes.
- Revue stricte indépendante : problèmes de provenance du diff, ordre des recettes,
  unités, couverture et recul de tête corrigés ; verdict **OK avec réserves** de
  volume/mémoire et de validation native Windows.

La recette réelle a détecté une perte de 334 signes `-0.0` dans le BIN 16.19 après
stockage JSONB. Les sources sont désormais écrites comme texte converti explicitement
en PostgreSQL `JSON`, et Serde utilise `float_roundtrip`. Le test de non-régression
vérifie le signe du zéro, les décimales et la republication ; le replay réel confirme
les empreintes. Les fiches et manifestes restent en JSONB pour les recherches.

## Reproduire

Après configuration de `DATABASE_URL` vers une base de recette isolée :

```sh
cargo run -p olc-collector --release -- sync-static --patch-count 2
cargo run -p olc-collector --release -- catalog --patch-count 2 --json
cargo run -p olc-collector --release -- catalog --rebuild <publication_id> --json
pnpm test
pnpm lint
```

Configurer également `OLC_TEST_DATABASE_URL` pour les tests PostgreSQL normaux.
Le test réel explicite attend `OLC_CATALOG_RECIPE_DATABASE_URL` vers la base isolée
contenant les deux publications :

```sh
cargo test -p olc-collector --test catalog reconstruction_reelle -- --ignored --nocapture
```

L’autre recette explicite lit les huit exports CommunityDragon locaux, nommés
comme leurs clés (`/` remplacé par `-`), dans `OLC_CATALOG_FIXTURE_DIR` :

```sh
cargo test -p olc-collector --lib real_public_exports_project_all_items_and_shards -- --ignored --nocapture
```

La recette manuelle sur données réelles a été exécutée sur macOS ; son équivalent
Windows reste à faire. Le code utilise les mêmes transports TLS Rust, commandes CLI
et requêtes SQL sur les deux OS. La validation CI Linux/Windows/macOS est suivie
dans la PR de livraison et doit être verte avant fusion. Aucun déploiement ou
nouveau pipeline statistique aval n’est inclus.
