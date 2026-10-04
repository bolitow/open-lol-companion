# Référentiel du jeu — #61, sous-ticket de #18

Le collecteur transforme les sources publiques en fiches versionnées, consommables
par l’API #19. Il enrichit notamment les statistiques d’objets absentes du dictionnaire
Data Dragon. Les fiches gardent leurs sources, leurs unités et leurs limites.

## Exploitation

PostgreSQL et `DATABASE_URL` suffisent ; aucune clé Riot ni client LoL n’est nécessaire.
Les commandes fonctionnent dans le terminal macOS/Linux et dans PowerShell après
configuration des variables d’environnement décrites dans le README du collecteur.

```sh
cargo run -p olc-collector --release -- sync-static --patch-count 2
cargo run -p olc-collector --release -- catalog --patch-count 2 --json
cargo run -p olc-collector --release -- catalog --version 16.19.1 --refresh --json
cargo run -p olc-collector --release -- catalog --rebuild <publication_id> --json
```

Les versions d’exemple ne sont pas codées en dur. Par défaut, `catalog` lit les deux
versions du manifeste statique en cache. `sync-static` doit avoir réussi auparavant.
`--refresh` revérifie CommunityDragon ; pour rafraîchir aussi Data Dragon, lancer
`sync-static --refresh` avant. Sans refresh, le complément déjà archivé pour cette
version est réutilisé. `--rebuild` repart des sources exactes archivées, sans réseau,
même après remplacement ou suppression du cache brut Data Dragon.

Politique explicite : `--community required` par défaut ; complément absent ou
incohérent = échec, ancienne publication conservée. `optional` autorise uniquement
une panne réseau à publier Data Dragon seul, avec `degraded=true` et
`community_unavailable`. Un document invalide n’est jamais dégradé silencieusement.
`off` publie volontairement Data Dragon seul avec `community_disabled`.
Le code de retour 3 indique une annulation ponctuelle. Aucun nouveau service continu
n’est installé : lancer `catalog` après la synchronisation voulue.

## Familles et champs

| Famille (`kind`) | Données exploitables | Données brutes ou limites |
| --- | --- | --- |
| `item` | Noms/descriptions FR/EN, icônes, prix total/combinaison/revente, recettes, cartes, catégories, achat/magasin, piles, conditions, statistiques explicites BIN | Disponibilité par carte ≠ disponibilité par file ; paramètres conditionnels séparés ; IDs réservés conservés ; calculs moteur et variantes de paramètres non exécutés |
| `champion` | ID numérique/technique, titre, ressource, attributs de base et croissance, classes | Standard et Classic ont des espaces distincts ; attributs hors dictionnaire restent bruts |
| `ability` | Passif/Q/W/E/R, coût, portée, délai et valeurs structurées par rang | Identifiant `<champion_id>:Q`… ; placeholders, `effect`, `vars`, `leveltip` ne deviennent pas des formules vérifiées |
| `rune` | Arbres, emplacements, textes et icônes, identifiants supplémentaires CommunityDragon | `rune_kind` distingue arbre/rune Data Dragon ; présence dans les styles exposée ; effets textuels non convertis en ratios |
| `rune_shard` | Fragments identifiés par `kStatMod` ou chemin officiel `StatMods`, emplacements et descriptions FR/EN | Anciens fragments conservés ; paramètres numériques structurés manquants signalés |
| `summoner_spell` | IDs, descriptions, délais, portée/coût, modes et niveau requis disponibles | Valeurs absentes non imputées |
| `map`, `queue`, `mode`, `game_type` | Identifiants et libellés publiés | Catalogues globaux `namespace=global`, `locale=und`, `version_scope=unversioned` : observés à une date, sans fausse attribution historique au patch |
| `profile_icon` | IDs et icônes par version | Pas de nom inventé |

Statistiques objets normalisées lorsqu’elles existent : PV, mana, AD, AP, armure,
résistance magique, vitesse d’attaque/déplacement, critique, vol de vie, accélération,
régénérations, pénétrations, létalité, omnivampirisme, ténacité, puissance de soins et
boucliers, résistance aux ralentissements, portée. Les clés précises sont dans
`project_item`, `project_champion` et `community::STATS`.
Une valeur `ratio=0.3` correspond à 30 %, tandis qu’une valeur
`percent_per_level=2.5` est exprimée en points de pourcentage par niveau.
La régénération plate de PV des objets est exprimée par seconde, les régénérations
des champions par cinq secondes ; elles ne sont pas additionnées sans conversion.
Pour `FlatMPRegenMod` (absent des sources vérifiées), l’unité reste `null`.
Les paramètres `mDataValues` d’effets gardent leur nom et leur nombre ; leur unité
reste `null` quand elle n’est pas démontrée. Aucune addition de ces paramètres n’est
présentée comme un simulateur de dégâts ou une règle de cumul.

## Contrat, provenance et couverture

Contrats Rust dans `catalog/model.rs`, miroirs TypeScript dans
[`catalog.ts`](../packages/shared/src/catalog.ts). Une fiche est identifiée par
publication + kind + namespace + locale + id. Une `CatalogValue` porte `value`,
`unit`, `status` et des références `{source_id,pointer}` (JSON Pointer).
Le manifeste résout chaque source vers son fournisseur, URL, version exacte et date
d’observation. `observed_at` garde la date PostgreSQL pour le cache Data Dragon et
les millisecondes Unix sous forme de chaîne pour CommunityDragon. Les empreintes SHA-256 portent sur le JSON canonique et son identité,
indépendamment de la date de relecture ; ce ne sont pas les empreintes des octets HTTP.

| Statut | Signification |
| --- | --- |
| `verified` | Valeur structurée et type reconnus dans une source ; ne garantit pas une mécanique calculable |
| `derived` | Transformation déterministe documentée : URL d’icône, appartenance à un arbre, ID/slot |
| `descriptive` | Texte rendu en texte seul, sans interprétation mécanique |
| `missing` | Champ explicitement nul ; un champ absent peut aussi être omis de la fiche |
| `unsupported` | Valeur présente mais forme ou calcul non interprété |
| `conflict` | Sources contradictoires : candidats et provenances préservés, aucune valeur gagnante inventée |

Zéro, faux, nul, chaîne vide et champ absent restent distincts. La comparaison des
floats BIN tolère l’erreur de représentation 32 bits (tolérance 1e-7 × max(|a|, |b|, 1)).
Les catalogues à langues incohérentes ou identités invalides sont refusés ; les
écarts de valeurs FR/EN restent visibles dans `coverage.issues`.
Les noms/descriptions sont du texte : jamais de `innerHTML`/HTML arbitraire côté client.
Les valeurs brutes non interprétées doivent elles aussi être affichées comme texte.

`coverage` par fiche compte les feuilles source, feuilles normalisées et chemins
non normalisés (`source_id:JSONPointer`), ainsi que les anomalies (placeholder, recette cyclique/inconnue,
contradiction…). Les tableaux/objets vides et `null` comptent chacun pour une feuille.
Les sorts/runes publiés séparément ne sont pas comptés deux fois dans leur parent.
Les enveloppes Data Dragon sont comptées une fois sur la première fiche de leur
source. Les données BIN communes aux deux langues contribuent à chaque fiche :
les totaux de fiches ne sont donc pas des nombres de champs source uniques.

`source_inventory` complète cette mesure : toutes les branches et feuilles de
chaque document archivé sont comptées une seule fois. Les branches sans lien vers
une valeur de fiche sont listées dans `raw_branches`, avec une raison. Cela inclut
les ressources auxiliaires BIN (visuels, objets de sorts, groupes/modificateurs,
conseils) gardées brutes. Une branche liée à une fiche ne signifie pas que toutes
ses feuilles sont normalisées ; consulter aussi la couverture de la fiche.

## API

Les routes publiques de référentiel suivent le cache/ETag des données statiques ;
les autres règles d’accès restent celles du [contrat API](../services/api/README.md).

```text
GET /v1/catalog/16.19.1/manifest
GET /v1/catalog/16.19.1/fr_FR/item?limit=50&offset=0
GET /v1/catalog/16.19.1/fr_FR/item/3078
GET /v1/catalog/16.19.1/fr_FR/item?stat=ability_haste&min_stat=15&map=11
GET /v1/catalog/16.19.1/fr_FR/champion?namespace=classic
GET /v1/catalog/16.19.1/und/queue?namespace=global
GET /v1/catalog-diff?from=<publication_id>&to=<publication_id>&locale=fr_FR&kind=item
```

Filtres liste : `search` littéral sur nom, `min_price`, `max_price`, `purchasable`,
`category`, `map`, `stat`, `min_stat`, `coverage=complete|incomplete`, `namespace`.
`offset` entre 0 et 1 000 000, `limit` entre 1 et 200 (défaut 50), ordre ID stable.
Les filtres numériques/booléens excluent missing/unsupported/conflict et les absences.
Sans filtre, ces fiches restent visibles. `complete` signifie zéro anomalie, zéro
champ non normalisé et aucun statut missing/unsupported/conflict ; une description
seule ne devient pas une formule même si elle est correctement couverte.
Version/publication ou fiche absente : 404 ; paramètres invalides : 400.
Une page, son total filtré et sa publication viennent d’une seule requête SQL.
Le diff distingue ajout/retrait, champs, stats, effets, texte, source et couverture.
Une nouvelle source ou description ne démontre pas un changement d’équilibrage.
Une version peut être corrigée : cache revalidable (ETag/304), jamais `immutable`.

## Stockage et reprise

Migration `0008_game_catalog.sql` : sources immuables en `game_catalog_sources`
(colonne PostgreSQL `JSON`, pour préserver notamment le signe de `-0.0` ; lecture
Serde avec `float_roundtrip`),
références dans `game_catalog_source_refs`, manifestes dans
`game_catalog_publications`, fiches dans `game_catalog_entries`, dernière publication
par version dans `game_catalog_current`. Insertion des sources/fiches/manifeste et
changement de tête atomiques, protégés par verrou PostgreSQL. L’échec ne supprime
pas une publication antérieure ; un résultat de COMMIT incertain n’est pas annoncé
comme un succès. Rejouer une publication historique ne rembobine pas la tête, y compris avec un
nouveau normaliseur ; seule une reconstruction de la tête courante peut la remplacer.
L’atomicité porte sur chaque patch ; si le second patch échoue, le premier peut
déjà être publié. Chaque fiche est bornée à 2 Mio ; elles ne sont jamais fusionnées en un seul JSONB.

Le transport CommunityDragon est séparé : hôte HTTPS autorisé explicitement,
pas de redirection ni authentification, requêtes séquentielles, timeout 35 s,
32 Mio maximum par document. Un 429 arrête la tentative sans retry prématuré.
Les métadonnées de build sont relues avant/après pour refuser un mélange pendant
une publication amont. Un alias de patch CommunityDragon reste mutable ; l’archive
locale par contenu est donc nécessaire à la reproductibilité.
Les archives référencées sont conservées ; aucune purge automatique n’est ajoutée.
Toute évolution du mapping ou du rapport doit incrémenter `NORMALIZER_VERSION`
pour produire une nouvelle publication au lieu de réutiliser une ancienne empreinte.

## Matrice des usages statistiques

Le catalogue rejoint les agrégats par ID et patch/contexte. Le patch d’agrégat
`16.19` doit être résolu vers une version publiée `16.19.x` ; ne pas joindre
aveuglément avec le dernier patch. Un ID inconnu reste affichable comme inconnu.
Aucune fiche descriptive n’est recopiée dans les groupes statistiques.

| Usage | Disponibilité et définition | Dépendance/limite |
| --- | --- | --- |
| Winrate champion | #18 : victoires/participations ×100, patch/plateforme/file/rôle/rang | Seuil défaut 100 ; intervalle temporel `[from_ms,to_ms)` configurable au calcul ; rang observé le plus proche de la partie (écart ≤ 168 h par défaut, #80), pas un MMR |
| Pickrate | #18 : participations champion / toutes participations du même groupe ×100 | `ALL` et rangs se recouvrent : ne pas les additionner ; échantillon collecté, pas population Riot entière |
| Banrate | #18 : drafts bannissant le champion / drafts complètes ×100 | Pas de faux rang/rôle du ban ; seuil de drafts |
| Builds, objets, runes, sorts | #18 : effectif, population éligible de chaque catégorie, taux de sélection/victoire | Variantes limitées aux20 plus populaires ; seuil ; achats tardifs et durée créent des biais ; pas de causalité |
| Compétences/achats | #18 : ordre des points Q/W/E/R, temps moyen ; achats nets hors annulations ambiguës | Timeline absente signalée, inventaire final distinct de l’ordre d’achat |
| Stats descriptives d’objet | #61 : valeurs explicites, recettes, paramètres, provenance par version | Des formules complexes restent non interprétées ; puissance théorique ≠ winrate |
| Dégâts/minute, CS/minute, vision, objectifs, écarts or/XP | Données Match-v5/timeline déjà archivables ; indicateurs supplémentaires à produire | #41/#26 : définitions, dénominateurs et couverture à fixer avant calcul ; APM exhaustif non promis |
| Matchups, synergies, maîtrise et suggestions | Référentiel prêt pour les jointures | #39/#42 : agrégations/modèles supplémentaires à construire |
| Historique joueur, rang et LP | #19 : lecture des parties/rangs observés | #20 : suivi temporel ; aucun passé inventé |
| Live, collection, pros, leaderboards, esports | Référentiel réutilisable | #23, #47–50 : sources/contrats et fonctionnalités propres |

Les taux d’objets Arena et d’augments ne sont pas produits. Les données statiques
restent séparées de ces mesures interdites par la politique Riot. Aucun accès au
processus du jeu, donnée cachée en partie, automatisation de décision ou contenu
DPM copié. Mention légale Riot dans le README du collecteur.

Sources : [Data Dragon](https://developer.riotgames.com/docs/lol#data-dragon),
[CommunityDragon exports versionnés](https://raw.communitydragon.org/16.19/),
[documentation CommunityDragon](https://github.com/CommunityDragon/Docs/blob/master/assets.md),
[politique Riot](https://developer.riotgames.com/docs/lol#game-policy).
La [recette réelle](recettes/2026-10-01-catalogue-jeu.md) détaille les mesures et limites OS.
