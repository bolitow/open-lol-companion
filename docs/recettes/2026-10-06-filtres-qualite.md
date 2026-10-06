# Filtres de qualité #111 — preuves synthétiques du 6 octobre 2026

Ce lot complète la documentation et les tests de la politique déjà en place. Il ne
change ni les seuils, ni la priorité `short_game` avant AFK, ni le traitement des
champs absents. Périmètre : cahier des charges §9/§10.2 et
[ticket #111](https://github.com/bolitow/open-lol-companion/issues/111).

La copie `olc_nuit_d` de Matthieu n'est pas disponible dans cet environnement.
Les tests ci-dessous utilisent uniquement des données synthétiques ; ils ne
remplacent pas sa recette et ne suffisent pas à clore le ticket. Les chiffres de
17 112 parties, 37 `short_game`, 602 `afk` supplémentaires et 0 `early_departure`
restent ceux de la sonde SQL historique du 4 octobre, sans nouvelle confirmation
de bout en bout ici.

## Contrat vérifié

La validation de forme et des remakes intervient avant les contrôles de qualité.
En 420/440 seulement, la durée est ensuite contrôlée (`short_game`), puis les types
des champs actifs `wasAfk`/`timePlayed`, puis les motifs `afk` et `early_departure`.
Une courte partie de forme valide avec un champ de qualité mal typé reste donc
`short_game`. Sur une partie assez longue, un mauvais type actif produit
`invalid_match`, même si un autre participant est AFK. Chaque partie n'a qu'un motif.

Un champ absent n'est pas jugé. Les seuils par défaut restent 300 s et 80 %, avec
AFK exclu ; 0 désactive chaque seuil et `--keep-afk` désactive le contrôle AFK.
Ces options désactivent aussi la validation du champ correspondant. Une reddition
normale reste valide et exactement 80 % joués est conservé.

710/3130 ne reçoivent aucun format présumé : `unknown_queue` les isole avant toute
contribution, même si leur réponse annonce `CLASSIC`. Les files identifiées et leurs
formats sont centralisés dans `services/collector/src/queues.rs`. Les données brutes
restent stockées ; le filtrage n'en fait ni du classé ni un nouveau mode connu.

430/480 acceptent les miroirs dans un format 5 contre 5 cohérent. Les deux
participations et leurs builds restent comptés. Le pick rate compte la partie
distincte une fois ; le winrate du champion compte chacune des deux participations.
Ces files ne publient aucun matchup de lane, avec ou sans miroir : l'appariement
reste limité à 420/440 et la couverture appariée vaut 0 ailleurs.

## Preuves automatiques

| Cas | Preuve | Origine |
| --- | --- | --- |
| Partie classée courte, départ précoce, reddition normale | Tests de l'accumulateur et PostgreSQL `aggregation` | Tests existants conservés |
| Exactement 80 % joué et une seconde en dessous | `exclut_les_parties_classees_avec_un_depart_precoce` (1440/1800 et 1439/1800) | Test existant conservé |
| Champ absent, AFK avéré, types invalides sur partie longue, priorité AFK avant départ | Tests de l'accumulateur | Tests existants conservés |
| Courte vs longue avec `wasAfk` ou `timePlayed` mal typé | `short_games_skip_malformed_quality_fields_until_duration_threshold` | Nouveau test sur logique existante |
| Types non lus lorsque les contrôles sont désactivés | `les_seuils_sont_configurables_publies_et_desactivables` | Fixture supplémentaire |
| 710/3130 rejetées sans contribution à Solo/Flex, builds et matchups compris | `une_file_inconnue_est_isolee_par_une_exclusion_explicite` | Assertions renforcées |
| Miroirs 430/480 valides : deux participations, builds et aucune lane appariée | `normal_mirrors_keep_participations_and_builds_without_lane_matchups` | Nouveau test sur logique existante |
| Compteurs et options dans le snapshot, `--keep-afk`, contrôles désactivés | Intégrations PostgreSQL et vraie CLI `aggregation` | Assertions renforcées ; sortie texte vérifiée |
| Options et trois compteurs qualité lus par tierlist/builds, stockage v1 et v2 | `api_serves_quality_counters_and_options_in_both_snapshot_formats` | Nouveau test de lecture de snapshots synthétiques |
| Ancien snapshot sans options qualité : 0/0/false sans inventer de filtres | `legacy_snapshot_does_not_invent_quality_options` | Nouveau test API, v1 et v2 |

Les intégrations demandent explicitement `OLC_TEST_DATABASE_URL`. Les helpers créent
et suppriment leurs propres bases jetables `olc_test_<pid>_<compteur>` ou
`olc_api_test_<pid>_<compteur>` ; aucune agrégation ne cible une base métier.
La CLI de test reçoit l'URL jetable, une clé Riot vide et un répertoire de travail
temporaire. Les preuves d'exécution et les éventuelles mutations contrôlées sont
consignées dans `work/ticket111-20261006/report.md` (rapport local).

## Recette réelle restante sur la copie de Matthieu

Cette liste concerne exclusivement la copie de recette `olc_nuit_d`. La commande
`aggregate` publie un nouvel instantané : elle doit donc viser cette copie, une fois
son accès disponible, jamais une base utilisateur en production.

- [ ] Confirmer le périmètre et les comptes bruts de la copie, sans données nominatives.
- [ ] Exécuter `olc-collector aggregate --all-stored` en build release et conserver
  les comptes sources/inclusions/exclusions ainsi que la durée du calcul.
- [ ] Comparer les motifs à la sonde historique du 4 octobre, sans en présumer
  l'identité si les parties de la copie ont changé.
- [ ] Contrôler la sortie texte, le snapshot publié et `meta.exclusions`/les options
  de tierlist et builds dans l'API ; vérifier qu'ils décrivent la même publication.
- [ ] Vérifier `--keep-afk` puis les seuils configurés et les contrôles désactivés sur
  cette copie ; publier les paramètres réellement utilisés avec chaque bilan.
- [ ] Vérifier les effectifs par file et le rejet de 710/3130 si elles sont présentes ;
  aucun effet sur 420/440 et aucun matchup pour 430/480.
- [ ] Consigner le résultat macOS et la CI Windows avant livraison, sans revendiquer
  la validation de la copie sur les seules fixtures synthétiques.
