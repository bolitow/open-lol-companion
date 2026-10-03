# Référentiel du jeu — #61, sous-ticket de #18

Le collecteur transforme les sources publiques en fiches versionnées, consommables
par l’API #19. Il enrichit notamment les statistiques d’objets absentes du dictionnaire
Data Dragon. Les fiches gardent leurs sources, leurs unités et leurs limites.

## Démonstrations vidéo des compétences — #4

La fiche Compétences ouvre un aperçu vidéo au survol ou au focus maintenu 300 ms
sur une icône. Les statistiques restent visibles dans la carte ; le clic ouvre
une fiche agrandie, avec vidéo et paramètres côte à côte. Les flèches et les touches
gauche/droite parcourent les cinq compétences du même champion, en boucle ; les
vignettes permettent aussi l’accès direct. Le clic extérieur ou Échap ferme la fiche
et rend le focus sans rouvrir l’aperçu, en conservant la sélection et les filtres.
Chaque changement met en pause le lecteur précédent. Depuis l’icône, Tab donne accès aux
commandes de l’aperçu ; Échap ferme celui-ci. Passer dans le panneau conserve la
lecture. Une seule démonstration est ouverte à la fois, muette et en boucle.
Fermer un aperçu, défiler ou quitter la fenêtre arrête cet aperçu. Le réglage de mouvements réduits désactive la lecture
automatique ; un bouton Lecture reste disponible, également si le moteur refuse
l’autoplay. Erreur de réseau, format non pris en charge ou chargement bloqué donnent
un état indisponible avec nouvelle tentative. Une référence absente n’est pas inventée.

`apps/desktop/public/game-data/ability-videos.json` est un manifeste léger séparé
des statistiques normalisées : identifiant `champion:passive|Q|W|E|R`, page Riot,
date de vérification, dimensions, poster et source MP4 ou WebM. Il est lu à l’ouverture
d’une fiche champion, puis gardé en mémoire. Cette fiche précharge au maximum ses cinq
vidéos disponibles, sans les lire. Le même élément vidéo et son tampon passent de
l’aperçu à la vue agrandie ; fermer celle-ci le met en pause sans jeter le préchargement.
Changer de champion, fermer sa fiche ou quitter Champions libère toutes ses sources
(`pause`, retrait de `src`, `load`) et ignore les réponses tardives du manifeste.
Un préchargement sans état lisible après 12 secondes est abandonné ; consulter le sort
permet de retenter. Les mouvements réduits conservent le préchargement silencieux,
mais imposent toujours le bouton Lecture.

Le média provient du CDN public Riot, sans clé ni compte et sans intégration des vidéos
à l’installeur. [`preload="auto"`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement/preload)
est une indication au moteur : le réseau peut encore ralentir une première lecture,
et cinq lecteurs ne constituent pas un plafond d’octets. Les ressources détenues par
l’app sont libérées à la fermeture ; le cache HTTP propre à la WebView n’est pas effacé.
Aucun cache hors ligne persistant n’est garanti.

Dans la fiche agrandie, récupération, ressource et portée sont alignées par ligne,
séparément des effets et ratios. Le fonctionnement reste visible intégralement même
quand les effets chiffrés manquent, sans dépliant ni texte tronqué. Les longs sorts
défilent à l’intérieur de la modale sur petite fenêtre ; la page reste fixe.

Les libellés et montants de dégâts ont une couleur par type : physiques orange,
magiques violets, bruts dans la couleur du texte (blanc en sombre, sombre en clair).
Le type suit le texte source FR/EN, jamais le ratio : les coefficients AD/AP gardent
leur couleur propre, même dans une formule de dégâts bruts.

Pour rafraîchir les références pendant la préparation d’une publication :

```sh
node apps/desktop/scripts/build-ability-videos.mjs
```

Node 20+ suffit. Le générateur lit les [pages officielles des champions](https://www.leagueoflegends.com/en-us/champions/),
extrait leurs références publiées (aucune URL de vidéo devinée), contrôle le roster
contre `champions.json`, puis vérifie les en-têtes MP4, WebM de repli et posters.
Les 404/410 deviennent des absences explicites ; une panne, un format inconnu ou
un catalogue incomplet bloque le remplacement du manifeste précédent. L’écriture
est atomique. Aucune vidéo n’est téléchargée par cette commande.

Vérification du 2 octobre 2026 : 173 champions, 865 références, 846 MP4 et 10 WebM
accessibles ; 9 absences (passifs Vladimir, Rammus, Kassadin, Karma, Wukong,
Heimerdinger, Ezreal, Rek’Sai et E d’Aphelios). Ces médias pédagogiques officiels
ne sont pas versionnés par patch et leur accessibilité ne garantit pas leur
actualité. Les champions à transformations/variantes gardent les cinq groupes
publiés par Riot ; aucune vidéo par sous-sort n’est promise. Attribution Riot
visible dans le lecteur ; mentions légales existantes conservées. Aucun accès
aux informations cachées, aucune action League et aucun secret.

## Effets chiffrés dans le desktop — #4 / #61

Diagnostic du 2 octobre 2026, catalogue 16.19.1 : les 865 compétences ont `stats:{}`
et `effects:[]`. Le normaliseur expose cooldown/coût/portée, mais conserve les effets
Data Dragon comme non interprétés. Les 692 actifs contiennent des variables dans leur
tooltip ; 683 restent incomplets après retrait du seul marqueur technique final.
Ahri Q, Annie Q et Ezreal Q ont notamment des tableaux `effect` à zéro, `vars:[]` et
`datavalues:{}` : utiliser leurs zéros comme dégâts serait faux.

Le desktop dispose maintenant d’un complément statique distinct,
`apps/desktop/public/game-data/ability-effects.json`, sans changer les contrats API,
Rust ou `@olc/shared`. Il relie les tooltips existants aux BIN publics CommunityDragon
du **même patch**. La racine `CharacterRecords/Root`, le slot et `mScriptName` doivent
correspondre à l’identifiant technique Data Dragon. Chaque entrée conserve l’URL,
le chemin du sort et le SHA-256 de la source. Le front refuse un autre patch, une
autre langue, le mode Classic, une identité/description modifiée ou un champ conflictuel.

Interprétation volontairement limitée : données nommées, constantes, valeurs d’effet
explicitement référencées, sommes, AP/AD total/AD bonus et multiplicateurs scalaires.
Les rangs commencent à 1 ; les valeurs identiques sont regroupées. Les formules restent
symboliques : `(base par rang + ratio AP/AD)`, jamais des dégâts calculés avec un état
de partie supposé. Aucun `eval`. Un terme, enum, multiplicateur, condition ou champ
inconnu invalide **toute** la formule concernée. Une absence n’est jamais zéro.

Les défauts `mStat=0` (AP), `mStatFormula=0` (total) et la formule 2 (bonus) ont été
croisés avec le [dump constructeur exact 16.19](https://raw.githubusercontent.com/LeagueToolkit/lol-meta-classes/main/dumps/16.19.8217343.json)
et les données du patch : [Aatrox Q/W](https://raw.communitydragon.org/16.19/game/data/characters/aatrox/aatrox.bin.json)
référencent explicitement `QTotalADRatio`/`WTotalADRatio`, [Talon](https://raw.communitydragon.org/16.19/game/data/characters/talon/talon.bin.json)
`BonusADRatio`. L’ancien mapping d’enums de calcrev/Hextechdocs ne correspond pas à ces
données. `UseNewStats`, les autres indices de stat et les autres formules sont refusés.

Couverture initiale, identique FR/EN : **644 compétences avec au moins une formule**, dont
**366 tooltips sans variable restante**. Cela ne garantit pas toutes les mécaniques du
sort. Les 173 passifs, sous-sorts/formes, calculs conditionnels, interpolations de niveau
et références à d’autres sorts ne sont pas encore interprétés. Le panneau indique
les effets indisponibles ; un tiret explicite remplace chaque variable non résolue.
Les montants et ratios ont leurs couleurs et pictogrammes. La carte donne un aperçu
compact, avec accès à tous les effets dans la fiche agrandie. Les descriptions Riot
restent du texte React, jamais du HTML injecté ; leurs marqueurs d’icônes sont retirés.

Régénération hors ligne depuis des BIN préalablement archivés et indexés :

```sh
node apps/desktop/scripts/build-ability-effects.mjs <dossier-sources>
```

Le dossier contient `<identifiant-technique-en-minuscules>.bin.json` et `index.json` :
`entries: [{champion_id, url, sha256, status:"ok"}]`. Les 173 identités, URLs du patch,
empreintes et versions FR/EN doivent correspondre avant le remplacement atomique.
Les sources utilisées pour cette passe sont conservées dans
`work/ability-effects-2026-10-02/sources/`. La commande n’interroge pas le réseau et ne
remplace pas le catalogue du collecteur. **Suite #61** : intégrer ce complément dans
le pipeline publié, enrichir les formes/passifs et produire une couverture par effet
avec tests représentatifs ; pas de promesse de couverture totale à ce stade.

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
| Winrate champion | #18 : victoires/participations ×100, patch/plateforme/file/rôle/rang | Seuil défaut 100 ; intervalle temporel `[from_ms,to_ms)` configurable au calcul ; rang observé récent, pas historique |
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
