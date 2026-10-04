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
| `OLC_RETENTION_IDENTIFIER_DAYS` | Facultatif : conservation des PUUID, Riot ID et observations de rang, 30 jours par défaut (#99) |
| `OLC_RETENTION_RAW_MATCH_DAYS` | Facultatif : conservation des parties brutes, 90 jours par défaut (#99) |
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
| `--queue` | 420 | File exacte ; 0 = toutes les files retournées (commande `run` ; la campagne utilise `--queues`) |
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

Ordre de réservation des travaux d'une exécution : joueurs de départ, historiques,
timelines, **détails de parties**, puis rangs. Les détails passent avant les rangs : un
rang ne sert que pour une partie déjà retenue et ne doit pas consommer le budget qui
permettrait de télécharger plus de parties (#90).

**Cache négatif.** Une partie téléchargée puis exclue du périmètre (plateforme, file,
patch ou fenêtre) est mémorisée dans `excluded_matches` : ses faits indexés seulement
(file, patch, date, durée, remake), ni détail ni identifiant de joueur. Une autre
exécution la rejuge sans appel si son périmètre l'exclut aussi ; si son périmètre
l'accepte, elle est téléchargée normalement. Le verdict n'est donc jamais figé.

**Maintenance des rangs inutiles.** Les demandes de rang créées avant le filtrage par
file peuvent rester en attente pour des parties non classées :

```sh
cargo run -p olc-collector --release -- close-unserved-ranks            # simulation, ne modifie rien
cargo run -p olc-collector --release -- close-unserved-ranks --apply    # ferme (outcome skipped:unserved_queue)
```

`--run-id <id>` limite l'opération à une exécution. Seules les demandes `pending` ou
`retry_wait` sont fermées, et seulement si le joueur apparaît dans des parties retenues
dont aucune n'est classée (420/440 hors remake) ; un joueur dont le détail est purgé ou
caviardé est conservé. Commencer par la simulation, jamais sur une base de production
sans sauvegarde.

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

Par défaut : les 15 plateformes, les files Solo/Duo (420) et Flex (440), les deux patches du cache,
Iron à Challenger, 5 seeds par strate, 100 historiques par seed, fenêtre de 28 jours,
observations des rangs activées, cible de 10 000 parties et budget de 100 000 appels
**par plateforme**, concurrence 4. Ce sont des plafonds/objectifs, pas une promesse
de volume atteint en 24 h. Les options `--platforms`, `--patches`,
`--target-per-platform`, `--seeds-per-division`, `--max-matches-per-seed`,
`--call-budget-per-platform`, `--concurrency`, `--queues` les adaptent.

**Files de la campagne** (décision du 4 octobre 2026, #90). `--queues` prend une liste
séparée par des virgules ; par défaut `420,440` (Solo/Duo et Flex). ARAM, Swiftplay,
Arena et les autres files restent possibles à la demande en les listant
explicitement, par exemple `--queues 450,480,1700` ou `--queues 420,440,450`. Riot ne
filtre l'historique que sur une file : la campagne crée donc **une exécution par
plateforme et par file**. Le budget d'appels et la cible de parties « par plateforme »
sont répartis également entre ses files (budget arrondi à l'inférieur, cible à la
supérieure) ; avec une seule file, ils restent entiers, et un budget inférieur au nombre
de files est refusé. Les parties des autres files
ne sont jamais téléchargées. Les rangs ne sont observés que pour 0, 420 et 440 ;
pour toute autre file, `collect_ranks` est coupé. `--queues 0` découvre toutes les
files de l'historique (comportement historique) et ne se combine avec aucune autre.

Une campagne crée atomiquement ses exécutions et conserve leur liste, leur fenêtre,
leur ordre de rotation et son échéance. Chaque exécution (une plateforme et une file)
dispose d'une tranche de 15 minutes ; les exécutions restant à traiter sont parcourues
à tour de rôle (30 tranches par rotation avec les deux files par défaut).
Un seul collecteur et son gouverneur de quotas sont partagés. À l'échéance, plus
aucune nouvelle exécution n'est lancée ; les travaux en cours ont au maximum
15 secondes pour terminer avant annulation et remise en attente. La reprise ne
repousse pas cette échéance. Un refus 401/403 suspend toute la campagne, même si une limite est atteinte simultanément.
La borne concerne les appels Riot ; une finalisation SQL bloquée peut retarder le retour
du processus, sans autoriser de nouveaux appels.

L'ouverture des transactions est protégée contre l'annulation : si le résultat de
`BEGIN` devient incertain, la connexion est fermée au lieu de retourner au pool.
Cela contourne le [défaut SQLx 0.8.6](https://github.com/transact-rs/sqlx/pull/4394)
sans requête supplémentaire sur les transactions terminées normalement.

### Campagnes par file : ARAM, Swiftplay, Arena (#97)

```sh
cargo run -p olc-collector --release -- campaign-queues --hours 24
cargo run -p olc-collector --release -- campaign-queues --queues 450,480,1700,1740,1750 --target-per-queue 500
cargo run -p olc-collector --release -- campaign-report 2
```

La campagne historique part de toutes les files et n'oriente pas le volume : en
recette, aucun mode hors Faille n'a atteint le seuil de publication. `campaign-queues`
crée **une exécution par plateforme et par file** (ARAM 450, Swiftplay 480, Arena 1700 par
défaut), chacune avec sa cible (`--target-per-queue`, 1000) et son budget d'appels
(`--call-budget-per-queue`, 20 000). L'historique de chaque joueur est demandé avec le
filtre `queue` de match-v5 ; les rotations, tranches de 15 minutes, échéance, reprise
(`campaign-resume`) et codes de sortie sont ceux de `campaign`. L'ordre de rotation est
plateforme puis file : une campagne écourtée couvre toutes les files des premières
plateformes. Les observations de rang ne sont demandées que si une file classée (420, 440)
est visée : les autres files sont agrégées en `UNRANKED_MODE`.

- Seuls les identifiants de files identifiées (`queues.rs`) sont acceptés ; une liste vide,
  dupliquée, nulle ou inconnue (par exemple 710 ou 3130) est refusée avant toute création.
- Les seeds restent issus du classement Solo, comme pour les autres files non Flex : les
  joueurs jamais classés ne sont pas atteints.
- Arena est demandée sous 1700. La recette a observé des parties 1740 et 1750 (absentes du
  catalogue Data Dragon) : ce sont des files distinctes, à ajouter explicitement dans
  `--queues`. Le périmètre d'une exécution compare la file exactement ; si le filtre
  `queue=1700` de Riot ne renvoie pas ces variantes, elles ne seront pas collectées. Cela
  n'a pas pu être vérifié sans appel réel.
- `campaign-report <id> [--json]` lit la base et donne, pour chaque plateforme et file,
  la cible, les parties retenues (déjà présentes comprises), les appels et l'état. Une
  ligne sous sa cible veut dire données non acquises, pas absence d'activité.
- Aucun taux d'augment ni d'objet Arena n'est produit ; les politiques Riot citées plus
  bas s'appliquent inchangées.

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

Le seau est partagé avec l'API des profils : le collecteur n'en consomme que 80 %
(16 appels/s et 80 par 2 min avec les limites ci-dessus, au moins 1 par fenêtre) ;
le reste est réservé aux requêtes interactives de l'API. Voir
[`services/api/README.md`](../api/README.md#profils-historique-et-quotas).

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
runes/fragments, sorts, augments Arena et Mayhem (catalogue statique : noms FR/EN, description quand
l'export `cdragon/arena` la publie, rareté, icône, modes qui les listent, #118) et catalogues. Aucune clé Riot requise. Une version déjà publiée
n'a pas d'augments tant que `--refresh` n'a pas relu CommunityDragon ; `--rebuild` rejoue les
archives antérieures sans augments. Aucune statistique d'augment n'est produite.
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

`--json` écrit une ligne par publication en mode continu. Son contenu dépend du mode de
recalcul : rapport complet pour le recalcul complet (défaut d'un `aggregate` ponctuel,
ou `--full`), bilan des lots et en-tête publié, listes vides, pour le recalcul
incrémental (défaut de `--watch`, voir [Recalcul par lots](#recalcul-par-lots-89)).
Un script qui lisait les listes d'un `aggregate --watch --json` doit donc passer
`--full` ou lire l'instantané publié.

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
relativement à la partie ; ils ne dépendent pas de l'heure du calcul.

`ALL` n'est pas « tous les rangs » du ladder (#82) : c'est l'échantillon collecté, où
chaque participation compte 1, sans pondération par la taille réelle des paliers. Les
seeds étant pris en nombre égal par strate, le haut du ladder y est surreprésenté. Pour
le rendre lisible, chaque couverture publie `tier_participations`, la répartition des
participations classées par palier observé figé à la partie (`IRON` … `CHALLENGER`,
tous rôles confondus). Sa somme égale `ranked_participations` ; `UNRANKED`, `UNKNOWN` et
`UNRANKED_MODE` gardent leurs compteurs propres. Elle est vide hors Solo/Flex et dans un
instantané antérieur. Aucune repondération n'est appliquée aux taux.
En attendant une collecte équilibrée par palier, la couverture publie aussi un indicateur
de biais dérivé de cette répartition : `apex_share`, part (entre 0 et 1, et non en %) des
participations classées en Master, Grandmaster et Challenger sur la somme de
`tier_participations`, et `high_elo_biased`, vrai quand `apex_share` dépasse strictement
0,5 (50 % pile n'est pas biaisé). Sans participation classée (hors Solo/Flex, instantané
antérieur), `apex_share` vaut `null` et `high_elo_biased` `false`. Un instantané publié
avant l'indicateur mais déjà doté de `tier_participations` est complété à la lecture par
l'API (`Coverage::complete_tier_bias`, même calcul). C'est une information de lecture :
rien n'est corrigé ni décidé à partir d'elle. Les rôles sont `TOP/JUNGLE/MIDDLE/BOTTOM/UTILITY/UNKNOWN` ; aucun rôle n'est
inventé à partir des objets ou du rang.

| Mesure | Définition et limites |
| --- | --- |
| `games`, `wins`, `losses` | Participations, pas toujours matchs distincts : les modes autorisant plusieurs exemplaires d'un champion peuvent contribuer plusieurs fois par match |
| `population` | Toutes les participations du même patch/plateforme/file/rôle/rang |
| `bucket_matches` | Parties distinctes comptant au moins une participation du même patch/plateforme/file/rôle/rang (#84) ; 0 dans un instantané antérieur |
| `win_rate` | Victoires / participations du champion × 100 ; nul en Arena (voir ci-dessous) |
| `pick_rate` | Parties où le champion apparaît / `bucket_matches` × 100 (#84) : « présent dans la partie », même base que `bans` ; une partie compte une fois même si le champion y est en double, donc au plus 100 %. Défini par `pick_rate_definition` (`champion_matches / bucket_matches * 100`) |
| `selection_share` | Participations du champion / population × 100 : part des sélections dans ce groupe, l'ancien `pick_rate` (#84), plafonnée à 50 % par rôle en 5v5 ; nulle avant #84 |
| `win_rate_lower_bound` | Borne inférieure de Wilson à 95 %, estimation descriptive de l'incertitude binomiale, bornée à 0–100 (0 exact pour 0 victoire, sans résidu flottant négatif) ; nulle en Arena |
| `win_rate_upper_bound` | Borne supérieure de Wilson à 95 % du winrate (#91), publiée et masquée avec `win_rate_lower_bound` |
| `pick_rate_lower_bound`, `pick_rate_upper_bound` | Intervalle de Wilson à 95 % du `pick_rate` (parties du champion sur `bucket_matches`, #91), nul quand le `pick_rate` l'est |
| `reliability` | `low` si `games` < `reliability_floor` (30), sinon `sufficient` (#91). Calculé même quand `min_games` masque le taux ; absent d'un instantané antérieur |
| `position` | Score de tier décroissant (#85), puis winrate brut, effectif et ID ; publiée pour tout groupe au-dessus du seuil, même sans tier. En Arena (#104) : placement moyen croissant, puis effectif décroissant et ID ; le ratio brut de victoires n'intervient jamais |
| `tier` | Lettre sur seuils absolus du score (#85), sans répartition forcée : S ≥ +2,5, A ≥ +1, B ≥ −1, C ≥ −2,5, D en dessous. Score = winrate lissé − μ + 0,02 × (`pick_rate` + ban rate), où μ est le winrate du compartiment en % (victoires / participations de tous ses champions × 100) et le winrate lissé (100 × victoires + 200 × μ) / (parties + 200). Le ban rate est celui du même périmètre au même rang de partie (#109) pour `ALL`, `UNRANKED_MODE` et les paliers classés, 0 pour `UNKNOWN`/`UNRANKED` (rangs de joueur sans équivalent par partie) ou sous le seuil de drafts. Nul si `pick_rate` < 0,5 % ou si moins de 20 champions du compartiment passent ce seuil. Formule publiée dans `tier_method`. Nul en Arena : sans winrate, pas de score |
| `placement_games`, `average_placement`, `top1_rate`, `top2_rate` | Arena seulement (#104) : participations au placement valide, placement moyen de la sous-équipe (1 = première), part (%) des participations classées première, puis première ou deuxième. Nuls hors Arena et sous le seuil (`placement_games` reste visible) |
| `most_picked_rank` | Rang connu avec le plus de participations de ce champion ; dépend des effectifs collectés par rang |
| `bans` | Tableau séparé patch/plateforme/file/rang de partie (#109) : matchs bannissant le champion / drafts complètes du même rang ; un double ban ne compte qu'une fois. Le rang `ALL` garde toutes les drafts ; chaque draft compte aussi sous le palier de sa partie |

Sous le seuil, taux champion/build et classement sont nuls ; les comptes restent
visibles. Le taux de ban demande au moins ce nombre de drafts complètes. La borne
Wilson et les tiers ne corrigent pas les biais d'échantillonnage ni les dépendances
entre parties d'un même joueur. La borne Wilson reste publiée comme intervalle mais
n'entre plus dans le tier : elle classait 54 % sur 150 parties derrière 50 % sur 2 000.
Le lissage rapproche un petit échantillon de la moyenne de son compartiment sans le
pénaliser sous elle ; des champions tous entre 49 et 51 % reçoivent tous B. Le tier
reste descriptif : il ne conseille aucun choix de draft. Le booléen `win` d'Arena ne
signifie pas nécessairement une première place. Les parties normales/PvE et Arena ne sont jamais mélangées à SoloQ.

Plancher de fiabilité (#91) : `reliability_floor` (constante de 30, publiée dans le rapport)
est **indépendant de `min_games`**. `min_games` décide de la publication des taux ; le plancher
signale qu'un taux publié repose sur un petit effectif. Avec `--min-games 1`, un champion à
10 parties a donc un `win_rate`, un intervalle de Wilson large et `reliability: low`. Les
bornes (`*_lower_bound`, `*_upper_bound`, 0–100) sont publiées pour le winrate et le pick rate
des champions, le ban rate et le winrate des variantes de build (borne haute) ; elles sont
nulles quand le taux correspondant l'est. L'effectif jugé est `games` pour un champion ou une
variante et `draft_matches` pour un ban. Aucun instantané n'est refusé sous le plancher : le
choix d'afficher, de marquer « test » ou de refuser la publication relève du client et du
produit. Un instantané antérieur à #91 se relit avec `reliability_floor` à 0 et sans
intervalle ni fiabilité. Le taux de ban demande au moins ce nombre de drafts complètes. La borne
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

Les remakes et parties incohérentes sont exclus avant toute contribution. Une partie dont
la file n'est pas identifiée (ni formats à deux camps, ni Arena, ni Swarm : par exemple 710
ou 3130, dont le sens n'est pas vérifiable hors ligne) l'est aussi, sous la raison
`unknown_queue` visible dans `exclusions` ; ses données brutes restent en base (#97).
Pour les files classées 420/440 (Solo/Duo et Flex), trois contrôles de qualité supplémentaires écartent
la partie entière (#111), chacun avec son compteur dans `exclusions` (visible dans le
rapport, la sortie texte de `aggregate` et `meta.exclusions` de l'API) :

| Motif (`exclusions`) | Règle | Réglage |
| --- | --- | --- |
| `short_game` | durée stockée de la partie < `--min-game-duration-s` (300 s par défaut, 0 à 900 ; 0 désactive) | `min_game_duration_s` publié dans le rapport |
| `afk` | au moins un participant a `wasAfk = true` (champ match-v5 des participants, déjà stocké) | actif par défaut ; `--keep-afk` le désactive ; `exclude_afk` publié dans le rapport |
| `early_departure` | un participant a un `timePlayed` < `--min-played-percent` % de la durée (80 par défaut, 0 à 100 ; 0 désactive) | `min_played_percent` publié dans le rapport |

Ordre d'évaluation : `remake`, `invalid_match`, puis `short_game`, puis `afk`, puis
`early_departure` ; une partie n'est comptée qu'une fois, sous son premier motif. Une partie
très courte avec un AFK est donc comptée `short_game`, pas `afk`. Une reddition normale
(`gameEndedInSurrender`) n'est jamais un motif : seuls la durée, `wasAfk` et le temps joué
comptent. Une clé absente (`wasAfk`, `timePlayed`) n'est pas jugée : rien n'est deviné. Une
valeur de mauvais type (`wasAfk` non booléen, `timePlayed` qui n'est pas un entier positif)
rend la partie `invalid_match`, seulement si le contrôle correspondant est actif. La
comparaison du temps joué est faite en entiers : exactement 80 % joués est conservé. Les
autres files ne sont pas concernées. Un rapport publié avant #111 relit `min_game_duration_s`
et `min_played_percent` à 0 et `exclude_afk` à `false` (contrôles non appliqués).
Chiffres de la copie de recette (17 112 parties 420/440 hors remake) : 37 parties de moins
de 300 s (`short_game`, dont 35 portaient aussi un AFK), 602 parties `afk` supplémentaires
(637 parties contiennent au moins un `wasAfk = true`, soit ≈ 3,7 %), 0 `early_departure`
(le plus petit rapport `timePlayed` / durée est 0,989 : `wasAfk` est le vrai signal, le
temps joué est conservé comme garde-fou pour des données atypiques) et 265 parties sans clé
`wasAfk`, non jugées. Il reste 16 473 parties conservées. Les parties entre 300 et 900 s
(≈ 150) restent conservées : le seuil par défaut ne coupe que les cas manifestement
anormaux. Les formats
classiques contrôlent 5 participants par équipe et un vainqueur ; Arena contrôle les
sous-équipes (duos, ou trios pour les files 1740/1750 observées). Swarm solo est supporté par
une fixture synthétique, sans recette réelle revendiquée. En coop contre IA, les
fiches des bots sont validées mais exclues des statistiques des joueurs ;
`excluded_bot_participations` en donne le compte. Les données de bans ne
comptent que si les deux listes de 5 slots et leurs tours sont complets. Les bans
n'ont pas de rang/rôle individuel attribuable dans les réponses Riot.

### Rang des bans : palier de partie (#109)

Riot ne donne pas le rang de l'auteur d'un ban : le rang d'un ban est celui de sa
**partie**. Pour une partie Solo/Flex retenue, le palier de partie est la médiane des
paliers (`IRON` … `CHALLENGER`) observés de ses joueurs, avec les mêmes rangs figés à la
partie que les picks (`rank_max_age_hours`). Ce palier est observé, jamais un MMR estimé.

- Il faut au moins 6 joueurs au palier connu sur 10 (`ban_rank_min_known_players`) ;
  `UNRANKED` et `UNKNOWN` ne comptent pas. À effectif pair, la médiane est le plus bas des
  deux paliers centraux : jamais un palier qu'aucun joueur n'a.
- Sous ce minimum, la partie est rangée sous `UNKNOWN` ; hors Solo/Flex, sous
  `UNRANKED_MODE`. Elle n'est jamais attribuée à un palier par défaut.
- Chaque draft complète compte une fois sous `ALL` et une fois sous le rang de sa partie :
  la somme des `draft_matches` des rangs (hors `ALL`) égale `draft_matches` de la couverture.
  Ne jamais additionner `ALL` et un palier.
- Le seuil `min_games` s'applique au dénominateur de chaque rang : un palier peu observé
  garde ses comptes mais un `ban_rate` nul.
- Chaque ban publie (#91) `ban_rate_lower_bound` et `ban_rate_upper_bound` (Wilson 95 %, nuls
  avec le taux) et `reliability` (`low` sous `reliability_floor` drafts du palier, même si
  `min_games` autorise le taux).
- `ban_rank_basis` (`match_median`) et `ban_rank_min_known_players` sont publiés dans le
  rapport ; la couverture ajoute `match_tier_matches` et `unknown_match_tier_matches`
  (parties Solo/Flex retenues avec ou sans palier). Un instantané antérieur se relit : ses
  bans valent `ALL`, ces champs valent vide/0.
- Les picks restent comptés au rang de chaque joueur, les bans au palier de la partie :
  les deux taux ne partagent pas exactement la même population, ce qui est publié ici.

### Paliers cumulés (#83)

Les paliers observés (`IRON` … `CHALLENGER`) partitionnent les participations classées :
l'agrégation publie en plus des paliers « X et plus », sous la même clé `rank` que les
autres populations (aucun nouvel appel Riot, aucune migration SQL) :
`IRON_PLUS`, `BRONZE_PLUS`, `SILVER_PLUS`, `GOLD_PLUS`, `PLATINUM_PLUS`, `EMERALD_PLUS`,
`DIAMOND_PLUS`, `MASTER_PLUS`. `MASTER_PLUS` réunit Master, Grand maître et Challenger ; il n'y
a pas de `GRANDMASTER_PLUS` ni de `CHALLENGER_PLUS`.

- Un palier cumulé ne contient que des participations au palier observé (donc ni `UNKNOWN`,
  ni `UNRANKED`, ni `UNRANKED_MODE`) : `IRON_PLUS` n'est pas `ALL`. Hors files Solo/Flex
  il n'y en a aucun.
- Parties, victoires, populations, bans, drafts, builds, compétences et événements d'objets
  s'additionnent entre paliers ; les builds sont cumulés **avant** la coupe à
  `max_build_variants_per_category` (une variante rare dans chaque palier peut entrer dans
  le haut du cumul), sans multiplier les clés de l'accumulateur pendant la lecture des parties.
- `bucket_matches` et le `pick_rate` comptent des parties **distinctes** : une partie dont les
  joueurs ont des paliers différents compte une fois dans « X et plus » (ce n'est donc pas
  la somme des paliers). Les bans suivent le palier de la partie (#109).
- Position, tier S/A/B/C/D, fiabilité et intervalles de Wilson sont calculés dans chaque palier
  cumulé comme dans les autres populations. `most_picked_rank` ne désigne jamais un palier cumulé.
- Ce sont des regroupements de paliers observés, jamais un MMR estimé ; ne jamais les
  additionner à `ALL`, à un palier observé ni entre eux. Le volume du rapport augmente d'autant
  de groupes, de bans et de variantes de build.
- Un instantané publié avant #83 n'a pas ces rangs : il se relit tel quel, sans erreur.

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
  secondaire, 2 runes secondaires, fragments offense/flex/défense. Page exacte,
  conservée comme preuve et pour l'import : un fragment différent crée une autre variante.
- Choix de runes (#86), dérivés de la page exacte sans nouvelle collecte, chacun avec sa
  propre population (parties à page complète), son effectif, son winrate et sa borne
  Wilson : `rune_keystone`, `rune_primary_style`, `rune_secondary_style` (`[id]`),
  `rune_secondary_pair` (`[arbre, rune, rune]`, paire triée), `rune_slot_1..3`
  (`[clé de voûte, rune]` : la rune de l'emplacement est conditionnée à sa clé de voûte) et
  `rune_shard_offense|flex|defense` (`[fragment]`). Ces catégories ne sont pas des
  recommandations : aucun assemblage de page n'est fait ici.
- Taux conditionnel `conditional_rate` (#86), en plus du `pick_rate` (taux sur toutes les
  parties de la population de la catégorie). Dénominateur : les `games` du choix parent dans
  le même groupe (champion, rôle, rang, patch, plateforme, file) — la clé de voûte
  (`rune_keystone` `[clé]`) pour `rune_slot_1..3`, l'arbre secondaire (`rune_secondary_style`
  `[arbre]`) pour `rune_secondary_pair`. Exemple : une rune à 36 parties sous une clé à 50
  parties vaut `72.0`. C'est un **pourcentage de 0 à 100**, comme `pick_rate`. `null` pour les autres catégories (leur sélection ne contient pas de parent),
  si le parent est absent ou à 0 partie, ou si l'effectif est sous `min_games` comme
  `pick_rate`. Le dénominateur est lu avant le plafond de variantes par catégorie : une ligne
  parent coupée n'invalide pas le taux d'une ligne conservée.
- `skill_order` : points Q/W/E/R normaux dans l'ordre temporel ; `special_skill_order`
  sépare les évolutions. Séquence intégrale quasi unique par partie : elle reste publiée
  comme preuve, les choix de montée ci-dessous portent les effectifs exploitables.
- Choix de montée (#87), dérivés de `skill_order` sans nouvelle collecte, chacun avec sa
  propre population, son effectif, son winrate et sa borne Wilson : `skill_start` (les
  3 premiers points, dans l'ordre ; parties avec au moins 3 points) et `skill_priority`
  (ordre dans lequel Q, W, E atteignent le rang 5, `[1|2|3, 1|2|3, 1|2|3]`). Deux sorts
  au rang 5 fixent l'ordre, le troisième étant dernier ; avec moins, la catégorie est
  absente pour la partie plutôt que devinée, la population de `skill_priority` est donc
  celle des parties où deux sorts au moins sont maximisés (parties longues). L'ultime
  est ignoré. Les champions à mécanique particulière (rang maximal ou sorts spéciaux
  différents) ne sont pas traités à part : validation sur timelines à faire. `skill_levels` expose l'ordre du point investi et son temps
  moyen, pas le niveau du champion (les points peuvent être gardés).
- `purchase_order` : achats incluant composants et consommables, avec retrait des
  achats annulés ; ce n'est pas un inventaire final reconstruit.
- `item_events` : achats, ventes, destructions et annulations regroupés par minute.

Chaque variante publie aussi `win_rate_lower_bound`, borne inférieure de Wilson à 95 %
bornée à 0–100 (le client desktop rejette toute page hors de cet intervalle), nulle sous le seuil ou sans performance publiable (Arena, toutes catégories), et depuis #91 `win_rate_upper_bound` (même règle)
et `reliability` (`low` sous `reliability_floor` parties, y compris pour Arena où seul l'effectif est publié).

Depuis #112, chaque variante publie aussi `win_rate_upper_bound` (borne supérieure de
Wilson à 95 %, mêmes bornage 0–100 et conditions de publication) et `win_rate_delta`,
écart signé en points de pourcentage entre son winrate et celui du groupe champion (même
patch, plateforme, file, rôle et rang), calculé sur les taux non arrondis. L'écart est
nul sous le seuil de la variante ou du groupe, et sans performance publiable. La
référence est le groupe entier, y compris pour une catégorie à population plus étroite
(pages de runes complètes, parties avec deux sorts au rang 5) : l'écart mesure la
variante contre le champion, pas contre sa propre catégorie. Intervalle et écart sont
descriptifs ; ils ne corrigent ni la durée ni la survie des parties.

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

## Moyennes de performance (#100)

`performance` publie, pour chaque population de `groups` (patch × plateforme × file ×
rôle × rang × champion, `ALL` compris), des moyennes post-partie lues dans le détail et
la timeline déjà chargés par la passe d'agrégation, sans requête supplémentaire.
Ce sont des agrégats descriptifs : ni note, ni benchmark, ni radar, ni MMR. Les définitions
exactes sont publiées dans `performance_method`.

| Champ | Définition |
| --- | --- |
| `participations` | Toutes les participations de la population (dénominateur de couverture) |
| `short_games_excluded` | Participations de la population dont la partie dure moins de 15 minutes (`duration < 900 s`) : écartées de toutes les moyennes, frames à 10 et 15 min comprises, mais comptées dans `participations`. Une partie de 900 s est comptée. Les autres sections (winrate, pick rate, builds) gardent ces parties (#100) |
| `games` | Participations d'une partie d'au moins 900 s dont `kills`, `deaths`, `assists`, `totalDamageDealtToChampions`, `totalMinionsKilled`, `neutralMinionsKilled`, `goldEarned` et `visionScore` sont des entiers positifs ou nuls, avec une durée stockée > 0 ; sinon la participation est écartée en bloc, rien n'est deviné |
| `kills`, `deaths`, `assists`, `damage_to_champions`, `vision_score` | Somme / `games` |
| `kda` | `(ΣK + ΣA) / max(ΣD, 1)` sur les sommes de la population, pas une moyenne de KDA par partie |
| `cs_per_min`, `gold_per_min` | Somme (CS = sbires + monstres neutres, or gagné) / somme des durées de partie en minutes |
| `frames[]` | Minutes 10 et 15 : première frame de timeline avec `minute × 60 000 ≤ timestamp < minute × 60 000 + frameInterval` ; `gold` = `totalGold`, `cs` = `minionsKilled + jungleMinionsKilled`, `xp` ; moyenne sur les participations ayant cette frame (`games` propre), parties d'au moins 900 s seulement. Une partie finie avant la minute n'y contribue pas |

Sous `--min-games`, les moyennes sont nulles et les effectifs restent publiés ; chaque
minute applique le seuil à son propre effectif. Les frames ne sont lues que si la timeline
de la participation est valide (mêmes règles que les builds). Les parties exclues (remake,
contrôles de qualité #111) n'y contribuent pas. Les parties de moins de 15 minutes sont
écartées (`short_games_excluded`), le reste des écarts est
`participations − games − short_games_excluded` (valeurs incomplètes). CS/min et or/min
restent pondérés par la durée (somme / somme des durées). Le seuil de 900 s s'applique à
toutes les files, alors que le plancher de 300 s de #111 ne concerne que Solo/Duo et Flex :
en ARAM et en Arena, `short_games_excluded` peut donc représenter une grosse part de
`participations`. Un rapport antérieur relit `performance` vide, `performance_method` vide
et `short_games_excluded` à 0.

## Matchups de lane (#123)

`matchups` publie, pour chaque champion, ses résultats contre l'adversaire qui occupait
le même rôle dans l'autre équipe. Ce sont des agrégats descriptifs de parties publiques :
aucune recommandation, aucun contre conseillé, aucune décision de draft. Les définitions
exactes sont publiées dans `matchup_method`.

| Champ | Définition |
| --- | --- |
| Files | 420 et 440 seulement : la validation y garantit un seul participant par (équipe, rôle). Les autres files, dont la normale 400, ne publient aucun matchup tant que l'appariement n'y est pas validé |
| Paire | Les deux participations d'équipes opposées ayant le même `teamPosition` (`TOP`, `JUNGLE`, `MIDDLE`, `BOTTOM`, `UTILITY`) ; un rôle `UNKNOWN`, absent ou en double n'est jamais apparié |
| Clé | patch × plateforme × file × rôle × `rank = ALL` × `champion_id` × `opponent_champion_id` ; une ligne par sens (`A contre B` et `B contre A`) |
| `rank` | `ALL` seulement : le rang observé est individuel, il rendrait les deux sens non complémentaires. Un rang de partie reste à définir |
| `games`, `wins`, `losses` | Parties appariées ; victoires et défaites de `champion_id`. Les deux sens d'une paire ont le même effectif et des victoires complémentaires |
| `win_rate`, `win_rate_lower_bound` | `wins / games × 100` et borne inférieure de Wilson à 95 %, toutes deux nulles sous `--min-games` ; l'effectif reste publié |

La couverture ajoute `lane_matchup_participations` : participations appariées à un
adversaire de lane dans le périmètre (0 hors files 420 et 440). Les parties exclues
(remake, contrôles de qualité #111) n'y contribuent pas. Aucune synergie de duo n'est
encore publiée. Un rapport antérieur relit `matchups` vide et `matchup_method` vide.

## Publication, stockage et exploitation

`champion_stats_snapshot` contient une tête : `source_snapshot_at`, `published_at`,
`storage_version`, `report`. En stockage v2, `report` contient les métadonnées ;
les neuf listes (`coverage`, `groups`, `bans`, `builds`, `skill_levels`, `item_events`,
`splits`, `performance`, `matchups`) sont dans `champion_stats_snapshot_chunks`, ordonnées par
section et `chunk_index`. Les migrations `0012`, `0013` et `0014` autorisent les sections
`performance` (#100), `matchups` (#123) et `splits` (#119).
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

### Recalcul par lots (#89)

```sh
cargo run -p olc-collector --release -- aggregate --sync-static --watch
```

Décision du 4 octobre 2026 : le recalcul horaire `aggregate --watch` est **incrémental par
défaut**. `--full` force le recalcul complet à chaque heure (`--watch --full`) ; il est
exclusif avec `--incremental`. `--incremental` reste accepté : sans effet avec `--watch`
(déjà le défaut), il active le mode par lots pour un `aggregate` ponctuel, dont le
défaut reste le recalcul complet (`--full` y est explicite et équivaut au défaut).

Le mode incrémental découpe le calcul en lots patch/plateforme/file. Chaque section du
rapport est indexée et triée d'abord par ce périmètre et aucune règle (tiers, rang le
plus joué, coupe à 20 variantes, médiane des écarts de rang) ne mélange deux
périmètres : concaténer les lots redonne exactement le recalcul complet. Chaque lot est
calculé seul, ses morceaux écrits, puis sa mémoire libérée : la mémoire est bornée par
le **plus gros lot**, pas par la base (sur la copie de recette, EUW1 Solo du patch
courant porte encore la moitié des parties).

Un lot n'est relu que si son empreinte change depuis la dernière publication. Elle
couvre : les parties du lot sous les filtres et leurs timelines (identifiants, état et
`xmin`, donc toute modification de ligne) ; les observations de rang de sa file dont la
date est à moins de l'écart maximal (+1 h d'arrondi) d'une de ses parties, seules
capables de changer un rang figé (#80) ; le contenu du catalogue d'objets retenu pour
son patch (version et classement : objets complets, bottes, trinkets, transformations),
pour qu'une même version republiée avec d'autres fiches (`catalog --refresh` ou
`--rebuild`, nouveau normaliseur, complément CommunityDragon obtenu après coup) relise le
patch dès qu'une étape de build peut changer, sans relecture si le classement est
inchangé ; les
paramètres publiés dans l'en-tête ; l'identité du binaire (chemin, taille, date), pour
qu'une nouvelle version recalcule tout une fois sans numéro à incrémenter. Tant
que la collecte observe des rangs dans une région, ses lots classés récents sont donc
relus à chaque heure ; le gain porte sur les anciens patches et les lots inactifs.

La migration `0018` ajoute `champion_stats_snapshot_lots` (empreinte, compteurs
additifs et nombre de morceaux de chaque lot) et rattache chaque morceau à son lot
(`lot_patch`, `lot_platform_id`, `lot_queue_id`, `NULL` en recalcul complet). Un lot
recalculé ou disparu est supprimé avec ses morceaux (cascade) ; un lot dont les
morceaux ne correspondent plus au nombre enregistré est recalculé. Le recalcul complet
supprime tous les lots. La migration crée aussi les index des pages d'un lot et de la
fenêtre des observations ; sur une grosse base, l'index de `matches` bloque les
écritures de la collecte le temps de sa construction.

Tout reste dans une seule transaction `REPEATABLE READ` sous le même verrou de calcul :
l'en-tête est écrit en premier (un instantané plus récent fait échouer le calcul), puis
réécrit avec les compteurs cumulés ; un échec conserve la publication et les lots
précédents. L'en-tête est identique à celui du recalcul complet. Seul l'ordre des lots
entre eux suit l'ordre d'écriture ; l'API, qui filtre toujours un patch, une plateforme
et une file, n'en dépend pas. `--json` affiche `lots`, `recomputed_lots`,
`reused_lots` et l'en-tête publié (`report`, listes vides : elles se lisent dans
l'instantané).

Pour une nouvelle section : l'indexer par périmètre (sinon elle ne peut pas être
calculée par lots), l'écrire dans `snapshot::write_sections` (commun aux deux modes)
et la classer dans `LotCounts::of` (compteur additif) ou la marquer ignorée ; la
déstructuration exhaustive de `incremental.rs` refuse de compiler sinon. Le test
`le_cumul_des_lots_reproduit_exactement_le_recalcul_complet` compare les deux modes.
Une section qui lit une nouvelle source doit aussi la faire entrer dans l'empreinte
(`current_lots`) ; un champ ajouté à `ItemCatalog` ne compile pas tant qu'il n'est pas
repris par `ItemCatalog::fingerprint`.
Mesures et vérification sur la copie de recette :
[recette du recalcul par lots](../../docs/recettes/2026-10-04-agregation-par-lots.md).

Tables : `collection_runs`, `collection_jobs`, `seed_players`, `run_discoveries`,
`run_matches`, `matches`, `excluded_matches`, `match_timelines`, `participant_rank_observations`,
`collection_campaigns`, `campaign_runs`, `static_data_releases`, `static_data_manifest`,
`champion_stats_snapshot`, `champion_stats_snapshot_chunks`, `champion_stats_snapshot_lots`. Les détails et timelines restent complets en JSONB, et
les observations de rang gardent leur historique daté, pendant les durées de rétention
ci-dessous.

Les pages de 25 parties bornent les données brutes simultanément lues, **pas toute la
mémoire** : en recalcul complet, les compteurs de variantes et événements restent en
RAM jusqu'à la publication ; le mode incrémental (défaut de `--watch`) les borne au
plus gros lot (#89).
La recette mesure temps et mémoire ; un passage à très grande échelle nécessitera
une stratégie de calcul/pagination supplémentaire. Les groupes trop petits restent
hors classement, même après une longue collecte.

## Rétention des données personnelles (#99)

```sh
cargo run -p olc-collector --release -- purge --json
cargo run -p olc-collector --release -- purge --watch   # immédiatement puis chaque heure
```

`purge` applique deux durées (celle des parties brutes vaut aussi pour le cache négatif `excluded_matches`), comptées depuis l'enregistrement de la donnée
(`fetched_at`, `observed_at`, dernière activité de l'exécution). Valeurs par défaut
**proposées, à valider** avant exploitation publique :

| Donnée | Durée | Après expiration |
| --- | --- | --- |
| Parties brutes : `matches`, `match_timelines` et leurs liens `run_matches` | 90 jours (`--raw-match-days`, `OLC_RETENTION_RAW_MATCH_DAYS`) | Supprimées, sauf partie liée à une exécution `running` |
| Identifiants et profil des participants (PUUID, Riot ID, icône, niveau) dans le JSONB des parties et timelines | 30 jours (`--identifier-days`, `OLC_RETENTION_IDENTIFIER_DAYS`) | `puuid`, `summonerId`, `summonerName`, `riotIdGameName`, `riotIdName`, `riotIdTagline`, `profileIcon`, `summonerLevel` retirés des participants ; `metadata.participants` remplacés par `""` |
| `participant_rank_observations` | 30 jours | Supprimées |
| `seed_players`, `run_discoveries`, `collection_jobs` d'une exécution sans activité depuis 30 jours | 30 jours | Supprimés ; `run_matches.seed_puuid` vidé |
| `excluded_matches` (cache négatif, #90) | 90 jours comme les parties brutes (`--raw-match-days`), comptés depuis `excluded_at` | Supprimées par la même commande ; la partie, si elle est redécouverte, est retéléchargée puis rejugée. Faits de partie sans PUUID ni détail (identifiant de partie, file, patch, date, durée) : pas de donnée personnelle de joueur, mais la table ne grossit pas sans limite |

Pourquoi ces valeurs : l'agrégation ne lit que les rangs observés depuis moins de
24 h et ne recalcule par défaut que les deux derniers patches (environ 4 semaines) ;
30 jours laissent la reprise d'une collecte et le contrôle d'une recette, 90 jours
permettent de recalculer six patches après une correction de l'agrégation. Le niveau de compte
(`summonerLevel`) part avec les identifiants : donnée de profil que l'agrégation ne lit pas. Les parties
pseudonymisées restent agrégées (champions, rôles, objets, timelines) ; leur rang
n'est plus attribuable, ce qui est déjà le cas après 24 h. Les bots (`BOT`, zéros)
gardent leur marqueur, nécessaire au comptage des files coop.

Conséquences : une exécution en pause depuis plus de 30 jours perd ses travaux ; sa
reprise se termine aussitôt en `incomplete` ; `report <id>` d'une exécution purgée ne retrouve
plus le détail de ses travaux (le bilan final reste dans `collection_runs.report`).
Une partie supprimée peut être retéléchargée si une nouvelle collecte la redécouvre.
La purge est idempotente, travaille par lots de 200 lignes et ne fait aucun appel
Riot. La migration `0010` ajoute des index GIN sur les participants du JSONB : sur une
base volumineuse, appliquer la première migration (premier lancement d'une commande
ou de l'API) à un moment calme. `--watch` reprend l'ordonnanceur horaire d'`aggregate` ; aucun superviseur
système n'est installé. Code de sortie 3 si Ctrl+C interrompt une purge ponctuelle.

Export et effacement d'un joueur sur demande : routes authentifiées de l'API
(`POST /v1/privacy/export` et `/v1/privacy/erase`, [README de l'API](../api/README.md)),
implémentées dans `src/privacy.rs` et partagées avec la purge.

## Conformité, données personnelles et validation

Clé côté service uniquement. PUUID/Riot ID restent dans la base privée, jamais dans
les rapports publiables. Les données brutes ne sont pas un export public. Cette
fonctionnalité exploite des parties terminées et des données statiques officielles,
sans action dans le jeu. Rétention, export et effacement : section précédente (#99).
Restent hors de ce service : CGU, politique de confidentialité, mentions légales,
hébergement UE documenté et clé de production (#2).

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
