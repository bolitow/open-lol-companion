# services/collector

Service Rust autonome de collecte Riot, synchronisation Data Dragon et agrégation
PostgreSQL (#17–18, sections 10.1–10.2 du [cahier des charges](../../docs/cahier-des-charges.md)).
Il fonctionne sans client LoL ni app desktop. L'API destinée à l'app reste le #19 ;
les imports côté client restent les #14–16. Le [service API #19](../api/README.md)
lit les instantanés et partage les quotas Riot PostgreSQL avec le collecteur.

## Configuration et démarrage

Prérequis : Rust stable et PostgreSQL 17. Pour une base locale :
`docker compose -f services/collector/docker-compose.yml up -d`.

Copier `.env.example` en `.env` dans le répertoire de lancement, ou à la racine du
projet, et renseigner les variables suivantes. Les fichiers `.env` sont ignorés par Git.
Les exemples ci-dessous sont exécutés depuis la racine du dépôt.

| Variable | Usage |
| --- | --- |
| `DATABASE_URL` | Connexion PostgreSQL du service |
| `RIOT_API_KEY` | Collecte authentifiée uniquement ; en-tête `X-Riot-Token`, jamais journalisé |
| `RUST_LOG` | Facultatif, `info` par défaut ; `warn,sqlx=error` pour une longue recette |
| `OLC_TEST_DATABASE_URL` | Tests : base d'administration permettant de créer et supprimer des bases jetables |

```sh
cargo run -p olc-collector --release -- migrate
cargo run -p olc-collector --release -- sync-static --json
cargo run -p olc-collector --release -- run --target 50 --collect-ranks
cargo run -p olc-collector --release -- report 1 --json
cargo run -p olc-collector --release -- aggregate --json
```

`sync-static` et `aggregate` n'utilisent aucune clé Riot. L'agrégation seule ne fait
aucun appel réseau. Par défaut, `run` et `aggregate` sélectionnent les deux patches
du dernier cache statique complet ; lancer `sync-static` auparavant. `--patches`
permet une sélection explicite ; `run --all-patches` et `aggregate --all-stored`
retirent ce filtre. Les anciennes exécutions reprennent leur périmètre initial.

## Collecte reprenable

Une exécution conserve une plateforme, une file et une fenêtre temporelle figées.
Les plateformes sont EUW1, EUN1, TR1, RU, ME1, NA1, BR1, LA1, LA2, KR, JP1, OC1,
SG2, TW2 et VN2, routées vers Europe, Americas, Asia ou SEA selon Riot.

1. Joueurs de départ via league-v4 : pages Iron à Diamond, listes dédiées pour
   Master, Grandmaster et Challenger (une seule strate pour chaque rang supérieur).
2. Historiques match-v5 paginés par 100, bornés dans le temps. `--queue 0` enlève
   les filtres de file/type et découvre les modes présents dans ces historiques.
3. Détails complets : contrôle de l'identifiant, plateforme, file demandée, fenêtre,
   patch et participants. Les modes de 1 à 64 participants sont conservables.
4. Timelines complètes, avec réutilisation des réponses déjà présentes.
5. Avec `--collect-ranks`, observations Solo et Flex de chaque participant disposant
   d'un PUUID, indépendamment du rang du joueur ayant servi à découvrir la partie.
   Elles ne sont demandées que pour les parties des files 420 et 440 qui ne sont pas
   des remakes : les autres modes n'ont jamais de rang applicable (`UNRANKED_MODE`).

| Option de `run` | Défaut | Effet |
| --- | --- | --- |
| `--platform` | EUW1 | Plateforme de classement et des parties |
| `--queue` | 420 | File exacte ; 0 = toutes les files retournées |
| `--patches` | Cache récent | Liste technique, par exemple `16.19,16.18` |
| `--all-patches` | Désactivé | Tous les patches présents dans la fenêtre |
| `--collect-ranks` | Désactivé | Rangs des participants, cache 24 h |
| `--target` | 1000 | Parties distinctes retenues, déjà présentes comprises |
| `--tiers` | GOLD,PLATINUM,EMERALD | Liste de rangs de découverte, Iron à Challenger |
| `--divisions` | I,II,III,IV | Divisions des rangs Iron à Diamond |
| `--window-days` | 14 | Fenêtre `[début, fin)`, figée au lancement |
| `--seeds-per-division` | 15 | Joueurs de départ par strate |
| `--max-matches-per-seed` | 10 | Historiques parcourus au maximum par joueur |
| `--call-budget` | 3000 | Budget total, reprises et nouvelles tentatives comprises |
| `--concurrency` | 2 | Requêtes simultanées, bornées à 16 |
| `--max-duration-mins` | Aucune | Durée d'un lancement, reprise possible |

`resume <id> --call-budget <total>` relève le budget ; `--retry-failed` remet les
travaux en échec en attente. `report <id> [--json]` donne le bilan. Une timeline
ayant reçu plusieurs 404 reste `unavailable` ; une panne réseau ne reçoit jamais
ce statut définitif.

Les seeds viennent du classement. Même en découvrant toutes les files, cet
échantillonnage ne couvre pas nécessairement les joueurs exclusivement non classés,
les débutants ou toutes les parties PvE. Une file absente du bilan signifie absence
de données acquises, pas absence d'activité. L'API ne fournit pas une liste exhaustive
de toutes les parties. La récupération des données brutes ne garantit pas que chaque
nouveau mode possède déjà une recette réelle des statistiques dérivées.

## Campagne de recette jusqu'à 24 heures

```sh
cargo run -p olc-collector --release -- campaign --hours 24
cargo run -p olc-collector --release -- campaign-resume 1
```

Par défaut : les 15 plateformes, toutes les files, les deux patches du cache,
Iron à Challenger, 5 seeds par strate, 100 historiques par seed, fenêtre de 28 jours,
observations des rangs activées, cible de 10 000 parties et budget de 100 000 appels
**par plateforme**, concurrence 4. Ce sont des plafonds/objectifs, pas une promesse
de volume atteint en 24 h. Les options `--platforms`, `--patches`,
`--target-per-platform`, `--seeds-per-division`, `--max-matches-per-seed`,
`--call-budget-per-platform`, `--concurrency` les adaptent.

Une campagne crée atomiquement ses exécutions et conserve leur liste, leur fenêtre,
leur ordre de rotation et son échéance. Chaque plateforme dispose d'une tranche de
15 minutes ; les plateformes restant à traiter sont parcourues à tour de rôle.
Un seul collecteur et son gouverneur de quotas sont partagés. À l'échéance, plus
aucune nouvelle plateforme n'est lancée ; les travaux en cours ont au maximum
15 secondes pour terminer avant annulation et remise en attente. La reprise ne
repousse pas cette échéance. Un refus 401/403 suspend toute la campagne, même si une limite est atteinte simultanément.
La borne concerne les appels Riot ; une finalisation SQL bloquée peut retarder le retour
du processus, sans autoriser de nouveaux appels.

L'ouverture des transactions est protégée contre l'annulation : si le résultat de
`BEGIN` devient incertain, la connexion est fermée au lieu de retourner au pool.
Cela contourne le [défaut SQLx 0.8.6](https://github.com/transact-rs/sqlx/pull/4394)
sans requête supplémentaire sur les transactions terminées normalement.

La clé de développement Riot expire après 24 h depuis sa génération : elle peut
expirer avant la fin d'une campagne. La remplacer dans l'environnement puis reprendre
la campagne seulement tant que son échéance originale n'est pas dépassée.

Les commandes ne créent pas de service système. Garder le processus actif, empêcher
la mise en veille si nécessaire, conserver les journaux localement. Une interruption
ne supprime pas les acquisitions. Les tâches restées `running` sont récupérées à la
reprise. Une requête reçue par Riot mais non enregistrée avant annulation pourra être
retéléchargée : unicité en base, pas garantie d'envoi exactement une fois.

## Quotas et erreurs

Les limites applicatives sont suivies par hôte, celles des méthodes par couple
hôte/méthode. Les en-têtes Riot remplacent les limites initiales prudentes d'une
clé de développement : 20 appels/s et 100/2 min. Les observations de rang ajoutent
jusqu'à un appel par nouveau joueur d'une partie 420/440 hors remake, avec cache
Solo/Flex de 24 h.

| Situation | Comportement |
| --- | --- |
| Quota local épuisé | Attente avant l'envoi |
| 429 | Respect de `Retry-After` et de sa portée ; pause prudente si absent |
| Réseau, timeout, 5xx, réponse invalide | Jusqu'à 5 essais, délai croissant et aléatoire |
| 401/403 | Suspension ; aucun nouvel appel du travail refusé pendant son drainage |
| 400 | Travail en échec |
| Détail/timeline 404 | Deux revalidations espacées de 10 puis 20 min |
| Classement participant 404 | Rang inconnu, travail en échec reprenable |

Un classement vide reçu avec HTTP 200 signifie **non classé** ; une erreur ou une
observation trop ancienne signifie **inconnu**. Le rang observé n'est ni le rang
historique du match ni un MMR. La table des observations conserve l'horodatage.

Un verrou PostgreSQL interdit deux collecteurs simultanés sur une même base.
La collecte, la synchronisation statique et les agrégats utilisent des verrous distincts.
Codes `run` : 0 cible atteinte, 2 sources épuisées sous la cible, 3 pause, 1 erreur.
Codes `campaign` : 0 parcours terminé (consulter chaque run pour la couverture),
3 pause/échéance, 1 erreur. Un objectif déjà acquis est réutilisé, pas compté comme
une nouvelle partie téléchargée.

## Synchronisation statique

```sh
cargo run -p olc-collector --release -- sync-static --patch-count 2 --json
cargo run -p olc-collector --release -- sync-static --refresh
cargo run -p olc-collector --release -- sync-static --watch
```

Le manifeste public et le realm EUW identifient la version live ; les patches distincts
les plus récents, sans dépasser le patch du realm, sont choisis numériquement. La dernière correction
du même patch est retenue même si le realm référence encore une build précédente.
Si le patch live manque au catalogue, la synchronisation échoue et garde le cache. Le patch
public 26.19 correspond à la version technique 16.19.1 au 1er octobre 2026. Aucune
version n'est figée dans le code. Data Dragon peut être publié avec retard : le cache
annonce sa version et la date de vérification, sans inventer une mise à jour.

FR/EN : index et détails de champions (Q/W/E/R, passif), namespace Classic distinct,
items, sorts d'invocateur, runes reforgées, cartes, icônes de profil. Les réponses JSON
complètes et leurs URLs sont conservées ; les images restent des références CDN.
Les bases d'assets standard/Classic sont distinctes. Les icônes de runes et les
catalogues globaux files/cartes/modes/types ne sont pas versionnés par patch : la
fraîcheur est celle du manifeste, et les URLs de runes viennent de leurs données.

8 téléchargements au plus en parallèle, tailles et délais bornés, validation avant
publication, 3 essais sur les pannes transitoires. Versions déjà complètes réutilisées ;
`--refresh` force leur vérification par téléchargement. L'ensemble des releases
sélectionnées et du manifeste est publié dans une transaction. Un téléchargement
incomplet ou un échec de commit garde le cache précédent. `--watch` vérifie chaque
heure, sans chevauchement ; une erreur arrête le processus avec code 1.

Sources : [Data Dragon](https://developer.riotgames.com/docs/lol#data-dragon),
[versions](https://ddragon.leagueoflegends.com/api/versions.json),
[files](https://static.developer.riotgames.com/docs/lol/queues.json),
[League-V4](https://developer.riotgames.com/apis#league-v4),
[Match-V5](https://developer.riotgames.com/apis#match-v5).

## Référentiel normalisé (#61)

Après `sync-static`, `cargo run -p olc-collector --release -- catalog --json`
publie les deux patches du cache : objets enrichis, champions/compétences,
runes/fragments, sorts et catalogues. Aucune clé Riot requise.
`--community required|optional|off` fixe la politique du complément ;
`--refresh` le revérifie, `--rebuild <publication_id>` reconstruit sans réseau.
Les sources exactes sont archivées par empreinte et les fiches publiées atomiquement.
Les champs inconnus et contradictions restent visibles, sans valeurs fabriquées.
Contrat complet, stockage, limites et matrice statistique :
[référentiel du jeu](../../docs/catalogue-jeu.md).

## Agrégats et contrat statistique (#18)

```sh
cargo run -p olc-collector --release -- aggregate --json
cargo run -p olc-collector --release -- aggregate --patches 16.19,16.18 --platforms EUW1,KR --queues 420,440
cargo run -p olc-collector --release -- aggregate --sync-static --watch
```

`--min-games 100` est le seuil par défaut ; `--from-ms` et `--to-ms` sélectionnent
une période UTC `[début, fin)` en millisecondes Unix. Sans `--sync-static`, aucun
réseau n'est utilisé. Avec cette option, les statiques sont vérifiées avant chaque
calcul ; un échec conserve l'ancien instantané et arrête le processus.

Le rapport JSON `schema_version: 2` sépare patch, plateforme, file, rôle et rang.
Chaque participation entre dans `ALL` et dans son rang observé : ne pas additionner
ces populations. Les files 420/440 utilisent le classement de la même file, figé à la
partie (#80) : l'observation la plus proche du début de partie, si l'écart ne dépasse
pas `--rank-max-age-hours` (168 h par défaut, 1 à 8 760). L'heure du calcul
n'intervient pas : recalculer des données inchangées redonne les mêmes rangs. L'écart est
arrondi à la seconde supérieure (la borne est incluse exactement) et l'observation est
cherchée par deux lectures d'index bornées (la dernière avant le début, la première
après) ; chiffres dans [`docs/recettes/2026-10-04-rang-fige.md`](../../docs/recettes/2026-10-04-rang-fige.md). Ce palier
observé n'est ni un MMR ni le rang exact au lancement de la partie. Les autres files
ont `UNRANKED_MODE`. `UNKNOWN` (aucune observation assez proche) et `UNRANKED` restent
distincts. `rank_scope` vaut `observed_rank_nearest_to_game_start_of_same_ranked_queue` ;
chaque couverture publie `unknown_rank_rate` (part `UNKNOWN` des participations
Solo/Flex, en %) et les écarts médian/maximal retenus (`rank_gap_median_hours`,
`rank_gap_max_hours`), nuls hors files classées ou sans observation. Chaque couverture
publie aussi `first_game_start_ms` et `last_game_start_ms` (début, en ms Unix, de la plus
ancienne et de la plus récente partie **incluse** du périmètre, remakes et parties
invalides exclus, #103) : la vraie fraîcheur, distincte de `source_snapshot_at` qui est
l'heure du calcul. Les écarts `rank_gap_*_hours` restent l'âge de l'observation de rang
relativement à la partie ; ils ne dépendent pas de l'heure du calcul. Les rôles sont `TOP/JUNGLE/MIDDLE/BOTTOM/UTILITY/UNKNOWN` ; aucun rôle n'est
inventé à partir des objets ou du rang.

| Mesure | Définition et limites |
| --- | --- |
| `games`, `wins`, `losses` | Participations, pas toujours matchs distincts : les modes autorisant plusieurs exemplaires d'un champion peuvent contribuer plusieurs fois par match |
| `population` | Toutes les participations du même patch/plateforme/file/rôle/rang |
| `win_rate` | Victoires / participations du champion × 100 ; nul en Arena (voir ci-dessous) |
| `pick_rate` | Participations du champion / population × 100 ; part des sélections dans ce groupe |
| `win_rate_lower_bound` | Borne inférieure de Wilson à 95 %, estimation descriptive de l'incertitude binomiale, bornée à 0–100 (0 exact pour 0 victoire, sans résidu flottant négatif) ; nulle en Arena |
| `position`, `tier` | Borne Wilson décroissante, puis taux, effectif et ID ; S/A/B/C/D par tranches 10/30/60/90/100 %, au moins 5 champions éligibles dans le groupe. En Arena : placement moyen croissant, puis effectif décroissant et ID ; le ratio brut de victoires n'intervient jamais |
| `placement_games`, `average_placement`, `top1_rate`, `top2_rate` | Arena seulement (#104) : participations au placement valide, placement moyen de la sous-équipe (1 = première), part (%) des participations classées première, puis première ou deuxième. Nuls hors Arena et sous le seuil (`placement_games` reste visible) |
| `most_picked_rank` | Rang connu avec le plus de participations de ce champion ; dépend des effectifs collectés par rang |
| `bans` | Tableau séparé patch/plateforme/file : matchs bannissant le champion / drafts complètes ; un double ban ne compte qu'une fois |

Sous le seuil, taux champion/build et classement sont nuls ; les comptes restent
visibles. Le taux de ban demande au moins ce nombre de drafts complètes. La borne
Wilson et les tiers ne corrigent pas les biais d'échantillonnage ni les dépendances
entre parties d'un même joueur. Le booléen `win` d'Arena ne désigne pas une première
place (sur les files 1740/1750 observées, il vaut vrai pour les trois meilleures des six
sous-équipes) : les groupes Arena ne publient donc ni `win_rate` ni borne de Wilson, et
leurs `wins`/`losses` bruts ne servent à aucun classement. Le placement est lu dans
`subteamPlacement`, à défaut `placement` (0 = absent). Il n'est retenu que si chaque
sous-équipe porte une valeur unique et que les sous-équipes occupent des places distinctes de
1 à leur nombre ; sinon la partie reste comptée dans `games` mais sans placement
(`unknown_placement_participations` dans la couverture). Le placement moyen par champion
n'est ni un winrate d'objet ni un taux d'augment ; aucune statistique d'augment n'est
calculée. Les variantes de builds hors objets (liste fermée : runes, sorts, ordre de
compétences) suivent la même règle en Arena : placement moyen au lieu du taux de
victoire (voir plus bas). Les parties normales/PvE et Arena ne sont jamais
mélangées à SoloQ.

Les remakes et parties incohérentes sont exclus avant toute contribution. Les formats
classiques contrôlent 5 participants par équipe et un vainqueur ; Arena contrôle les
sous-équipes (duos, ou trios pour les files 1740/1750 observées). Swarm solo est supporté par
une fixture synthétique, sans recette réelle revendiquée. En coop contre IA, les
fiches des bots sont validées mais exclues des statistiques des joueurs ;
`excluded_bot_participations` en donne le compte. Les données de bans ne
comptent que si les deux listes de 5 slots et leurs tours sont complets. Les bans
n'ont pas de rang/rôle individuel attribuable dans les réponses Riot.

## Builds et timelines

Chaque catégorie possède son propre effectif disponible ; une donnée manquante ne
compte jamais comme un choix vide. `builds` donne participations, victoires, population,
pickrate et winrate de chaque variante. Exception imposée par la
[politique Riot](https://developer.riotgames.com/docs/lol#game-policy) : les catégories
d'items Arena (`item`, `final_items`, `trinket`, `purchase_order` et les étapes
`starter`, `boots`, `core`, `item_slot_4..6`) ne publient ni victoires, ni winrates, ni
borne Wilson (`wins`/`win_rate`/`win_rate_lower_bound` nuls, `performance_available: false`) ; leur
tri ne dépend pas des victoires, et ils ne publient pas non plus de placement (#104). Aucun taux d'augment n'est produit.
En Arena (#104), les catégories de la liste positive `ARENA_PLACEMENT_CATEGORIES` (`runes`,
`summoner_spells`, `skill_order`, `special_skill_order`, et aucune autre : une catégorie
nouvelle n'y publie rien tant qu'elle n'y est pas ajoutée) ne publient pas non plus de taux de victoire (`wins`/`win_rate`/
`win_rate_lower_bound` nuls, `performance_available: false`) : le booléen `win` d'Arena n'est
pas une première place. Elles publient `placement_games` (participations au placement valide,
voir les règles de cohérence plus haut) et `average_placement` (placement moyen de la
sous-équipe, 1 = première, nul sous le seuil). À effectif égal, le placement moyen croissant
départage les variantes ; la publication des 20 variantes les plus fréquentes reste fondée sur
la popularité. Aucun taux de première ou de deuxième place n'est publié pour les variantes
(les objets n'en publient pas non plus), ni aucun placement par objet ou par augment. Hors
Arena, ces catégories gardent victoires et taux de victoire, avec `placement_games` à 0 et
`average_placement` nul. Au plus 20 variantes par catégorie/groupe sont
publiées, par popularité, sans modifier leur dénominateur ; `omitted_build_variants`
annonce, pour diagnostic, le total **global** des variantes supplémentaires conservées
seulement dans les sources brutes (tous groupes confondus, donc sans sens pour une fiche).
Chaque variante publiée porte `omitted_variants` (#113) : le nombre de variantes coupées
dans son propre (groupe, catégorie), calculé à la finalisation ; `null` dans un rapport antérieur.

- `final_items` : ensemble trié d'items distincts des slots 0–5 ; `item` donne chaque
  item individuel, au plus une fois par participation ; `trinket` correspond au slot 6.
- `summoner_spells` : paire d'identifiants regroupée sans tenir compte de l'ordre D/F
  (une seule variante par paire). L'ordre publié est l'orientation D/F la plus fréquente
  parmi les parties de la variante (`summoner1Id` en D, `summoner2Id` en F), avec ou sans
  Flash ; à égalité, l'ordre numérique. Cette orientation n'influence ni l'effectif ni le classement.
- `runes` : 11 identifiants ordonnés — arbre principal, 4 runes principales, arbre
  secondaire, 2 runes secondaires, fragments offense/flex/défense.
- `skill_order` : points Q/W/E/R normaux dans l'ordre temporel ; `special_skill_order`
  sépare les évolutions. `skill_levels` expose l'ordre du point investi et son temps
  moyen, pas le niveau du champion (les points peuvent être gardés).
- `purchase_order` : achats incluant composants et consommables, avec retrait des
  achats annulés ; ce n'est pas un inventaire final reconstruit.
- `item_events` : achats, ventes, destructions et annulations regroupés par minute.

Chaque variante publie aussi `win_rate_lower_bound`, borne inférieure de Wilson à 95 %
bornée à 0–100 (le client desktop rejette toute page hors de cet intervalle), nulle sous le seuil ou sans performance publiable (Arena, toutes catégories).

### Étapes d'achat (#81)

Les empreintes exactes ci-dessus fragmentent la population (aucune variante
`final_items`/`purchase_order` à 100 parties sur la recette). Les étapes sont des
catégories supplémentaires, chacune avec sa propre population, dérivées de la séquence
nette d'achats (`purchase_order`) jointe au catalogue normalisé #61 du patch de la
partie (`game_catalog_current`, version `16.19.x` la plus récente pour le patch `16.19`) :

| Catégorie | Règle | Population |
| --- | --- | --- |
| `starter` | Achats nets strictement avant 1 min 30 (90 000 ms), balises exclues ; multiensemble trié (deux potions restent deux) | Participations à achats nets connus |
| `boots` | Premier achat de bottes : étiquette `Boots` ou chaîne `builds_from` remontant à 1001/2422 (Gunmetal Greaves n'a pas l'étiquette) ; `[]` si aucune paire achetée | Idem |
| `core` | Trois premiers objets complets distincts, **dans l'ordre d'achat** (non trié) | Participations ayant terminé au moins 3 objets |
| `item_slot_4`, `_5`, `_6` | 4e, 5e, 6e objet complet distinct | Participations ayant terminé au moins N objets |

Objet complet : `purchasable`, en boutique, sans `builds_into`, `price_total` ≥ 2000, ni
`Consumable`/`Trinket`/`Boots` ni bottes par chaîne ; la profondeur n'est pas utilisée
(Infinity Edge est de profondeur 2). Les transformations sont ramenées à l'objet acheté
via `special_recipe` (Muramana → Manamune, Séraphin → Archange, Fimbulvetr → Approche de
l'hiver) et un objet revendu puis racheté ne compte qu'une fois. Seules les valeurs
`verified`/`derived` du catalogue sont lues. La timeline match-v5 ne signale aucun retour
ni sortie de base : la fenêtre de 1 min 30 est une approximation documentée.

Sans catalogue publié pour le patch, aucune étape n'est produite et
`missing_item_catalog_participations` compte ces participations ;
`item_stage_participations` compte celles qui ont des étapes. Un remboursement sans
objet identifiable supprime `purchase_order` et donc les étapes. Le rapport publie
`build_stage_method` (règles) et `item_catalogs` (patch → version jointe). Prérequis
d'exploitation : publier le catalogue (`catalog`, ci-dessus) après `sync-static` ;
`aggregate --sync-static` ne le publie pas. Biais : le core et les emplacements ne
décrivent que les parties assez longues pour les atteindre (survie et durée).

### Durée, côté et premiers objectifs (#119)

Trois lectures du déroulé de la partie, toutes tirées de données publiques de match-v5
(`gameDuration`, `teamId`, `teams[].objectives.*.first`), sans estimation :

- **Tranche de durée** (liste `splits`, `dimension: "duration"`) : parties et victoires
  de chaque champion pour chaque groupe (patch, plateforme, file, rôle, rang, y compris
  `ALL`). Tranches à borne basse incluse : `lt_20` (moins de 20 min), `20_25`, `25_30`,
  `30_35`, `35_40`, `gte_40` ; `20_25` va donc de 20 min 00 s à 24 min 59 s. La durée est lue
  dans la colonne `matches.game_duration_s`, déjà normalisée en secondes à l'ingestion
  (pas dans le JSON brut, dont l'unité varie selon l'ancienneté de la partie). Une durée
  nulle ou absente n'entre dans aucune tranche. Reddition et parties courtes hors remake
  restent incluses : leur comptage séparé n'est pas encore publié.
- **Côté** (`splits`, `dimension: "side"`, `bucket` `blue` ou `red`) : winrate du champion
  selon l'équipe 100 (bleue) ou 200 (rouge), **uniquement pour le rang `ALL`** : une
  ventilation par rang et par côté diluerait les effectifs. La couverture publie aussi
  `blue_side_matches`, `blue_side_wins` et `blue_side_win_rate` (winrate du côté bleu du
  périmètre).
- **Premiers objectifs** (couverture : `first_blood`, `first_dragon`, `first_tower`) : pour
  chaque objectif, `matches` et `wins` (parties où une équipe l'a pris en premier, et
  victoires de cette équipe), `blue_matches` et `blue_wins` (même lecture quand c'est
  l'équipe bleue ; le rouge se déduit par différence), `win_rate` et `blue_win_rate`. Le
  premier sang est `objectives.champion.first`. Une partie n'est comptée que si exactement
  une des deux équipes porte `first: true` : aucune équipe première (objectif jamais pris),
  donnée contradictoire ou absente → partie ignorée pour cet objectif, jamais devinée. Une
  équipe qui prend un objectif en premier a pu le prendre parce qu'elle était déjà en
  avance : ce winrate décrit une corrélation, pas un effet causal.

Taux et bornes de Wilson sont nuls sous le seuil `min_games` du rapport ; les effectifs
restent publiés. Aucune ligne n'est produite pour Arena, la coop contre l'IA (les humains
y occupent toujours le même camp) ni les modes qui n'opposent pas les équipes 100 et 200
(Swarm), où `win` ne désigne pas une victoire de côté. Les
instantanés antérieurs n'ont pas `splits` ni ces champs de couverture : ils se lisent comme
vides (`0` ou `null`) jusqu'au prochain calcul.

Les événements système `participantId=0` sont ignorés. Une timeline absente ou
incohérente ne devient pas une séquence vide : les compteurs de couverture l'indiquent.
Les remboursements `ITEM_UNDO` sans identifiant d’objet (`beforeId=afterId=0`,
`goldGain` non nul) préservent compétences et événements connus, mais suppriment
l’ordre net des achats concerné : aucun objet remboursé n’est deviné.
`unidentified_item_undos` compte ces événements. Les timestamps exposés sont
relatifs au début du match, sans identifiant de joueur.

## Publication, stockage et exploitation

`champion_stats_snapshot` contient une tête : `source_snapshot_at`, `published_at`,
`storage_version`, `report`. En stockage v2, `report` contient les métadonnées ;
les sept listes (`coverage`, `groups`, `bans`, `builds`, `skill_levels`, `item_events`,
`splits`) sont dans `champion_stats_snapshot_chunks`, ordonnées par section et `chunk_index`.
Chaque morceau contient au plus 512 entrées et 1 Mio de JSON sérialisé avant conversion
PostgreSQL. Une entrée individuelle dépassant cette borne fait échouer la publication,
sans supprimer de statistiques. Le rapport JSON public et le CLI restent au schéma 2.
Les lecteurs doivent lire tête et morceaux en une seule requête ou dans une transaction
`REPEATABLE READ`, filtrer les morceaux puis reconstruire les listes côté application.
Ne pas reconstituer le rapport entier en JSONB SQL : sa limite interne est de 256 Mio.
La migration conserve les anciens rapports complets en stockage v1 jusqu'au prochain
calcul réussi ; la lecture API des deux formats est livrée séparément par le #19. Un ancien écrivain est refusé après
le passage en v2, pour éviter un mélange silencieux de versions.

La migration `0014` ajoute la section `splits` (#119) à la contrainte de section des
morceaux, en conservant les six sections existantes ; elle ne réécrit pas les lignes
(simple validation de la contrainte, qui verrouille brièvement la table). Le premier calcul
qui suit la publie.

La migration `0009` ajoute aux morceaux une colonne `populations` calculée et stockée,
avec un index GIN. Elle regroupe les combinaisons distinctes de dimensions sous leur
section ; l'API peut sélectionner les morceaux utiles sans parcourir tous les JSON.
PostgreSQL maintient ces métadonnées à chaque insertion ou modification des entrées,
sans changement du format publié ni des bornes de morceaux. Les morceaux existants
sont indexés à la migration, qui réécrit et verrouille cette table ; prévoir une
fenêtre d'exploitation adaptée. Le coût du calcul et de l'index passe à l'écriture.
`build.rs` suit le répertoire de migrations pour que les nouveaux fichiers SQL soient
embarqués même après une compilation incrémentale. Voir la
[recette de performance](../../docs/recettes/2026-10-02-filtrage-builds.md).

Un recalcul remplace la tête et tous les morceaux dans une seule transaction,
sans additionner l'ancien. Transaction `REPEATABLE READ`, lecture par lots de 25,
verrou de calcul distinct de la collecte, publication atomique ; une base sans données
éligibles publie un bilan vide. Un conflit de publication n'écrase pas un instantané
plus récent. Après une coupure pendant COMMIT, relire la base : le résultat du commit
peut être inconnu, aucun succès n'est annoncé avant confirmation.

`--watch` calcule immédiatement puis chaque heure. Si un calcul dépasse une heure,
un seul rattrapage immédiat est exécuté puis les échéances manquées sont sautées.
Ctrl+C annule le calcul ou l'attente (0 en mode continu, 3 en ponctuel).
Aucun superviseur système n'est installé par le binaire.

Tables : `collection_runs`, `collection_jobs`, `seed_players`, `run_discoveries`,
`run_matches`, `matches`, `match_timelines`, `participant_rank_observations`,
`collection_campaigns`, `campaign_runs`, `static_data_releases`, `static_data_manifest`,
`champion_stats_snapshot`, `champion_stats_snapshot_chunks`. Les détails et timelines restent complets en JSONB ; les
observations de rang gardent leur historique daté.

Les lots bornent les données brutes simultanément lues, **pas toute la mémoire** :
les compteurs de variantes et événements restent en RAM jusqu'à la publication.
La recette mesure temps et mémoire ; un passage à très grande échelle nécessitera
une stratégie de calcul/pagination supplémentaire. Les groupes trop petits restent
hors classement, même après une longue collecte.

## Conformité, données personnelles et validation

Clé côté service uniquement. PUUID/Riot ID restent dans la base privée, jamais dans
les rapports publiables. Les données brutes ne sont pas un export public. Cette
fonctionnalité exploite des parties terminées et des données statiques officielles,
sans action dans le jeu. La conservation/suppression des données personnelles et la
clé de production restent à traiter avant exploitation publique (#2).

Open LoL Companion isn't endorsed by Riot Games and doesn't reflect the views or
opinions of Riot Games or anyone officially involved in producing or managing Riot
Games properties. Riot Games, and all associated properties are trademarks or
registered trademarks of Riot Games, Inc.

```sh
pnpm test
pnpm lint
# Avec PostgreSQL réel, exemple Unix :
OLC_TEST_DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres pnpm test
```

Les tests créent des bases jetables et utilisent des réponses synthétiques, sans
appeler Riot. Sans `OLC_TEST_DATABASE_URL`, les tests PostgreSQL sont ignorés : ne
pas présenter ce résultat comme une recette de stockage complète. Le code n'a pas
de branche système spécifique ; la recette locale est macOS + PostgreSQL Linux,
la validation native Windows reste à exécuter en CI et sur machine Windows.

Recettes : [prototype d'agrégation](../../docs/recettes/2026-10-01-agregation.md),
[extension et campagne](../../docs/recettes/2026-10-01-agregation-complete.md).
