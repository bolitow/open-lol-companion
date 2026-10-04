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
ces populations. Les files 420/440 utilisent leur propre classement récent (24 h
au début du calcul) ; les autres ont `UNRANKED_MODE`. `UNKNOWN` et `UNRANKED` restent
distincts. Les rôles sont `TOP/JUNGLE/MIDDLE/BOTTOM/UTILITY/UNKNOWN` ; aucun rôle n'est
inventé à partir des objets ou du rang.

| Mesure | Définition et limites |
| --- | --- |
| `games`, `wins`, `losses` | Participations, pas toujours matchs distincts : les modes autorisant plusieurs exemplaires d'un champion peuvent contribuer plusieurs fois par match |
| `population` | Toutes les participations du même patch/plateforme/file/rôle/rang |
| `win_rate` | Victoires / participations du champion × 100 |
| `pick_rate` | Participations du champion / population × 100 ; part des sélections dans ce groupe |
| `win_rate_lower_bound` | Borne inférieure de Wilson à 95 %, estimation descriptive de l'incertitude binomiale |
| `position`, `tier` | Borne Wilson décroissante, puis taux, effectif et ID ; S/A/B/C/D par tranches 10/30/60/90/100 %, au moins 5 champions éligibles dans le groupe |
| `most_picked_rank` | Rang connu avec le plus de participations de ce champion ; dépend des effectifs collectés par rang |
| `bans` | Tableau séparé patch/plateforme/file : matchs bannissant le champion / drafts complètes ; un double ban ne compte qu'une fois |

Sous le seuil, taux champion/build et classement sont nuls ; les comptes restent
visibles. Le taux de ban demande au moins ce nombre de drafts complètes. La borne
Wilson et les tiers ne corrigent pas les biais d'échantillonnage ni les dépendances
entre parties d'un même joueur. Le booléen `win` d'Arena ne signifie pas nécessairement
une première place. Les parties normales/PvE et Arena ne sont jamais mélangées à SoloQ.

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
d'items Arena (`item`, `final_items`, `trinket`, `purchase_order`) ne publient ni
victoires ni winrates (`wins`/`win_rate` nuls, `performance_available: false`) ; leur
tri ne dépend pas des victoires. Aucun taux d'augment n'est produit. Au plus 20 variantes par catégorie/groupe sont
publiées, par popularité, sans modifier leur dénominateur ; `omitted_build_variants`
annonce les variantes supplémentaires conservées seulement dans les sources brutes.

- `final_items` : ensemble trié d'items distincts des slots 0–5 ; `item` donne chaque
  item individuel, au plus une fois par participation ; `trinket` correspond au slot 6.
- `summoner_spells` : paire d'identifiants, indépendante de l'ordre D/F.
- `runes` : 11 identifiants ordonnés — arbre principal, 4 runes principales, arbre
  secondaire, 2 runes secondaires, fragments offense/flex/défense.
- `skill_order` : points Q/W/E/R normaux dans l'ordre temporel ; `special_skill_order`
  sépare les évolutions. `skill_levels` expose l'ordre du point investi et son temps
  moyen, pas le niveau du champion (les points peuvent être gardés).
- `purchase_order` : achats incluant composants et consommables, avec retrait des
  achats annulés ; ce n'est pas un inventaire final reconstruit.
- `item_events` : achats, ventes, destructions et annulations regroupés par minute.

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
les six listes (`coverage`, `groups`, `bans`, `builds`, `skill_levels`, `item_events`)
sont dans `champion_stats_snapshot_chunks`, ordonnées par section et `chunk_index`.
Chaque morceau contient au plus 512 entrées et 1 Mio de JSON sérialisé avant conversion
PostgreSQL. Une entrée individuelle dépassant cette borne fait échouer la publication,
sans supprimer de statistiques. Le rapport JSON public et le CLI restent au schéma 2.
Les lecteurs doivent lire tête et morceaux en une seule requête ou dans une transaction
`REPEATABLE READ`, filtrer les morceaux puis reconstruire les listes côté application.
Ne pas reconstituer le rapport entier en JSONB SQL : sa limite interne est de 256 Mio.
La migration conserve les anciens rapports complets en stockage v1 jusqu'au prochain
calcul réussi ; la lecture API des deux formats est livrée séparément par le #19. Un ancien écrivain est refusé après
le passage en v2, pour éviter un mélange silencieux de versions.

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
