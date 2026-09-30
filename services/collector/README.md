# services/collector

Collecteur Riot API d'Open LoL Companion (ticket #17, section 10.1 du [cahier des charges](../../docs/cahier-des-charges.md)).
Binaire Rust autonome : il n'a besoin ni de l'app desktop ni du client LoL, et tourne sur Windows, macOS et Linux.

Prototype actuel : **parties Ranked Solo/Duo EUW** (`EUW1`, file 420), avec leur détail et leur timeline, stockées dans PostgreSQL.
L'agrégation (builds, winrates, tierlist) relève de #18.

## Chaîne de collecte

1. **Joueurs de départ** : `league-v4` (`euw1`), par rang et division, page après page, jusqu'à `--seeds-per-division` joueurs par strate.
2. **Historiques** : `match-v5 by-puuid/ids` (`europe`), file 420, bornés par la fenêtre, au plus `--max-matches-per-seed` parties par joueur.
3. **Détails** : `match-v5 matches/{id}`. Chaque partie est contrôlée (identifiant, `platformId = EUW1`, `queueId = 420`, `gameStartTimestamp` dans la fenêtre, 10 participants) avant d'être retenue.
4. **Timelines** : `match-v5 matches/{id}/timeline`, pour chaque partie retenue.

Ordre de traitement : premières parties de chaque joueur, puis deuxièmes, etc., en alternant les strates.
L'échantillon ne dépend donc pas d'un seul rang ni de quelques gros joueurs.
Il valide la chaîne ; il ne prétend pas représenter toute la population EUW.

> Le rang enregistré dans `seed_players` est un **instantané au moment de la collecte**.
> Ce n'est ni le rang du joueur au moment de ses parties, ni celui des neuf autres participants.

## Prérequis

- Rust stable (voir [`docs/DEMARRAGE.md`](../../docs/DEMARRAGE.md)).
- PostgreSQL 14 ou plus. Le plus simple : `docker compose -f services/collector/docker-compose.yml up -d`, sinon une installation locale (Postgres.app ou Homebrew sur macOS, installeur officiel sous Windows).
- Une clé Riot (compte [Developer Portal](https://developer.riotgames.com)). La clé de développement **expire après 24 h**.

## Configuration

Copiez `services/collector/.env.example` en `.env` à la racine du dépôt (ou exportez les variables) :

| Variable | Rôle |
| --- | --- |
| `RIOT_API_KEY` | Clé Riot, envoyée uniquement dans l'en-tête `X-Riot-Token`. Jamais journalisée. |
| `DATABASE_URL` | Connexion PostgreSQL, ex. `postgres://postgres:postgres@localhost:5432/olc` |
| `RUST_LOG` | Facultatif : niveau des journaux (`info` par défaut, `olc_collector=debug` pour le détail) |
| `OLC_TEST_DATABASE_URL` | Tests uniquement : base d'administration où créer des bases jetables |

`.env` est ignoré par git.

## Utilisation

```bash
cargo run -p olc-collector --release -- migrate          # crée ou met à jour les tables
cargo run -p olc-collector --release -- run              # nouvelle exécution : 1 000 parties par défaut
cargo run -p olc-collector --release -- resume 1         # reprend l'exécution n° 1
cargo run -p olc-collector --release -- report 1         # bilan (ajouter --json pour le format JSON)
```

Options de `run` (valeurs par défaut) :

| Option | Défaut | Rôle |
| --- | --- | --- |
| `--target` | 1000 | Parties distinctes à retenir |
| `--tiers` | `GOLD,PLATINUM,EMERALD` | Rangs de départ (IRON à DIAMOND) |
| `--divisions` | `I,II,III,IV` | Divisions de départ |
| `--window-days` | 14 | Fenêtre, figée au lancement et conservée à la reprise |
| `--seeds-per-division` | 15 | Joueurs de départ par rang et division |
| `--max-matches-per-seed` | 10 | Parties découvertes au plus par joueur |
| `--call-budget` | 3000 | Appels Riot au plus pour toute l'exécution |
| `--concurrency` | 2 | Requêtes simultanées (aussi pour `resume`) |
| `--max-duration-mins` | aucune | Arrêt propre après cette durée (aussi pour `resume`) |

Options de `resume` : `--call-budget <total>` pour relever le budget, `--retry-failed` pour relancer les travaux en échec (après une panne réseau, par exemple).

Avec les valeurs par défaut, compter environ 2 200 appels (12 pages de classement, environ 180 historiques, 1 000 détails, 1 000 timelines).
Avec une clé de développement (100 requêtes / 2 min), il faut **au moins 45 minutes**.

Codes de sortie : `0` cible atteinte, `2` exécution terminée sous la cible, `3` exécution en pause (arrêt demandé, budget, durée ou clé refusée), `1` erreur.

## Arrêt et reprise

Tout l'avancement vit dans PostgreSQL (`collection_jobs`). Chaque résultat est écrit dans la même transaction que l'état de son travail et le compteur d'appels.

- **Ctrl+C** : plus aucun nouveau travail, attente des requêtes en cours, exécution en `paused`.
- **Arrêt brutal** (crash, coupure) : au redémarrage, les travaux restés `running` repartent. Un détail déjà enregistré n'est pas retéléchargé ; seule l'étape interrompue est refaite. Une requête reçue mais non enregistrée au moment du crash peut être envoyée une seconde fois : l'unicité est garantie en base, pas un téléchargement « exactement une fois ».
- **Clé refusée (401/403)** : collecte suspendue, progression conservée. Mettez à jour `RIOT_API_KEY` puis `resume`.
- **Relance d'une exécution terminée** : aucun appel.
- **Nouvelle exécution** : les parties déjà en base sont retenues sans être retéléchargées (`run_matches.already_present`).

Un verrou PostgreSQL empêche deux collecteurs de travailler en même temps sur la même base.

## Quotas et erreurs

Tous les appels, nouvelles tentatives comprises, passent par un seul gestionnaire de quotas :

- fenêtres applicatives par hôte de routage (`euw1`, `europe`) et fenêtres par méthode ;
- limites et compteurs repris des en-têtes `X-App-Rate-Limit(-Count)` et `X-Method-Rate-Limit(-Count)` ; avant la première réponse, limites prudentes d'une clé de développement (20 / 1 s, 100 / 2 min).

| Situation | Comportement |
| --- | --- |
| Quota momentanément épuisé | Attente avant l'envoi |
| `429` | Pause de `Retry-After` sur la portée indiquée (application, méthode, service). Sans en-tête : pause prudente de 10 s, sur tout l'hôte si le type manque. Ne compte pas comme une tentative |
| Timeout, erreur réseau, `5xx`, JSON invalide | Nouvel essai avec délai croissant (2 s → 5 min, avec une part aléatoire), 5 tentatives au plus, puis échec |
| `401` / `403` | Suspension de la collecte |
| `400` | Échec immédiat du travail |
| Timeline `404` | Deux revalidations espacées (10 puis 20 min), puis timeline `unavailable` |
| Détail `404` | Deux revalidations, puis échec |

Une erreur réseau ou serveur ne devient jamais une timeline « indisponible ».
Les messages d'erreur enregistrés ne contiennent ni clé, ni URL, ni PUUID.

## Données

| Table | Contenu |
| --- | --- |
| `collection_runs` | Paramètres figés, fenêtre, statut, appels, bilan JSON |
| `seed_players` | Joueurs de départ et rang observé (instantané) |
| `collection_jobs` | Travaux : étape, état (`pending`, `running`, `retry_wait`, `done`, `failed`), tentatives, prochaine reprise, erreur filtrée, résultat |
| `run_discoveries` | Chaque mention d'une partie dans un historique (provenance, doublons) |
| `matches` | Détail complet (JSONB) et colonnes indexées : plateforme, file, version, patch, début, durée, remake |
| `match_timelines` | Timeline complète (JSONB) ou `unavailable` |
| `run_matches` | Échantillon retenu par chaque exécution, y compris les parties déjà en base |

Les réponses Riot sont conservées en entier pour recalculer les données dérivées (#18) sans retélécharger.
Le JSONB ne garde pas les octets exacts reçus (ordre des clés, espaces), seulement leur contenu.

## Conformité et données personnelles

- La clé ne vit que dans l'environnement du service ; l'app desktop n'y a jamais accès.
- Les PUUID et Riot ID sont des données personnelles. Ils restent en base et n'apparaissent pas dans les journaux.
- Les réponses brutes ne doivent pas être publiées telles quelles : seuls les agrégats de #18 ont vocation à être exposés.
- La durée de conservation et la suppression des données d'un joueur restent à définir avant la mise en production (clé de production, #2).

## Tests

```bash
cargo test -p olc-collector                                   # unitaires ; tests PostgreSQL ignorés
OLC_TEST_DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres \
  cargo test -p olc-collector                                 # + tests d'intégration PostgreSQL
```

Les tests n'appellent jamais Riot : un faux serveur en mémoire renvoie des réponses synthétiques conformes aux schémas, sans donnée de joueur réelle.
Les tests d'intégration créent puis suppriment une base jetable par test.
