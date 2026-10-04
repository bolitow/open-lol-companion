# Rapport de la nuit du 4 octobre 2026 — tickets livrés en PR brouillon

Généré le 04/10/2026 à 07:31. Toutes les PR sont en brouillon, une branche par ticket. Chaque PR cible sa branche de base : seules les premières de chaque pile (#131, #132, #133, #134) ciblent `main`, les autres ciblent la PR précédente de leur pile. Fusionner chaque pile du bas vers le haut : GitHub recible automatiquement vers `main` quand la base est fusionnée et supprimée.

**43 PR livrées, 0 ticket(s) non livrable(s).**

## Lane A — Agrégation : populations, tiers, builds, nouvelles statistiques (branches empilées)

| Ticket | PR | Lien | Base | Verdict de la revue stricte | Réserves | Questions |
|---|---|---|---|---|---|---|
| #90 | [#131](https://github.com/bolitow/open-lol-companion/pull/131) feat(collector): ne demander les rangs que pour les parties classées valides (#90) | Refs | `main` | OK avec réserves (la liste 420/440 existe aussi en littéral dans aggregation/mod | 4 | 3 |
| #80 | [#136](https://github.com/bolitow/open-lol-companion/pull/136) fix(agregation): figer le rang de chaque participation à la date de sa partie (#80) | Refs | `feat/90-budget-rangs` | OK avec réserves | 8 | 6 |
| #81 | [#143](https://github.com/bolitow/open-lol-companion/pull/143) feat(collector): builds par étapes (départ, bottes, core, objets 4 à 6) avec borne Wilson (#81) | Refs | `feat/80-rang-fige-partie` | OK avec réserves | 6 | 3 |
| #111 | [#152](https://github.com/bolitow/open-lol-companion/pull/152) feat(collector): exclure les parties classées AFK, départ précoce ou très courtes (#111) | Refs | `feat/81-builds-par-etapes` | OK avec réserves | 5 | 4 |
| #84 | [#156](https://github.com/bolitow/open-lol-companion/pull/156) feat(collector): calculer le pick rate des champions par partie (#84) | Refs | `feat/111-filtres-parties` | OK avec réserves | 8 | 5 |
| #109 | [#161](https://github.com/bolitow/open-lol-companion/pull/161) feat(collector): ban rate par palier de partie et route /v1/bans pour la draft (#109) | Refs | `feat/84-pick-rate-par-partie` | OK avec réserves | 5 | 5 |
| #85 | [#171](https://github.com/bolitow/open-lol-companion/pull/171) feat(collector): tier sans répartition forcée, lissage vers la moyenne du compartiment et seuils absolus (#85) | Refs | `feat/109-ban-rate-par-palier` | OK avec réserves | 8 | 3 |

### PR #131 (ticket #90)

Questions ouvertes :
- Faut-il unifier la liste 420/440 de aggregation/model.rs (rank_for) avec RANKED_QUEUE_IDS ? Je ne l'ai pas fait pour ne pas toucher à la lane agrégation et éviter les conflits entre branches empilées.
- Les jobs participant_rank déjà créés pour d'anciennes parties hors 420/440 dans la base de recette restent en attente : faut-il une purge ou un passage en 'done' ? Je ne l'ai pas fait, c'est une décision d'exploitation.
- Le reste de #90 (ordre de réservation détails avant rangs dans storage.rs:437, cache négatif par match_id, campagne queue_id 0) est-il à faire dans une branche suivante de la lane ?

Réserves de la revue stricte :
- La liste 420/440 existe encore en littéral à trois endroits en plus de RANKED_QUEUE_IDS (services/collector/src/model.rs:83) : services/collector/src/storage.rs:776 (`for queue_id in [420, 440]` dans store_participant_ranks, même fichier que l'import de la constante), services/collector/src/aggregation/model.rs:568 (rank_for) et :618 (validate). Si une liste change sans les autres, elles divergent. L'auteur ne signale que l'agrégation, pas storage.rs:776. Le mieux serait d'utiliser RANKED_QUEUE_IDS au moins dans storage.rs, ou d'ouvrir un ticket pour l'unification.
- Le ticket #90 n'est que partiellement traité. Restent l'ordre de réservation détails avant rangs (storage.rs:437-438), le cache négatif des parties exclues (collector.rs:441) et la campagne queue_id 0 (main.rs:380-418). L'auteur le signale bien. La future PR doit donc utiliser `Refs #90` et non `« Closes » du ticket 90`.
- Les jobs participant_rank déjà en attente dans collection_jobs pour des parties 450 ou des remakes, créés par des exécutions antérieures (par exemple sur olc_nuit_a1), consommeront encore du budget à la reprise. Ce diff ne contient ni migration ni commande de nettoyage. L'auteur le signale ; il faudra le traiter avec les autres volets de #90.
- Je n'ai pas reproduit la phase rouge du TDD : la consigne interdit de modifier des fichiers et `git stash` est interdit. L'arithmétique des comptes annoncés tient : 4 parties × 10 PUUID = 40 au lieu de 20, et 3 × 10 = 30 au lieu de 10. Les tests discriminent bien : le remake est en 420 dans le premier test et en 440 dans le second, il est donc testé indépendamment du filtre de file. Et comme job_key = puuid, les assertions `LIKE 'aram-%'` et `NOT LIKE 'solo-%'` portent bien sur les jobs de rang.

### PR #136 (ticket #80)

Questions ouvertes :
- Écart maximal par défaut : 168 h, comme suggéré par le ticket, ou 336 h ? Sur olc_nuit_a1, 162 871 des 171 120 participations Solo/Flex ont au moins une observation. Avec 168 h, 129 926 reçoivent un rang (environ 80 % des participations observées) ; avec 336 h, ce serait 151 535 (environ 93 %). L'écart médian est de 58,8 h et le p90 de 283 h. Cette base est biaisée, car ses observations ne couvrent que le 1er au 3 octobre pour des parties remontant à juillet.
- Départage à écart égal : j'ai retenu l'observation la plus ancienne, en général celle d'avant la partie. Faut-il plutôt préférer celle d'après la partie ?
- Faut-il stocker le rang dès la collecte dans une table du type match_participant_rank, comme le recommande le ticket, plutôt que de le recalculer depuis les observations à chaque agrégation ? Cette livraison donne déjà un résultat déterministe sans migration.
- Extension rank_basis (médiane des paliers observés, couverture minimale de 6 sur 10) : non livrée. Faut-il un ticket séparé ?
- Le desktop ne lit que trois champs de méta. Faut-il un ticket pour afficher la part UNKNOWN et les écarts (rank_gap_*) ?
- Le cache de collecte de 24 h (services/collector/src/storage.rs:752-765, README ligne 63, main.rs:164), cité par le ticket, relève de #90 et n'est pas modifié ici.

Réserves de la revue stricte :
- Écart avec la recommandation du ticket : le rang n'est pas stocké à la collecte (pas de table du type match_participant_rank ni d'écart persistant). Il est recalculé à chaque agrégation à partir de l'observation la plus proche. C'est conforme au périmètre P0 « Agrégation » fixé pour ce livrable, et recalculer des données inchangées redonne bien le même résultat. En revanche, ce choix n'est pas signalé dans la partie Vigilance. Il faudrait l'écrire noir sur blanc dans la PR ou dans un commentaire du ticket #80, en précisant qu'un rang peut encore changer si une observation plus proche arrive après coup (par exemple, une observation antérieure à la partie remplacée par une observation postérieure plus proche).
- services/collector/src/aggregation/storage.rs:73-76 : `ORDER BY abs(extract(epoch …))` ne peut pas exploiter l'index (platform_id, puuid, queue_id, observed_at DESC). Toutes les observations d'un joueur dans une file sont donc triées pour chaque participation. Sur olc_nuit_a1, le coût mesuré reste stable (316 s contre 327 s), mais il grossira avec l'historique d'observations en production. À surveiller, ou à borner plus tard avec `observed_at BETWEEN game_start ± max_age`.
- Arrondi SQL : `round(abs(...))::bigint` peut faire passer sous la borne incluse une observation située 0,4 s au-delà de rank_max_age_hours. L'effet est négligeable, mais il vaudrait mieux le documenter ou utiliser `ceil`.
- Compatibilité des anciens instantanés : un seul test unitaire, sur `Coverage` seul. Aucun test ne vérifie qu'un instantané publié sans ces champs est relu par l'API (ScopeCoverage aplati) et ressort avec `null`. Un test dans services/api/tests/stats.rs le confirmerait.
- La borne haute de `--rank-max-age-hours` (8761) n'est vérifiée qu'en test unitaire. La CLI n'est testée qu'avec la valeur 0.
- Le desktop ne lit pas encore unknown_rank_rate ni rank_gap_*_hours, et l'utilisateur n'est pas prévenu quand le filtre de rang repose sur une forte part UNKNOWN. L'auteur le signale ; c'est à suivre dans un ticket dédié.
- schema_version reste à 2 alors que le sens du rang change. Le changement de `rank_scope` sert de signal, mais les consommateurs qui ne vérifient que schema_version ne le verront pas. Ce choix est à valider par le produit, tout comme les 168 h par défaut et le départage, à écart égal, en faveur de l'observation la plus ancienne.
- Les chiffres avant/après mesurés sur olc_nuit_a1 (UNKNOWN de 59,17 % à 24,07 %, groupes classés, durée, mémoire) n'apparaissent que dans le résumé de l'auteur. Aucune recette datée ne les conserve dans docs/recettes. Je n'ai pas pu les revérifier : psql n'est pas installé dans l'environnement.

### PR #143 (ticket #81)

Questions ouvertes :
- La correction de la borne Wilson touche aussi la tierlist, qui la publiait déjà avant #81. Je l'ai donc mise sous « Corrigé » dans le CHANGELOG, en plus de la ligne « Ajouté » des builds par étapes. Faut-il plutôt la fondre dans la ligne « Ajouté » ?
- Le contrôle du client desktop (0..=100, page entière rejetée au moindre écart) reste strict. Veux-tu en plus une tolérance côté client, par exemple ramener à 0 une valeur négative infime, pour les instantanés déjà publiés avec une tierlist à 0 victoire ?
- L'agrégation de contrôle a publié un nouvel instantané dans la base de recette olc_nuit_a1 : `aggregate --json` passe par recalculate_filtered, qui publie toujours. C'est sans conséquence si cette base reste jetable.

Réserves de la revue stricte :
- Classement « objet complet » trop large hors Faille : sur le catalogue 16.19.1 de olc_nuit_a1, la règle classe aussi comme complets les enclumes Arena (220001-220006, « Legendary … Item » à 2250), les variantes Arena 22xxxx, 4403 (Golden Spatula) et 226630 (Goredrinker, retiré). Sans effet sur la Faille, car ces objets n'y sont jamais achetés. En revanche, les variantes Arena core/item_slot_N contiendront ces identifiants. La performance Arena reste masquée, donc la conformité tient. À filtrer par carte ou namespace dans un ticket ultérieur.
- Le ticket #81 reste plus large que ce livrable. Ne sont pas traités : la suppression de la table de conversion codée en dur crates/lcu-connector/src/imports/items.rs:94-103 (recommandation du ticket, à remplacer par special_recipe) et l'affichage desktop des étapes (hors périmètre annoncé). Le ticket ne doit pas être fermé par cette PR (pas de « « Closes » du ticket 81 » sans mention du reste).
- Le départ se limite à la fenêtre de 90 000 ms : la timeline ne signale pas la sortie de base, et l'approximation est documentée. Un participant sans achat avant 1 min 30 publie une variante starter vide [], comptée comme un choix observé. Ce cas est cohérent avec la règle des bottes, mais n'est pas signalé explicitement dans le README.
- Catalogue joint = révision la plus récente du patch (16.19.2 avant 16.19.1), pas la version exacte de la partie : c'est documenté. Une partie jouée avant un correctif est donc classée avec le catalogue corrigé.
- Biais de survie et de durée du core et des emplacements 4 à 6 : documenté dans le README et catalogue-jeu.md, mais le winrate de ces étapes reste à présenter avec prudence dans le futur affichage desktop.
- Chiffres de l'agrégation de contrôle (starter 435, boots 428, core 78, slot4 36, slot5 2, slot6 0 variantes à ≥ 100 parties) : rapportés par l'auteur. Je ne les ai pas revérifiés, car relancer l'agrégation publierait un nouvel instantané.

### PR #152 (ticket #111)

Questions ouvertes :
- Valider l'ordre des motifs : `short_game` avant `afk` fait que 35 des 37 parties courtes avec AFK restent comptées `short_game`. Mettre `afk` en premier changerait la lecture des compteurs, pas le total exclu.
- Faut-il garder `early_departure` ? Il ne se déclenche jamais sur la recette (rapport timePlayed/durée minimal de 0,989) ; il est conservé comme garde-fou à la demande du cahier des charges.
- 265 parties de la recette n'ont pas la clé `wasAfk` (anciennes données ?) : elles ne sont pas jugées. Les écarter ou les afficher comme « qualité inconnue » serait un choix produit à confirmer.
- Relancer `olc-collector aggregate --all-stored` en build release sur olc_nuit_a1 pour confirmer les chiffres de bout en bout (non fait ici : trop long en debug).

Réserves de la revue stricte :
- Le livrable ne ferme pas tout le ticket #111. Le contrôle de format des files rares (710, 3130 : uniquement les files listées avec leur format attendu) et la documentation des miroirs (430/480) restent à faire. La liste « hors périmètre » de la DoD cite des sujets d'autres tickets (filtres UI, populations, tiers, builds) mais pas ces deux restes. La PR doit utiliser « Refs #111 », pas « « Closes » du ticket 111 », et un ticket de suivi doit nommer ces deux points.
- Recette : l'agrégation complète par la CLI (`olc-collector aggregate --all-stored`) n'a pas abouti et rien n'a été publié dans olc_nuit_a1. J'ai refait les sondes SQL en lecture seule et elles redonnent exactement les chiffres annoncés : 17112 parties, 37 short_game (dont 35 avec AFK), 602 afk supplémentaires, 637 avec au moins un wasAfk=true, 0 early_departure, 265 sans clé wasAfk. J'ai aussi vérifié qu'aucune partie 420/440 de 300 s ou plus, hors remake, n'a de wasAfk non booléen ni de timePlayed d'un type invalide : 0 et 0, donc aucun invalid_match supplémentaire et les 16473 parties conservées sont cohérentes. Le chiffre publié reste issu d'une sonde et non de la CLI. La lenteur (plus de 580 s en debug) existait déjà et n'est pas causée par ce diff.
- services/api/README.md:89-93 : `meta.exclusions` est un compteur global à l'instantané, comme `omitted_build_variants`. Une réponse filtrée par population ou par périmètre porte donc ces chiffres globaux. Il faut le préciser, comme c'est déjà fait pour `omitted_build_variants`.
- services/collector/README.md:259-262 : l'ordre annoncé (remake, invalid_match, short_game, afk, early_departure) est un peu inexact. Un `invalid_match` dû à un `wasAfk` ou un `timePlayed` d'un mauvais type n'est détecté qu'après `short_game` : une partie courte avec un `timePlayed` invalide est comptée `short_game`. Il faut le préciser, ou bien faire la vérification de type avant `short_game`.
- TDD : le passage rouge puis vert des tests unitaires afk est annoncé mais je ne peux pas le vérifier. L'auteur indique lui-même que les tests d'intégration PostgreSQL et API n'ont pas eu de passage rouge séparé. Les assertions vérifient bien le comportement (compteurs, ordre des motifs, désactivation, snapshot publié, CLI).

### PR #156 (ticket #84)

Questions ouvertes :
- Dénominateur : le ticket recommande games / (population / k), avec k le nombre d'emplacements par partie. J'ai compté directement les parties distinctes (champion présent / parties du compartiment). Avec la formule k, un compartiment de rang (par exemple GOLD TOP avec un seul top GOLD par partie) donnerait 200 %, un champion en double donnerait plus de 100 %, et k est mal défini avec rôles inconnus, bots ou lobbies Arena. Le comptage direct est borné à 100 % et partage la base du ban rate. À confirmer par Matthieu.
- Masquage : le ticket propose de masquer pick_rate seulement si les parties du compartiment sont inférieures à min_games. J'ai conservé le filtre existant (champion.games >= min_games, en plus du dénominateur >= min_games) pour ne pas changer silencieusement la politique de publication. Décision produit : lever ou non le filtre champion pour publier les champions rares.
- Non livré, hors périmètre : présence = pick_rate + ban_rate sur le même périmètre (#109) et affichage de la définition dans l'interface (site #20). Le champ pick_rate_definition est prêt à être affiché.
- Compatibilité : schema_version reste à 2, car l'API le contrôle strictement. Un instantané publié avant ce changement garde l'ancien pick_rate avec l'ancienne définition, et bucket_matches vaut 0. Les consommateurs doivent lire pick_rate_definition. Un recalcul de l'agrégation est nécessaire pour obtenir la nouvelle sémantique ; je ne l'ai pas lancé sur la base de recette.
- Le gh issue view --comments a renvoyé une sortie vide ; j'ai lu le ticket via --json (corps, aucun commentaire). Le ticket cite aussi le cahier §9 et §10.2 ; je n'ai pas modifié le cahier.

Réserves de la revue stricte :
- services/collector/src/aggregation/tests.rs:358-383 (le_denominateur_du_pickrate_est_propre_au_rang_observe) : le test ne distingue pas les cas. Le commentaire de la ligne 365 (« Seule la seconde partie a un top GOLD ») est faux : la partie `a` reçoit aussi `fake-puuid-0` GOLD. Résultat : bucket_matches GOLD = bucket_matches ALL = 2 = population GOLD, et un dénominateur par portée (patch/plateforme/file) ou par population passerait aussi. Correction suggérée : ajouter une troisième partie sans top GOLD, puis vérifier que bucket_matches GOLD (2) diffère de bucket_matches ALL (3) et que le pick rate GOLD se calcule sur 2. Aucun test actuel ne prouve que le dénominateur dépend du rôle et du rang.
- services/collector/src/aggregation/tests.rs:314 (le_pickrate_par_partie_est_comparable_au_ban_rate_dans_un_mode_sans_role) : le nom annonce une comparaison avec le ban rate, mais aucune assertion ne porte sur les bans. Renommer le test ou lui ajouter une assertion sur le ban rate.
- services/collector/README.md:236 et docs/catalogue-jeu.md:167 : « même base que `bans` » / « comme le banrate » en dit trop. Le ban rate se calcule sur les drafts complètes de la portée patch/plateforme/file, sans rôle ni rang. Le pick rate se calcule sur les parties ayant au moins une participation du même rôle et du même rang, sans condition de draft valide. Écrire « même unité (la partie) » pour ne pas engager #109 (présence = pick + ban).
- model.rs:571-574 : le masquage reste sur `games >= min_games`, alors que le ticket recommandait de ne masquer que si le dénominateur (population / k) est sous le seuil. L'auteur l'annonce comme une décision produit ; c'est un écart au ticket à faire valider explicitement par l'humain.
- La formule retenue (parties distinctes du compartiment) diffère de celle du ticket (games / (population / k)). L'écart est documenté et justifié : bornée à 100 %, juste par rang. À valider par l'humain, comme le point précédent.
- Même `schema_version: 2` mais deux sens pour `pick_rate` : un instantané antérieur à #84, relu par l'API, garde l'ancien `pick_rate` avec bucket_matches = 0 et selection_share = null. Seul `pick_rate_definition` distingue les deux. Il n'y a pas encore de consommateur, mais le site (#20) devra lire `pick_rate_definition` (ou tester bucket_matches > 0) avant d'afficher la valeur.
- Cas limite non testé : en mode à doublons (URF), si min_games est supérieur à bucket_matches mais inférieur ou égal à games, `pick_rate` est nul alors que `selection_share` est publié. C'est cohérent avec rate(), mais ce n'est pas documenté.
- La DoD renvoie aux « décisions produit listées en open_questions », que le relecteur n'a pas eues sous les yeux. Seul le résumé de l'auteur les mentionne.

### PR #161 (ticket #109)

Questions ouvertes :
- Médiane à effectif pair : plus bas des deux paliers centraux (retenu) ou plus haut ?
- Le minimum de 6 joueurs connus sur 10 est une constante. Faut-il une option de collecteur comme `--rank-max-age-hours` ?
- Un instantané antérieur à #109 servi avec `rank=GOLD` renvoie maintenant zéro ban (avant, les bans de tout l'échantillon). Comportement acceptable ?
- Faut-il des tickets pour la phase de ban dérivée de `pickTurn`, la présence pick+ban (dénominateur `draft_matches`), l'affichage dans DraftBoard et les suggestions de bans issues des matchups ?
- `UNKNOWN` a deux sens (participant sans rang pour les picks, partie à moins de 6 joueurs connus pour les bans). Faut-il un libellé distinct côté bans (par exemple `UNKNOWN_TIER`) ?

Réserves de la revue stricte :
- TDD côté API : les tests ont été écrits avant l'implémentation mais sans compilation rouge intermédiaire. Compensation : mutation vérifiée en retirant le filtre `b.rank == query.rank`, le test échoue puis repasse au vert une fois la condition restaurée. Pour la médiane, les tests ont été écrits avec l'implémentation.
- Populations différentes : picks au rang de chaque joueur, bans au palier de la partie. Documenté dans le README du collecteur et celui de l'API.
- `rank_for` est exécuté deux fois par participant (médiane, puis boucle principale). Coût négligeable, non refactoré pour garder le diff chirurgical.
- Pas de test automatisé prouvant que `/v1/bans` ne lit aucun morceau de classement ; la logique SQL est simple et la réponse v1/v2 est vérifiée identique.
- Le ticket cite `docs/catalogue-jeu.md:290`, ligne absente de ce worktree (187 lignes) ; la ligne Banrate (168) a été mise à jour.

### PR #171 (ticket #85)

Questions ouvertes :
- Le ticket recommandait des seuils « en écarts-types » ; l'implémentation utilise des seuils absolus en points de winrate (S ≥ +2,5, A ≥ +1, B ≥ −1, C ≥ −2,5). Faut-il garder ces seuils absolus ?
- Les constantes (200 parties fictives, poids de présence 0,02, 0,5 % de pick rate, 20 champions éligibles) sont fixées a priori : faut-il les calibrer sur la base de recette avant publication ?
- Fixture API : la valeur neutre « tier_method fixture » a été choisie pour ne pas dériver de la vraie chaîne. Convient-elle ?

Réserves de la revue stricte :
- Seuils absolus exprimés en points de winrate (S >= 2,5, A >= 1, B >= -1, C >= -2,5, dans services/collector/src/aggregation/tier.rs:13), alors que le ticket #85 recommande des seuils « en écarts-types ». L'auteur le signale dans sa DoD : le porteur doit valider ce choix, de préférence dans un commentaire du ticket.
- Le terme de présence n'est pas borné par rapport au winrate. pick_rate + ban_rate peut approcher 200, et PRESENCE_WEIGHT = 0,02 (tier.rs:7) ajoute alors jusqu'à +4 points : un champion exactement au μ de son compartiment peut atteindre S par la seule présence. Il faut soit plafonner ce terme, soit documenter une calibration sur données réelles avant tout affichage du tier.
- Aucune vérification sur données réelles : personne n'a mesuré sur la copie de recette olc_nuit_a1 combien de compartiments atteignent 20 champions éligibles (pick rate >= 0,5 %). Le nouveau chemin de tier risque de ne rien produire avec le volume actuel. Suggestion : une agrégation sur la recette, avec la distribution des éligibles et des lettres, pour calibrer 200, 0,02 et les seuils.
- services/collector/src/aggregation/model.rs:690 : `bucket_totals[&bucket(&g.key)]` peut paniquer en production. L'invariant tient, puisque la table est construite à partir des mêmes groupes, et le code voisin indexe de la même façon (populations, bucket_matches). Pas bloquant, mais c'est le seul équivalent d'unwrap ajouté sur ce chemin.
- Cas non documenté : tier_eligible (model.rs:712-714) exige pick_rate.is_some(). Or pick_rate vaut None, et non < 0,5, quand bucket_matches < min_games ou quand games < min_games. Le README (services/collector/README.md:240) dit seulement « Nul si pick_rate < 0,5 % ». Ajouter le cas pick_rate nul.
- schema_version reste à 2 alors que la sémantique de tier et de position change. Pour un consommateur d'instantanés stockés, tier_method est le seul moyen de distinguer l'ancien calcul du nouveau. Acceptable puisque la forme est inchangée et qu'aucun écran n'affiche le tier, mais une phrase dans le README le rendrait explicite.
- Pour les rangs de joueur UNKNOWN et UNRANKED, le ban rate vaut 0 (model.rs:670-672) : leurs scores ne sont pas comparables à ceux des paliers classés. C'est documenté dans le README et tier_method, et testé ; à garder en tête le jour où ces compartiments seront affichés.
- Le reformatage rustfmt de l'initialiseur de l'Accumulator (model.rs:341-381) est hors fonctionnalité, mais il est inévitable : rustfmt ignorait le bloc tant que la ligne littérale de tier_method dépassait la largeur maximale. Cosmétique, acceptable.

## Lane D — Files hors classée et catalogue

| Ticket | PR | Lien | Base | Verdict de la revue stricte | Réserves | Questions |
|---|---|---|---|---|---|---|
| #97 | [#132](https://github.com/bolitow/open-lol-companion/pull/132) feat(collector): campagnes par file pour ARAM, Swiftplay et Arena (#97) | Closes | `main` | OK avec réserves (réserves : Arena 1700 contre 1740/1750 non vérifiable hors lig | 8 | 5 |
| #116 | [#135](https://github.com/bolitow/open-lol-companion/pull/135) feat(catalog): objets ARAM et Arena et surcharges de valeurs par mode (#116) | Refs | `feat/97-campagnes-par-file` | OK avec réserves | 7 | 5 |
| #118 | [#141](https://github.com/bolitow/open-lol-companion/pull/141) feat(catalog): catalogue statique des augments Arena et Mayhem (#118) | Refs | `feat/116-catalogue-objets-cartes` | OK avec réserves | 4 | 2 |
| #107 | [#144](https://github.com/bolitow/open-lol-companion/pull/144) feat(catalog): conserver le type de dégâts des infobulles en segments typés (#107) | Refs | `feat/118-augments-catalogue` | OK avec réserves | 12 | 5 |
| #93 | [#146](https://github.com/bolitow/open-lol-companion/pull/146) feat(lcu): lire la version du jeu installée dans le client LoL (#93) | Refs | `feat/107-type-degats-serveur` | OK avec réserves | 9 | 4 |
| #116 (suite) | [#148](https://github.com/bolitow/open-lol-companion/pull/148) fix(catalog): réserves de la revue sur l'export desktop des objets (#116) | Refs | `feat/93-patch-client` | OK avec réserves | 5 | 2 |

### PR #132 (ticket #97)

Questions ouvertes :
- Arena : la campagne vise 1700 par défaut, mais la recette a observé 415 parties en 1740 et 1750 (aucune en 1700), toutes deux absentes du catalogue Data Dragon. Le périmètre compare la file exactement : si le filtre queue=1700 de match-v5 ne renvoie pas ces variantes, rien ne sera retenu. Seul un appel réel tranche. Faut-il passer le défaut à 1700,1740,1750 ?
- Identifier 710 (CLASSIC, 10 participants, partie classée matched) et 3130 (CLASSIC, CUSTOM_GAME) : impossible hors ligne, absentes du catalogue des files. Pour l'instant elles sont exclues des agrégats (`unknown_queue`), ce qui retire 53 parties des prochaines publications. Valider cette exclusion ou fournir leur définition.
- Les seeds restent issus du classement Solo pour ARAM, Swiftplay et Arena : les joueurs jamais classés ne sont pas atteints. Une source de seeds dédiée (autre endpoint ou liste de joueurs) est une décision produit non tranchée ici.
- Cibles par défaut (1000 parties et 20 000 appels par plateforme et par file) à ajuster selon la clé de production : avec la clé de développement (100 requêtes / 2 min) elles sont très au-dessus de ce qu'une nuit permet.
- Les 100 parties minimales par groupe (min_games) restent le seuil : atteindre 1000 parties par file et plateforme ne garantit pas que chaque champion dépasse le seuil. Le ticket #43 (winrate ARAM) peut demander un seuil ou un regroupement différents.

Réserves de la revue stricte :
- services/collector/src/main.rs:491 et services/collector/src/campaign.rs:125 (start_runs) : collect_ranks est décidé pour toute la campagne, pas pour chaque exécution. Avec `--queues 420,450`, l'exécution ARAM porte aussi collect_ranks=true (storage.rs:1019 lit le drapeau par exécution). Elle dépense donc des appels league-v4 pour des parties agrégées en UNRANKED_MODE, ce qui pèse avec une clé limitée à 100 requêtes / 2 min. Correction triviale : dans start_runs, mettre `collect_ranks: template.collect_ranks && [420, 440].contains(&queue_id)` et ajouter un test.
- Pages de seeds redemandées pour chaque couple (plateforme, file) : 3 files sur une plateforme donnent 3 fois les mêmes pages league-v4 sur la même valeur de routage. Ce comportement est hérité des exécutions indépendantes, mais le README (section campagnes par file) ne le dit pas. À documenter, ou à traiter par un passage de seeds partagé dans un ticket de suivi.
- services/collector/src/aggregation/model.rs:618 puis 243 et 624 : depuis le contrôle `unknown_queue`, la branche `gameMode == "CHERRY"` n'agit plus que pour des files déjà listées. Une partie Arena sous un futur identifiant inconnu (par exemple 1760) est donc exclue. L'auteur l'annonce dans Vigilance, mais aucun test ne le fixe : `une_file_inconnue_est_isolee_par_une_exclusion_explicite` ne renseigne jamais gameMode. Ajouter un cas (file 9999 avec gameMode CHERRY, exclusion attendue `unknown_queue`).
- services/collector/src/aggregation/tests.rs:637 : `les_files_identifiees_hors_classe_restent_agregees` teste le prédicat `queues::is_identified`, pas l'agrégation, malgré son nom. Aucun test d'agrégation n'utilise une partie Swiftplay (480). ARAM (450) et Arena (1700, 1740, 1750) sont couverts par les tests existants.
- Arena par défaut sous 1700 (campaign.rs, DEFAULT_QUEUES) : le ticket indique que les 414 parties Arena de la recette étaient toutes en 1740 ou 1750. La valeur par défaut risque donc de ne rien collecter si le filtre `queue` de match-v5 compare exactement. Le point est signalé par l'auteur et conforme au périmètre de la lane (Arena 1700). À vérifier par un appel réel unique avant toute campagne de volume.
- Ticket #97 : la recommandation demande d'« identifier 710 et 3130 » et d'utiliser le catalogue des files. Le livrable les isole sous `unknown_queue` et s'appuie sur une liste codée en dur (queues.rs), choix justifié parce que Data Dragon ignore 1740 et 1750. L'auteur signale les deux points ; l'identification reste à mener hors de cette lane. ARAM Mayhem (2400, §10.1 du cahier des charges) n'est pas dans les files par défaut mais reste acceptée par `--queues`.
- Message d'erreur peu précis : campaign.rs:77 renvoie « file inconnue ou nulle » sans citer l'identifiant fautif. Vérifié en CLI : `campaign-queues --queues 450,710` répond « paramètre de campagne invalide : file inconnue ou nulle ». Citer la file aiderait l'utilisateur. Par ailleurs, la sortie texte de `campaign-report` n'affiche pas le status_reason de chaque exécution (présent seulement en --json).
- TDD non vérifiable a posteriori : la phase rouge annoncée (stub is_identified, échec de compilation de start_per_queue et coverage) ne peut pas être rejouée. Les trois tests unitaires de campaign.rs ont été écrits avec le code, comme l'auteur le déclare.

### PR #135 (ticket #116)

Questions ouvertes :
- Régénérer et republier apps/desktop/public/game-data/catalog avec le nouvel export (objets ARAM/Arena, effets par mode) ? Opération réseau (~1,5 k icônes), diff volumineux, métadonnées observed_at non reproductibles et autoImport.test.ts importe en_US.json versionné : à décider et valider par Matthieu.
- Le front (`searchableItems` dans apps/desktop/src/app/preparation.ts) filtre sur maps['11'] : un filtre sensible au mode sera nécessaire à l'ouverture d'ARAM/Arena (#43, #44). Hors de ce lot.
- Les clés de mode de la source (ARAM, cherry, URF, SWIFTPLAY, NEXUSBLITZ, hachages comme {bffdf499}) ne sont pas mappées à nos files : `cherry` ressemble à Arena et un hachage à Mayhem mais cela n'est pas vérifiable ; faut-il établir et documenter une correspondance une fois vérifiée ?
- Contexte d'équilibrage ARAM par champion (dégâts infligés/subis) : toujours aucune source vérifiée, non livré.
- Refus d'import d'une variante contenant un identifiant absent du catalogue (docs/imports-client.md) non traité : relève du moteur d'import, pas du catalogue.

Réserves de la revue stricte :
- Catalogue publié non régénéré, alors que le périmètre de ce livrable inclut « exposition dans le catalogue publié ». L'auteur le signale (README, résumé, Vigilance), donc ce n'est pas bloquant, mais l'impact visible du ticket (objet inconnu à l'affichage et à l'import en ARAM ou Arena) reste entier tant que la republication n'a pas eu lieu. Deux conséquences : le fichier versionné apps/desktop/public/game-data/catalog/manifest.json:2436-2439 et 2453-2456 porte encore `mode: "map_11_with_components"` sans `maps` ni `by_map`, ce qui ne respecte plus le type `DesktopCatalogItemFilter` ajouté dans packages/shared/src/catalog.ts:69-74 (aucun code ne le lit aujourd'hui) ; et les décomptes du README (458 fiches, 1 477 PNG) devront être mis à jour à la republication. Prévoir un ticket ou une étape de publication validée.
- Les tests de l'export ne tournent ni avec `pnpm test` ni en CI : services/collector/Cargo.toml ne déclare pas `[[example]] name = "export_desktop_catalog" test = true`. Le journal de `pnpm test` ne contient aucune ligne « Running unittests examples/export_desktop_catalog.rs ». Les 3 nouveaux tests (garde_les_objets_propres_a_laram..., le_decompte_par_carte..., une_carte_exportee_absente...) et les 2 assertions renommées ne passent qu'avec `cargo test -p olc-collector --example export_desktop_catalog` (13 passed, relancé par moi). Le problème est antérieur au diff, mais la preuve TDD de l'export échappe à la commande de la DoD. Suite à donner : déclarer l'exemple avec test = true.
- Le repli est très large (services/collector/examples/export_desktop_catalog.rs:113-121). Il suffit qu'un seul objet n'ait pas l'une des clés 11, 12 ou 30, par exemple si Data Dragon retire la carte 30 quand l'Arena sort de rotation, pour que tout l'export passe en `all_items` (870 objets et toutes leurs icônes). Le choix est délibéré et documenté, mais il pèse en exploitation. Pistes : ne prendre en compte que les cartes présentes dans l'ensemble des données, ou signaler la carte manquante dans `reason`.
- Les clés de mode hachées sont majoritaires, pas l'exception. Sur items.cdtb.bin.json réel (75 objets avec DataValuesModeOverride), on compte `{bffdf499}` 38 fois, `{a110bc47}` 16 fois et `{ad33a648}` 5 fois, contre 36 pour ARAM et 4 pour cherry. Environ la moitié des effets de mode auront donc un identifiant opaque et un `unresolved_mode_key:`, et `records_with_issues` augmentera d'autant. docs/catalogue-jeu.md:61-66 et CHANGELOG.md:9 présentent le hachage comme un cas marginal : à préciser. À noter aussi : la clé Arena dans la source est `cherry`, pas « Arena ».
- Code dupliqué : `mode_overrides` (services/collector/src/catalog/community.rs:461-540) recopie environ 40 lignes de la boucle `mDataValues` de `bin_item` (community.rs:~686-731) : calcul du statut, ajout des chemins mappés, `merge` et signalement des conflits. Extraire une fonction commune `data_values(...)` éviterait que les deux copies divergent.
- Mise en forme du CHANGELOG : sous `### Modifié` (CHANGELOG.md:88-89), la nouvelle puce est suivie d'une ligne vide qui coupe la liste avant la puce #97. Supprimer la ligne vide.
- Typage peu contraignant : `maps: [&'static str; 3]` côté Rust est traduit en `CatalogItemMapId[]` et non en tuple, et `CatalogModeEffectId` reste indicatif puisque `CatalogEffect.id` est toujours `string` et qu'aucun code du front ne lit les effets. Rien de faux, mais le type ne protège pas grand-chose.

### PR #141 (ticket #118)

Questions ouvertes :
- Le jeton `%i:…%` est conservé et signalé plutôt que retiré : faut-il plutôt le retirer du texte publié lors d'une itération ultérieure, quand l'interface affichera les augments ?
- Le fichier `cdragon/arena/{fr_fr,en_us}.json` est hors du schéma `plugins/rcp-be-lol-game-data` : on accepte une source dont le format n'est pas garanti par le jeu (épinglée au patch et à l'URL) ?

Réserves de la revue stricte :
- Le téléchargement des augments est couplé au reste du complément. `services/collector/src/catalog/community_http.rs:226` ajoute les cinq ressources d'augments au même passage `fetch_sources_with`, et `fetch_json` transforme tout statut autre que 200 en `CatalogError::Network`. Dans `services/collector/src/catalog/pipeline.rs:142-147`, un seul 404 sur `cdragon/arena/*.json` (export généré par CommunityDragon, dont la stabilité n'est pas garantie) a donc deux effets : en `--community optional`, tout le complément saute (objets #116, perks…) et la publication sort dégradée avec `community_unavailable` ; en `required`, le catalogue est refusé. Un changement de format donne `InvalidSource`, qui refuse la publication même en `optional`. La doc décrit le « tout ou rien » des augments et pose la question ouverte sur la stabilité de `/cdragon/`, mais ne dit pas qu'une indisponibilité des augments casse aussi l'enrichissement existant. À trancher : rendre les augments dégradables comme un groupe séparé, ou au moins documenter cette conséquence dans la question ouverte de `docs/catalogue-jeu.md`.
- `services/collector/src/catalog/community_augments.rs:132-133, 141-142, 159, 169, 173-174` : l'indexation `sources[...]` d'une `BTreeMap` panique si une clé manque. Elle n'est pas atteignable aujourd'hui, car `validate_sources` (`community.rs:53-75`) exige les cinq clés avant `augments::validate` et `enrich`. `perks.rs` utilise le même motif. La garantie vient donc de l'ordre des appels, pas du type : un futur appel direct à `augments::enrich` sans validation paniquerait. On pourrait préférer `.get(..).ok_or(CatalogError::InvalidSource)` ou un commentaire d'invariant.
- Détail de test : `apps/desktop/src/app/catalog.test.ts:25` utilise une icône `/game-data/catalog/icons/augment/1205.png`, alors que l'export réel produit `icons/{sha256}.png` (`export_desktop_catalog.rs:74-79`). C'est sans effet sur le comportement testé, mais la fixture ne reflète pas la forme réelle.
- Aucun test croisé ne vérifie que l'ensemble des champs produits par le Rust correspond exactement à `CATALOG_AUGMENT_FIELDS` (`packages/shared/src/catalog.ts`). J'ai vérifié à la main : les 8 champs correspondent. Une évolution future pourrait toutefois désaligner les deux sans qu'un test échoue.

### PR #144 (ticket #107)

Questions ouvertes :
- Faut-il aussi typer les autres balises de Data Dragon (scaleAP, scaleAD, scaleArmor, scaleMR, healing, shield, status, speed...) ? Ce lot ne type que les trois types de dégâts ; les ratios PV/armure/RM de la recommandation du ticket restent à décider et à spécifier.
- L'interprétation des effets du BIN côté serveur (étendre les statistiques reconnues au-delà de ability_power, attack_damage, bonus_attack_damage) n'est pas faite : hors périmètre de ce livrable, à traiter dans un lot séparé avec Louison pour la suppression de la regex abilityPresentation.ts (ce fichier n'existe pas dans ce worktree).
- Statut du champ : derived retenu (transformation déterministe d'une balise) ; à valider si la revue préfère descriptive comme pour tooltip.
- Faut-il aussi segmenter description (texte court des compétences) et les infobulles de CommunityDragon (augments, runes) ? Non fait : Data Dragon tooltip seulement.
- Une nouvelle publication du catalogue (catalog --refresh ou --rebuild) et la régénération de apps/desktop/public/game-data/catalog sont nécessaires pour que le champ atteigne l'API et l'export desktop ; l'export passe les champs tels quels (vérifié par lecture, pas exécuté).

Réserves de la revue stricte :
- Sorts d'invocateur : services/collector/src/catalog/normalize.rs:332 (common_spell) ajoute aussi `tooltip_segments` aux fiches `summoner_spell`. C'est le même chemin de code, la fonction est testée unitairement et l'auteur l'a signalé, mais aucun test de normalize ne le couvre. Il faut ajouter un test de normalisation sur une source summoner.json dont le `tooltip` est balisé (Embrasement : <trueDamage>).
- CHANGELOG.md:9 ne parle que de « l'infobulle des compétences », alors que les sorts d'invocateur reçoivent aussi le champ (docs/catalogue-jeu.md et catalog.ts le disent). Il faut compléter la ligne.
- services/collector/src/catalog/project_value.rs:214 : `json!(segments)` sur une expression non littérale se déplie en `serde_json::to_value(&segments).unwrap()`. La panique est inatteignable (Vec<struct{String, Option<enum>}>), donc ce n'est pas bloquant. Pour respecter la règle « pas d'unwrap hors tests », préférer un `serde_json::to_value` explicite, avec gestion de l'erreur ou commentaire.
- packages/shared/src/catalog.test.ts:111 : le transtypage `segments as unknown as JsonValue` montre que `CatalogTooltipSegment` (une interface, sans signature d'index) ne s'assigne pas à `JsonValue`. Le miroir est exact champ par champ mais pas intégré structurellement à `CatalogValue.value` ; un alias `type` éviterait le transtypage. De plus, le test « miroir du JSON Rust » construit sa propre fixture au lieu de consommer une sortie Rust : l'alignement tient à l'inspection, comme pour le test des augments existant.
- La première espace à une frontière de balise prend le type en vigueur au dernier blanc rencontré : dans `a <magicDamage> b`, l'espace de séparation est typée `magic`. L'invariant texte tient et le commentaire l'explique, mais la doc (docs/catalogue-jeu.md) ne le précise pas pour le desktop.
- Une fermeture qui ne correspond pas (`<magicDamage>x</physicalDamage>`) laisse le type courir jusqu'à la fin de l'infobulle, et aucun test ne fixe ce cas. Sans impact réel aujourd'hui : relevé sur les 726 infobulles fr_FR 16.19.1 (championFull et summoner), 0 balise de dégâts non fermée, orpheline ou imbriquée.
- apps/desktop/public/game-data/README.md:28-29 liste `fields.tooltip` par famille. Il faudra y ajouter `tooltip_segments` à la prochaine régénération des assets (régénération hors périmètre, signalée).
- docs/catalogue-jeu.md:155 : la ligne `derived` du tableau des statuts donne ses exemples (icône, arbre, ID/slot) sans le nouvel usage (segments typés). Ajout d'une ligne facultatif.
- NORMALIZER_VERSION passe de 2 à 3 (model.rs:8) : d'autres lanes peuvent l'incrémenter en parallèle, il faudra renuméroter et coordonner à la fusion.
- Poids : le texte de chaque infobulle est stocké deux fois (`tooltip` et `tooltip_segments`), ce qui alourdit le catalogue publié et les futurs assets desktop. À mesurer à la prochaine publication.
- Les tags encodés en entités (`&lt;magicDamage&gt;`) deviennent de vraies balises de type, parce que le décodage passe avant l'analyse. C'est le comportement hérité de plain_text, sans risque puisque la sortie n'est que du texte.
- Le rouge TDD est annoncé (échec de compilation côté Rust, export absent côté vitest) mais ne peut pas être revérifié après coup.

### PR #146 (ticket #93)

Questions ouvertes :
- Format réel de /lol-patch/v1/game-version : le schéma dit seulement « string » et aucun client n'était lancé sur ce Mac cette nuit. Il faut le relire sur un client Windows et macOS pour confirmer la forme majeur.mineur.build.révision.
- La version doit-elle plus tard faire partie de LcuSession et être mise à jour par WebSocket (lue à la connexion puis à chaque mise à jour), plutôt que d'être lue par l'interface au démarrage et à chaque sélection ?
- Libellés FR/EN des erreurs unavailable et invalid_response : à ajouter côté interface (Louison), probablement sur le modèle de shared/imports.ts. Aucun libellé n'a été écrit d'avance ici.
- Recoupement avec GET /v1/static/manifest de l'API et correspondance avec le libellé public (26.19 pour 16.19) : non traités, ils dépendent d'une décision produit sur la source de vérité et sur le repli affiché.

Réserves de la revue stricte :
- Format réel de la chaîne non vérifié : le schéma ne dit rien au-delà de « string ». Il faut relire la réponse de `/lol-patch/v1/game-version` sur un client réel, sous Windows et sous macOS, avant que l'interface ne s'appuie sur `patch`. Le parseur exige « majeur.mineur » entiers (crates/lcu-connector/src/patch.rs:31-50) et peut répondre `invalid_response` si le client renvoie une autre forme.
- Ticket #93 : la PR ne doit pas fermer le ticket (pas de « « Closes » du ticket 93 »). La liste « hors périmètre » de l'auteur oublie plusieurs points de la recommandation : lecture au démarrage et à chaque sélection, recoupement avec `live_version` par plateforme, régénération atomique du catalogue, des effets, des icônes et des séries de skins, abandon de l'alias CommunityDragon `latest`, et test qui échoue si un artefact embarqué diffère de la version du catalogue. Il faut les ajouter au reste à faire de la PR ou du ticket.
- apps/desktop/src-tauri/src/lib.rs:63 : `client_patch` relance la découverte (lockfile puis recherche du processus, dans spawn_blocking) à chaque appel, sans cache. Il faut écrire dans docs/patch-client.md que l'interface ne doit pas l'appeler en boucle. Si l'appel devient fréquent, prévoir une mise en cache dans la session.
- Indexation de la doc : docs/patch-client.md n'est lié que depuis packages/shared/README.md. Les contrats voisins (imports-client.md, amis-client.md) sont aussi référencés depuis docs/DEMARRAGE.md, docs/cahier-des-charges.md ou docs/compte-actif.md. Ajouter un lien pour que le contrat soit trouvable par le développeur de l'interface.
- crates/lcu-connector/src/patch.rs, test serialise_le_contrat_public_en_camel_case : seule la sérialisation de `InvalidResponse` est vérifiée. Ajouter `Unavailable` → "unavailable" pour couvrir tout le contrat miroir de @olc/shared.
- Zéros en tête : "016.019.x" donne `patch` = "16.19" pendant que `gameVersion` reste brut. Ce comportement n'est ni testé ni documenté. Soit le tester, soit le refuser explicitement.
- Choix discutable mais documenté : une réponse 200 dont le corps n'est pas du JSON donne `unavailable` et non `invalid_response`, parce que l'erreur de décodage de reqwest est confondue avec une erreur de transport (patch.rs:57-63).
- La source est un dump communautaire du schéma LCU (KebsCS), pas une documentation Riot. Je l'ai vérifiée moi-même le 4 octobre 2026 : l'opération GetLolPatchV1GameVersion existe et répond 200 avec un schéma `{type: string}`. L'API LCU est tolérée mais non supportée (cahier §2) et peut changer à chaque patch.
- La commande Tauri n'a pas de test direct : le passage d'un échec de découverte à `Unavailable` n'est pas couvert. C'est la même pratique que pour les commandes voisines, et le câblage est couvert par le test de permissions (relancé : 1/1). Non bloquant.

### PR #148 (ticket #116)

Questions ouvertes :
- Le seuil du repli all_items reste « un seul objet illisible déclenche le repli » (comportement inchangé) : faut-il le restreindre à une part significative d'objets, ou la journalisation par carte suffit-elle ? À valider par Matthieu.
- Le manifeste du catalogue publié contiendra unreadable_by_map seulement après la prochaine régénération (hors périmètre ici).

Réserves de la revue stricte :
- services/collector/examples/export_desktop_catalog.rs:116-131 : la branche « champ maps présent mais statut ni Verified ni Derived » a été réécrite (`.filter(...)` puis `contains_key` pour choisir `uncertain_map`), mais aucun test ne la couvre. Le nouveau test passe seulement par l'objet sans clé `maps` (400) et les clés absentes ou non booléennes (200, 300), et n'assert jamais `result.reason`. Il manque un cas avec `status = ValueStatus::Conflict` sur `maps`, qui vérifie `reason == Some("uncertain_map")` et le décompte sur les trois cartes. Il faut aussi asserter `reason == Some("unknown_map")` dans le_repli_all_items_denombre_par_carte_les_objets_a_disponibilite_illisible.
- services/collector/examples/export_desktop_catalog.rs:48 : rien ne teste côté Rust le contrat de sérialisation de `unreadable_by_map` (`skip_serializing_if = "BTreeMap::is_empty"`). Le commentaire TS (packages/shared/src/catalog.ts:74-78) promet un champ « omis (jamais `{}` ni `null`) », mais le test TS ne vérifie qu'un objet écrit à la main. À ajouter : un test Rust qui fait `serde_json::to_value(&select_items(..))` et vérifie que la clé est absente en filtre normal et présente en repli.
- apps/desktop/public/game-data/README.md:15 : ce contrat consommateur de `item_filter` cite toujours seulement `mode`, `maps` et `by_map`. `reason` et le nouveau champ optionnel `unreadable_by_map` (présent seulement en repli `all_items`) n'y sont pas documentés. À compléter d'une phrase.
- Point (3) du périmètre : l'auteur a choisi l'option minimale (« au moins le journaliser par carte »). Le seuil de déclenchement reste inchangé : un seul objet illisible suffit encore à basculer tout l'export en `all_items`. C'est conforme au libellé, mais ça laisse ouverte la restriction du repli à une part significative des objets. À tracer dans le ticket ou la PR comme suite possible.
- community.rs : la factorisation `data_values` ne change pas le comportement (`paths` passe de Vec à BTreeSet mais finit fusionné dans `mapped`, déjà un BTreeSet, et la sémantique transactionnelle de `mode_overrides` est conservée). En revanche, le conflit sur `mDataValues` de base (`conflict:effect_parameter.{name}` sans mode) ne semble couvert par aucun test existant, contrairement à la variante par mode (community_tests.rs:502).

## Lane E — Plateforme : quota, WebSocket, API profils, CI, RGPD, jeton

| Ticket | PR | Lien | Base | Verdict de la revue stricte | Réserves | Questions |
|---|---|---|---|---|---|---|
| #122 | [#133](https://github.com/bolitow/open-lol-companion/pull/133) feat(api): réserver une part du quota Riot partagé aux requêtes interactives (#122) | Refs | `main` | OK avec réserves (riot_busy sans libellé desktop, test d'intégration sensible au | 7 | 5 |
| #125 | [#139](https://github.com/bolitow/open-lol-companion/pull/139) feat(desktop): client WebSocket des publications de données (#125) | Refs | `feat/122-priorite-quota` | OK avec réserves | 8 | 2 |
| #96 | [#140](https://github.com/bolitow/open-lol-companion/pull/140) feat(api): exposer victoires, défaites et drapeaux league-v4 dans le rang du profil (#96) | Refs | `feat/125-ws-client-desktop` | OK avec réserves | 10 | 4 |
| #121 | [#142](https://github.com/bolitow/open-lol-companion/pull/142) ci(patch): lancer les tests quand Data Dragon publie un nouveau patch LoL (#121) | Refs | `feat/96-profil-rang-api` | OK avec réserves | 7 | 6 |
| #99 | [#147](https://github.com/bolitow/open-lol-companion/pull/147) feat(rgpd): rétention, export et effacement des données personnelles (#99) | Refs | `ci/121-tests-par-patch` | OK avec réserves | 8 | 8 |
| #98 | [#150](https://github.com/bolitow/open-lol-companion/pull/150) feat(desktop): saisir l'accès API dans les réglages et garder le jeton dans le trousseau (#98) | Refs | `feat/99-rgpd-retention` | OK avec réserves | 12 | 5 |
| #90 (suite) | [#153](https://github.com/bolitow/open-lol-companion/pull/153) feat(collector): mieux employer le budget d'appels Riot après le filtre des rangs (#90) | Refs | `feat/98-jeton-trousseau` | OK avec réserves | 10 | 6 |
| #122 (suite) | [#157](https://github.com/bolitow/open-lol-companion/pull/157) fix(quota): décrire la réserve réelle et fiabiliser les tests du quota partagé (#122) | Refs | `feat/90-budget-rangs-suite` | OK avec réserves | 5 | 1 |

### PR #133 (ticket #122)

Questions ouvertes :
- Valider la part réservée de 20 % (4 appels/s et 20 par 2 min pour l'interactif). Une recherche de profil avec vingt parties demande environ 24 appels, donc dépasse la réserve sur 2 min : elle étale alors ses appels sur la fenêtre ou obtient riot_busy après 20 s. Faut-il une réserve plus grande, ou un cache plus agressif côté API ?
- Affichage « Riot occupé » dans le desktop : riot_busy est dans ApiErrorCode, mais PlayerError, knownErrors (playerStore.ts) et playersCopy.ts FR/EN ne le connaissent pas, donc l'interface le replie sur « unavailable ». À traiter dans la lane desktop ou un ticket dédié.
- La réserve s'applique aussi aux fenêtres par méthode apprises via les en-têtes Riot (le collecteur perd 20 % de chaque limite de méthode). Confirmer ce choix ou la limiter à la fenêtre applicative.
- Contention du verrou de ligne FOR UPDATE : non mesurée ici (la recommandation du ticket demande de mesurer avant d'optimiser). Une mesure sur la base de recette en lecture seule peut être planifiée séparément.
- Le câblage Interactive de main.rs de l'API n'a pas de test automatisé (la priorité est privée au transport). Faut-il l'exposer pour un test de câblage ?

Réserves de la revue stricte :
- CHANGELOG.md:9 et services/api/README.md:124-131 promettent trop sur la capacité. Le texte affirme qu'une recherche de profil « ne dorme plus » pendant une rafale. Or la réserve interactive est de 20 appels par 2 min sur la route europe (clé de développement 100/2 min). Un profil plus la première page d'historique coûte déjà environ 12 appels sur cette route (account + ids + 10 détails). Une seconde page ou un second utilisateur dans la même fenêtre de 2 min attend encore et finit en riot_busy. Reformuler en « limite l'attente », citer ce plafond, et envisager une ligne « Modifié » : le débit du collecteur baisse de 20 %.
- services/api/src/profiles.rs:104-106 : riot_busy signifie seulement « délai de 20 s dépassé », pas « attente de quota ». Le timeout englobe le gouverneur local, la boucle de réservation SQL (verrou FOR UPDATE), l'appel HTTPS (jusqu'à 15 s) et l'écriture observe() en base. Une réponse Riot lente mais réussie, ou une base bloquée, remonte donc aussi en riot_busy. L'auteur l'a signalé ; à préciser dans la doc ou dans un ticket de suivi.
- services/api/README.md:141-142 décrit rate_limited de façon incomplète (seulement « plus de 4 appels Riot en cours sur l'instance »). Ce code est aussi renvoyé par server.rs:118 (32 emplacements HTTP pleins), realtime.rs:60 (emplacements WebSocket pleins) et profiles.rs:107 (429 de Riot propagé). Corriger la description.
- services/collector/tests/shared_quota.rs:145-178 : le test d'intégration dépend du temps. Il se fixe 1 s pour atteindre 8 envois, exige que l'appel interactif passe en moins de 700 ms, et ne laisse que 10 ms de marge (1490 ms au lieu de 1500) entre l'horloge Rust et l'horloge PostgreSQL. Risque de test instable sur une CI chargée. Signalé par l'auteur.
- services/api/src/main.rs:110 : le câblage Priority::Interactive n'est couvert par aucun test, et CoordinatedTransport::new choisit Background par défaut. Si .with_priority(Interactive) disparaît, le code compile et l'API se dégrade sans bruit. Suggestion : passer la priorité en argument du constructeur, ou ajouter un test de démarrage. Le mécanisme lui-même est couvert par le test d'intégration.
- crates/build-client/src/profiles.rs:273 : le desktop transforme tout 503 en PlayerError::Unavailable. riot_busy s'affiche donc encore « Le service de profils ne répond pas », exactement la mauvaise attribution que le ticket dénonce. L'affichage « Riot occupé » sort du périmètre de cette lane et figure dans « Non fait » : ouvrir un ticket de suivi FR/EN.
- Contention du verrou de ligne (SELECT … FOR UPDATE, une ligne par hôte) non mesurée, alors que le ticket le recommande. Signalé par l'auteur ; à reporter dans un ticket.

### PR #139 (ticket #125)

Questions ouvertes :
- La branche 401/403 est conservée pour un éventuel proxy d'authentification devant l'API (option a). Faut-il plutôt la supprimer (option b) si aucun proxy n'est prévu en production ?
- Faut-il, dans un ticket distinct, faire renvoyer 401 au handshake par services/api/src/realtime.rs ? Ce serait un changement de protocole côté serveur, hors périmètre de ce livrable.

Réserves de la revue stricte :
- Périmètre « invalidation du cache de builds » : le cœur Rust ne garde aucun cache de builds. Le cache de imports/guard.rs ne conserve que des reçus d'import, pas de contenu de build. Le seul cache réel est l'état React de apps/desktop/src/app/useBuilds.ts, qui ne relit les builds que si la clé de requête change. Le livrable fournit donc le signal (`revision`) sans l'invalidation effective, ce qui est conforme au périmètre (aucun changement React) et signalé dans la Vigilance. #125 (§10.2, mise à jour automatique) ne doit pas être fermé sur ce diff : il faut un ticket de suite pour brancher `PUBLICATION_STATE_EVENT` dans useBuilds et AutoImportPanel (relecture quand `revision` change).
- crates/build-client/src/publications.rs:216 (boucle watch_publications) : le repli n'a pas d'aléa (jitter). Quand le serveur redémarre, il ferme toutes les sessions en 1001 et tous les desktops dont la session a duré plus de 60 s se reconnectent ensemble après 1 s. Le plafond de 128 sockets et la reprise sur 429 limitent l'effet, mais un aléa de ±20 % est conseillé.
- La branche wss n'est jamais exercée de bout en bout : tous les tests passent par ws://127.0.0.1. Le Connector::Rustls construit avec le ClientConfig stocké (sans ALPN ajouté par reqwest) n'a pas été vérifié face à un vrai serveur TLS. Recette manuelle à faire sur une API HTTPS, sous Windows et sous macOS.
- Le serveur ferme en 1008 "unauthorized" aussi quand le premier message n'arrive pas dans les 5 s (services/api/src/realtime.rs:92-101). Sur un réseau très lent, le desktop s'arrête alors définitivement (Unauthorized) au lieu de retenter. Cas marginal, à documenter ou à distinguer par la raison de fermeture.
- apps/desktop/src-tauri/src/publications.rs (test) : `include_str!` cherche `'publication-state'` entre apostrophes. Le test casserait si le fichier TS passait aux guillemets doubles, même sans vraie divergence de nom.
- packages/shared/src/shared.test.ts : le test publications ne vérifie que l'ordre de `Object.keys` d'un littéral, ce qui est presque tautologique. Le vrai garde-fou d'alignement est le test serde côté Rust, qui ne couvre que la variante `connected` de PublicationStatus. Les autres variantes ont été vérifiées à la lecture et sont alignées.
- La glue Tauri (`apply`, `setup`, `emit_to("main")`) n'a aucun test de comportement. Le label "main" correspond bien à tauri.conf.json. Une config invalide (InvalidConfiguration) laisse l'état à `not_configured`, ce qui est un peu trompeur.
- docs/DEMARRAGE.md (paragraphe #125) : retours à la ligne irréguliers, à reformater. La case DoD « aucune modification de services/api » est inexacte au sens strict : services/api/README.md est modifié, mais il s'agit de doc seulement.

### PR #140 (ticket #96)

Questions ouvertes :
- La commande Tauri (apps/desktop/src-tauri/src/players.rs) mappe champ par champ et ne relaie pas encore wins/losses/drapeaux vers l'interface : à faire avec la partie LCU/desktop (Louison), en réutilisant le JSON de référence pour garder l'alignement.
- Les copies de ProfileRank dans crates/lcu-connector et apps/desktop/src-tauri n'ont pas été modifiées (hors périmètre) : veux-tu les aligner sur le même contrat (provisional, season_peak, previous_season_end sont prévus par le ticket) dans la partie desktop ?
- Les noms league-v4 (wins, losses, hotStreak, veteran, freshBlood, inactive) viennent du schéma LeagueEntryDTO connu et du ticket ; je n'ai pas fait d'appel Riot réel pour les revérifier : à confirmer lors d'une recette avec la clé.
- Après déploiement, le cache de profils (5 min) peut servir brièvement des rangs sans les nouveaux champs : acceptable ?

Réserves de la revue stricte :
- apps/desktop/src-tauri/src/players.rs:95-101 : public_profile copie les rangs champ par champ vers lcu_connector::ProfileRank (crates/lcu-connector/src/players.rs:22-28), qui n'a aucun des six nouveaux champs. Tout ce que l'API renvoie désormais est donc perdu avant d'arriver à l'interface. L'impact du ticket (winrate saisonnier officiel) n'est pas encore livré de bout en bout. C'est bien signalé comme partie desktop (Louison) et hors périmètre, mais le ticket #96 ne doit pas être fermé sur ce livrable.
- services/collector/src/storage.rs:771-790 : store_participant_ranks n'enregistre toujours que tier, division, LP et statut. Le collecteur lit maintenant wins/losses et les drapeaux, puis les jette. C'est cohérent avec « partie API seulement », mais ni le README ni le CHANGELOG ne précisent que ces champs ne sont pas conservés en base.
- packages/shared/src/api.ts:159-166 : côté TS les champs sont déclarés `?: T | null`, alors que l'API Rust envoie toujours `null`. Le miroir est donc compatible, pas strictement identique. Le choix se justifie parce que PlayerProfile = Omit<Profile,'puuid'> & {source} (packages/shared/src/players.ts:9) sert aussi au chemin LCU, qui n'envoie pas encore ces clés. Accepté, à resserrer en `T | null` une fois la partie LCU/Tauri faite.
- packages/shared/src/profileContract.test.ts:37-40 : `golden as Profile` ne prouve rien, puisqu'un cast `as` passe même sans vraie compatibilité. Le vrai garde-fou est la comparaison des clés avec Record<keyof …, true>. Ce second test n'apporte rien : il faudrait le renommer ou le supprimer.
- crates/build-client/tests/profiles.rs:42-57 : lit_victoires_defaites_et_drapeaux_envoyes_par_api ne vérifie ni fresh_blood ni inactive. L'aller-retour du JSON de référence couvre ces deux champs, mais le test HTTP gagnerait à vérifier les six.
- services/api/README.md:111 : la phrase « Ils ne sont disponibles que pour le compte actif, depuis le client LoL (partie desktop) » peut laisser croire que le peak et la saison précédente sont déjà fournis côté desktop, alors que lcu-connector ne les lit pas encore. Préciser « prévu, partie desktop à faire ».
- CHANGELOG.md:9 : une seule ligne très longue qui mélange l'effet visible pour un joueur (W/L, drapeaux) et un détail pour contributeurs (JSON de référence, README). rules/changelog.md demande une ligne par changement. Point cosmétique.
- Vigilance de l'auteur inexacte : le cache de profils (services/api/src/profiles.rs:78, 231-236) est en mémoire, il est donc vidé à chaque redéploiement et ne peut pas servir un ancien profil sans les nouveaux champs. Le seul vrai décalage possible est un client desktop plus ancien que l'API, déjà géré puisque les Option absents deviennent None.
- Cahier des charges §8.1 et recommandation du ticket : exposer `inactive` est conforme tant que c'est une donnée brute. Le jour où une interface ou des étiquettes s'en serviront, il faudra éviter toute étiquette négative publique sur un tiers. Rien n'est affiché dans ce diff.
- Endpoint league-v4 non revérifié en direct (aucune collecte Riot lancée, comme annoncé). Les noms hotStreak, veteran, freshBlood, inactive, wins et losses correspondent au contrat LeagueEntryDTO connu.

### PR #142 (ticket #121)

Questions ouvertes :
- Valider qu'une issue automatique par patch (fermée si les tests passent) est acceptable comme mémoire du dernier patch contrôlé, plutôt qu'un cache ou une variable de dépôt.
- Contrôles propres au nouveau patch non livrés : exhaustivité du catalogue exporté, import runes/sorts/items pour tous les champions du patch sur données générées, validation du format match-v5 sur un échantillon figé. À faire dans un ticket dédié (ne pas fermer #121 avec cette PR).
- Les scripts de mise à jour du catalogue de skins cités par le ticket (scripts/update-skin-lines.py) n'existent pas dans cette base : « mettre à jour le patch par défaut à partir de la détection » reste à traiter ailleurs.
- Activer plus tard des runners Windows/macOS sur nouveau patch ? Laissé volontairement hors périmètre (coût) ; les recettes LCU réelles restent manuelles.
- Les étapes de test sont dupliquées entre ci.yml et patch-watch.yml. Option ultérieure : rendre ci.yml appelable (workflow_call) une fois le comportement de dorny/paths-filter sur un événement schedule vérifié.
- Premier déclenchement réel à constater après fusion sur main (les workflows planifiés ne tournent que sur la branche par défaut) : lancer « Run workflow » avec force pour vérifier la création de l'issue et le jq env.TITLE de gh.

Réserves de la revue stricte :
- Valeur de détection limitée (signalée par l'auteur et dans le CHANGELOG) : patch-watch.yml rejoue sur un code et un Cargo.lock inchangés des tests qui tournent tous sur données figées (les tests qui lisent Data Dragon passent par des données de test). Un nouveau patch ne peut donc pas faire échouer ce job, sauf incident du runner ou dérive d'une action ou d'une image. Le critère §14 « non-régression sur le dernier patch » n'est pas encore vérifié réellement. Il faut un ticket de suite, à lier dans la PR, pour les contrôles sur données du patch courant : exhaustivité du catalogue exporté, import runes/sorts/items pour 100 % des champions, échantillon match-v5.
- scripts/patch-watch.mjs:51-80 : main() n'a aucun test automatisé (fetch Data Dragon, analyse de la sortie gh, écriture GITHUB_OUTPUT). C'est de la colle d'E/S et l'auteur le signale. Injecter fetch, exec et le fichier de sortie permettrait de tester au moins l'écriture des sorties should_run, title, etc.
- .github/workflows/patch-watch.yml:74-87 : les étapes de test sont recopiées depuis ci.yml (choix justifié : paths-filter et runners Windows/macOS). Risque de dérive si une étape de ci.yml change. Ajouter dans ci.yml, au-dessus des étapes concernées, un commentaire « à garder synchronisé avec patch-watch.yml ».
- scripts/patch-watch.test.mjs:1 : le commentaire dit « Exécutés par `node --test scripts` » alors que package.json et les deux workflows lancent `node --test scripts/patch-watch.test.mjs`. Aligner le commentaire.
- Workflow non validé par actionlint (absent). Seul le parsing YAML a été vérifié, et le premier déclenchement réel n'est possible qu'après fusion sur main. J'ai vérifié en local que le script détecte 16.19.1, donc le patch 16.19 (should_run=true), et que le filtre gh/jq `select(.title == env.TITLE)` est accepté par gh.
- Comportement opérationnel à accepter : un échec transitoire laisse l'issue ouverte et bloque la relance automatique du patch (seul « force » relance). Les workflows planifiés sont suspendus après 60 jours sans activité. Chaque patch réussi crée une issue aussitôt fermée, soit environ 26 par an. L'index de recherche des issues peut avoir un léger retard, sans effet avec un cron quotidien.
- Le ticket demande aussi que les recettes LCU manuelles soient tracées dans rules/ et dans le DoD. Elles ne sont mentionnées que dans docs/DEMARRAGE.md. La mise à jour du patch par défaut des scripts de skins est absente parce que ces scripts n'existent pas dans cette base (vérifié : aucun fichier suivi sous scripts/ avant ce diff), ce qui est bien signalé.

### PR #147 (ticket #99)

Questions ouvertes :
- Durées de conservation par défaut à valider : 30 jours pour PUUID, Riot ID et observations de rang ; 90 jours pour les parties brutes. Elles sont comptées depuis l'enregistrement (fetched_at / observed_at / dernière activité de l'exécution) et non depuis le début de la partie.
- Contrôle d'identité : le sujet du JWT est un identifiant de session émis par le serveur, pas une identité Riot. J'ai choisi une procédure d'opérateur : liste OLC_API_PRIVACY_OPERATORS, vérification de l'identité hors de l'API, PUUID obtenu par la route profil. Est-ce la cible, ou faut-il prévoir un libre-service via RSO plus tard ?
- Liste d'opposition : après un effacement, une nouvelle collecte peut ré-échantillonner le joueur. Faut-il conserver un registre des effacements (PUUID haché et date) pour bloquer la recollecte ?
- Pseudonymisation complète après 30 jours : une ré-agrégation de ces parties ne peut plus leur attribuer de rang (déjà le cas après 24 h, puisque l'agrégation ne lit que les rangs observés depuis moins d'un jour). Faut-il aussi retirer summonerLevel ou d'autres champs quasi identifiants ?
- Une exécution en pause depuis plus de 30 jours perd ses travaux et sa reprise se termine aussitôt en incomplete. Est-ce acceptable, ou faut-il exclure les exécutions en pause ?
- Exploitation : `purge --watch` doit-il tourner comme un processus supervisé à part, ou être intégré à un `--watch` existant (aggregate / sync-static) ?
- Premier déploiement : la migration 0010 construit des index GIN sur detail et timeline ; elle s'applique au premier lancement d'une commande ou de l'API sur la base réelle. Créneau à choisir.
- Hors de ce livrable, toujours à traiter (lien avec #2) : CGU, politique de confidentialité RGPD, mentions légales, hébergement UE documenté, finalités et bases légales.

Réserves de la revue stricte :
- services/collector/src/privacy.rs:259-263 (purge, étape parties brutes) : seule une exécution au statut `running` protège ses parties, quel que soit son `updated_at`. Une collecte tuée (crash, kill -9, coupure de courant) reste `running` pour toujours : ses parties ne sont alors jamais supprimées à 90 jours. Le PUUID, lui, est bien retiré à 30 jours. Correction proposée : protéger seulement les exécutions `running` actives, avec `AND r.updated_at >= now() - make_interval(days => <identifier_days>)` (même notion d'activité que l'étape 2), et ajouter un cas « running inactif depuis 40 jours » au test d'intégration.
- services/collector/src/storage.rs:817-820 et src/privacy.rs redact_row : la purge marque une timeline `unavailable` de plus de 30 jours (`identifiers_redacted_at = now()`). Si un travail `timeline` encore en attente dans une autre exécution la récupère ensuite, l'upsert la remplace par une timeline complète sans remettre `identifiers_redacted_at` à NULL : les PUUID y restent jusqu'à la suppression à 90 jours. Ce chemin est étroit : il faut un travail créé avant l'enregistrement `unavailable`, encore en attente plus de 30 jours dans une exécution restée active. Correction proposée : ajouter `identifiers_redacted_at = NULL` au `DO UPDATE` de `store_timeline`, avec un test.
- services/api/src/profiles.rs:231-236 et 299-313 : l'effacement ne vide pas les caches mémoire de l'API (profil : TTL de 300 s ; projections de parties indexées par (match_id, puuid) : pas de TTL, vidées seulement au-delà de 4096 entrées). « Efface partout » vaut pour PostgreSQL uniquement : à préciser dans le README de l'API, ou purger ces caches dans `erase`.
- Export (services/collector/src/privacy.rs export_subject) : pour les timelines, l'export ne donne que les identifiants de partie, pas les frames du participant ; pour les travaux de collecte, seulement un compteur. Pour le droit d'accès, c'est un choix à valider et à ajouter aux questions ouvertes.
- Réserves déjà signalées par l'auteur, confirmées : effacement à relancer si `jobs_in_flight` > 0 ; lecture de OLC_API_PRIVACY_OPERATORS dans main.rs non testée ; index GIN et B-tree de la migration 0010 créés sans CONCURRENTLY, avec un verrou d'écriture sur matches et match_timelines pendant leur construction ; `report <id>` perd le détail des travaux après la purge.
- Non testés : la commande CLI `purge` (--watch, --json, code de sortie 3) et la reprise annoncée en `incomplete` d'une exécution en pause depuis plus de 30 jours. Les fonctions sous-jacentes, elles, sont testées sur PostgreSQL.
- Hors périmètre, bien déclaré dans le CHANGELOG et le DoD : finalités et bases légales, CGU, politique de confidentialité, mentions légales, hébergement UE. Les durées par défaut (30 et 90 jours) restent à valider par l'humain.
- Les autres lanes de la nuit peuvent aussi créer une migration 0010. Aucune collision sur les branches distantes ni locales, mais le numéro est à revérifier à l'intégration.

### PR #150 (ticket #98)

Questions ouvertes :
- Modèle de distribution des jetons : aucun endpoint d'émission ni de rafraîchissement n'existe. `olc-api token` limite la durée à 24 h, donc un utilisateur final devrait recoller un jeton chaque jour. Il faut choisir entre un enregistrement d'installation (limité par IP, avec invitation en bêta), un jeton de rafraîchissement ou des jetons longs révocables. Côté serveur, c'est hors de ce livrable.
- Les builds empaquetés (installeur) doivent-ils encore honorer OLC_API_URL et OLC_API_TOKEN, prioritaires sur le trousseau, ou seulement `pnpm dev` ?
- keyring 3.6.3 (backends natifs mûrs, déjà en cache) a été choisi plutôt que la 4.2.0, qui a été restructurée : faut-il prévoir une migration plus tard ?
- macOS : une version non signée ou recompilée déclenche une demande d'accès au trousseau, et l'accès au Keychain dépend de la signature de l'app. La stratégie de signature est liée à #9.
- La carte n'affiche pas encore l'état « jeton refusé ou expiré » (unauthorized) : il n'apparaît que dans les écrans builds et profils et dans publication_state. Faut-il l'afficher aussi dans les réglages ?

Réserves de la revue stricte :
- Windows : la branche windows-native (keyring 3.6.3, set_secret, mapping TooLong et NoStorageAccess) n'a été compilée qu'en lecture de source. Seule la cible aarch64-apple-darwin est installée localement, donc ni cargo check ni clippy n'ont tourné pour Windows. La CI Windows (PR non brouillon) et un test manuel du Gestionnaire d'identifiants sont indispensables avant la fusion. La case DoD « Windows et macOS vérifié » est donc trop affirmative.
- Le vrai trousseau n'a été exercé de bout en bout sur aucun des deux OS : invite Keychain sur une build non signée, entrée Windows `api-access.io.github.bolitow.openlolcompanion`. L'auteur le signale ; il faut le faire avant la fusion.
- Doc incomplète. docs/DEMARRAGE.md:148 (publications) et :231-232 (profils joueurs) disent encore que ce sont « les mêmes OLC_API_URL et OLC_API_TOKEN », sans mentionner le trousseau. docs/imports-automatiques.md:28 ne parle que des variables. Ces passages ne sont pas faux, car les variables restent prioritaires, mais ils sont à compléter par « ou Réglages → Accès à l'API ».
- Ticket #98, partie hors périmètre mais signalée : il n'y a ni état « expiré » dans la carte ni rafraîchissement. Avec un jeton olc-api d'au plus 24 h, la carte affiche « Configuré depuis le trousseau » alors que builds et profils sont en `unauthorized`. Il faut un suivi pour remonter l'état unauthorized dans la carte et pour la conception de l'endpoint d'émission et de rafraîchissement, conception à valider et documentée comme telle.
- Tests de la colle Tauri : `api_access::apply`, `setup`, `save_api_access` et `clear_api_access` (abandon de l'ancien watcher, appel à publications::restart, verrou `writes`) n'ont aucun test direct. Seules les fonctions pures `replace`, `reset` et `accept` sont couvertes. Les deux tests api_access ont été écrits avec le code, sans rouge observé, ce que l'auteur déclare lui-même.
- ApiAccessSettings.tsx : le vidage du champ jeton après un enregistrement réussi et le pré-remplissage de l'URL ne sont pas testés au niveau composant. Le test SettingsScreen vérifie `autoComplete="off"`, attribut déjà présent sur le champ de recherche : cette assertion passe quel que soit le code. Seule `type="password"` discrimine.
- UX : après l'enregistrement d'un jeton, l'écran de builds qui affiche `not_configured` n'a pas de bouton Réessayer (BuildPreparation.tsx) et ne se recharge qu'à la prochaine publication reçue, quand revision augmente. Les messages not_configured des builds et des profils ne renvoient pas vers Réglages → Accès à l'API.
- Si le premier `api_access_status` échoue côté invoke, le statut reste null et le fieldset reste désactivé jusqu'au remontage du composant. C'est improbable, car la commande ne renvoie jamais d'erreur. De même, `store.save` renvoie false sans message quand l'état n'est pas inscriptible.
- publications::restart : si le mutex PublicationsState est empoisonné, `lock().ok()?` renvoie None et l'écoute ne repart pas, sans aucun signal. C'est un cas marginal.
- Cargo.lock ajoute aussi une seconde version de core-foundation, non mentionnée par l'auteur, en plus de security-framework 2.11.1. Les deux viennent des dépendances iOS de keyring et sont sans effet sur les cibles visées.
- .env est ignoré par git (.gitignore:2) : git status ne peut pas prouver qu'il n'a pas été modifié. Sa date de modification (4 oct. 02:45) est seulement notée, sans conclusion.
- Doc de crate : les méthodes `SecretStore::write`, `KeyringStore` sous Linux et les variantes de `CredentialError` n'ont pas de commentaire `///` (rules/documentation.md, fonction publique d'une crate). C'est mineur.

### PR #153 (ticket #90)

Questions ouvertes :
- Quelles files la campagne doit-elle viser par défaut ? Le cahier des charges (§10.1) liste Solo, Flex, 5v5 classé, ARAM, Mayhem, Swiftplay et Arena, et l'agrégation traite toutes ces files : « files servies » ne se réduit donc pas à 420/440. J'ai ajouté `campaign --queue` en gardant le défaut 0 (comportement inchangé). À trancher : défaut 420+440, ou plusieurs files par campagne (une exécution par plateforme et file, ce qui multiplie les exécutions et demande de répartir le budget).
- Purge du cache négatif excluded_matches : aucune rétention n'est appliquée (pas de donnée personnelle, mais la table grossit). À aligner sur les durées de #99 (par exemple raw_match_days) ?
- Numéro de migration 0011 : aucun 0011 trouvé dans les branches locales, mais d'autres lanes peuvent en créer un ; à renuméroter à la fusion si besoin.
- Ordre de réservation : j'ai gardé les timelines avant les détails (seul l'ordre détails avant rangs est changé). Faut-il aussi placer les détails avant les timelines ? Gain de parties, mais des timelines manquantes si le budget s'arrête tôt (builds, #81).
- Les littéraux [420, 440] de services/collector/src/aggregation/model.rs, services/api/src/profiles.rs et crates/build-client/src/profiles.rs ne sont pas remplacés par RANKED_QUEUE_IDS (hors périmètre du ticket) : à traiter dans un ticket de nettoyage ?
- Après fusion de #131, rejouer la maintenance en simulation sur une copie de la base de recette pour chiffrer les demandes de rang à fermer avant tout --apply.

Réserves de la revue stricte :
- services/collector/src/storage.rs:816-848 (close_unserved_rank_jobs) : performance non mesurée sur un vrai volume. Pour chaque rang en attente, deux sous-requêtes corrélées évaluent `m.detail -> 'metadata' -> 'participants' @> to_jsonb(j.job_key)` sur toutes les parties de l'exécution, sans index. Chaque évaluation décompresse le JSONB complet de la partie. Au volume de la recette (≈46 000 travaux, ≈8 600 parties), on s'attend à des centaines de millions de tests de contenance, et l'UPDATE verrouille les lignes pendant ce temps. Suggestion : calculer une seule fois, dans un CTE, les couples (run_id, puuid) via jsonb_array_elements_text, séparément pour les parties classées non remake et pour toutes les parties lisibles, puis faire des anti-jointures par hachage. Mesurer avec EXPLAIN sur une copie avant tout --apply.
- services/collector/src/main.rs:146-150 et 497-499 : le câblage de `campaign --queue` (queue_id: queue, collect_ranks: queue_needs_ranks(queue)) n'a pas de test d'intégration. Seule la fonction queue_needs_ranks est testée unitairement, et tests/campaign.rs n'exerce que queue_id 0. Le défaut est inchangé (0 donne collect_ranks=true), donc ce point n'est pas bloquant. Ajouter un test de campagne en file 450 qui vérifie collect_ranks=false dans les paramètres des exécutions.
- Le point « concentrer les campagnes sur les files servies » n'est livré qu'en option : --queue vaut 0 par défaut, et une seule file est possible (pas 420+440 ensemble). Comme le cahier des charges §10.1 sert aussi ARAM, Arena, etc., garder 0 par défaut se défend. L'auteur l'a signalé en question ouverte ; une décision produit reste à prendre.
- services/collector/src/storage.rs (store_match) : quand une partie d'abord exclue est ensuite acceptée par un autre périmètre (test c), sa ligne reste dans excluded_matches. C'est sans effet aujourd'hui, car find_match est consulté avant le cache négatif, mais la ligne devient obsolète. On peut la supprimer dans la transaction de store_match.
- excluded_matches n'a aucune rétention et grossit sans limite. La table ne contient aucun PUUID ni détail ; la purge est documentée dans le README comme « à décider ». Point signalé par l'auteur.
- services/collector/tests/budget.rs:128 (une_partie_exclue_d_un_perimetre_reste_telechargeable_dans_un_autre) : le commentaire dit « la file 440 accepte cette partie », mais la seconde exécution utilise params(0), c'est-à-dire toutes les files. La logique du test est juste, seul le commentaire trompe.
- Recouvrement avec la PR #131 (non fusionnée) : la constante RANKED_QUEUE_IDS et la ligne `use` de storage.rs sont identiques à celles de #131, et le filtre de link_match (storage.rs vers 1115-1150) reste celui de main. Tant que #131 n'est pas fusionnée, cette branche crée encore des rangs pour toutes les files. Il faut fusionner #131 avant ou en même temps. Le conflit attendu sur model.rs, storage.rs et le CHANGELOG est trivial.
- La migration 0011_excluded_matches.sql n'entre en collision avec aucune branche distante ni avec les branches locales lane-e, 98 et 99. Le numéro est à reconfirmer au moment de la fusion.
- Mineur : dans services/collector/src/main.rs, le champ `json` de CloseUnservedRanks n'a pas de commentaire d'aide pour la CLI.
- Non vérifié : psql est absent de la machine, donc je n'ai pas pu compter les rangs réellement en attente hors 420/440 dans olc_nuit_bc. Je n'ai pas lancé le binaire sur cette base, volontairement : connect() y appliquerait la migration 0011, et les autres lanes qui partagent cette base ne la connaissent pas.

### PR #157 (ticket #122)

Questions ouvertes :
- Les attentes « ne passe pas » de 50 à 200 ms des autres tests restent chronométrées : elles ne peuvent pas échouer sous charge, mais elles ne prouvent le blocage que sur cette durée. Faut-il les allonger ?

Réserves de la revue stricte :
- services/collector/src/shared_quota.rs:11 : le commentaire de INTERACTIVE_RESERVE_PERCENT dit encore « une recherche de profil n'attend pas la fin d'une rafale ». C'est la même sur-promesse que celle retirée du CHANGELOG et du README. Le fichier est déjà dans le diff : reformuler en « réduit l'attente d'une recherche de profil pendant une rafale », pour que le code ne contredise pas la doc.
- services/collector/tests/shared_quota.rs:31-33 : le commentaire placé sur LONG_WINDOW_MS (« Marge des attentes « ne doit pas passer » : très inférieure aux fenêtres de quota ») décrit les attentes courtes (50 à 200 ms), pas la constante, qui est la fenêtre de 60 s elle-même. À reformuler, par exemple « Fenêtre de quota des tests, bien plus longue que toute attente du test ».
- services/api/README.md:172-175 : pour le cas du 429 Riot, « refus sans attente de quota » est inexact, car l'appel a pu attendre un créneau avant que Riot ne réponde 429. « propagé tel quel » est aussi ambigu face à « Aucun corps brut Riot » : le 429 est converti en code `rate_limited`, rien n'est relayé. Le saut de ligne est irrégulier : une ligne très longue, puis « `riot_busy` (503, » isolé. Proposition : « … ou Riot a répondu 429 (converti en `rate_limited`, sans contenu Riot) ».
- CHANGELOG.md:15 : l'entrée #122 est devenue un paragraphe très long qui mêle l'effet utilisateur (réserve, limites, codes d'erreur) et des détails internes de test (fenêtres de 60 s, attentes de 50 à 200 ms). rules/changelog.md demande une ligne compréhensible par un joueur ou un contributeur. Suggestion : déplacer la priorité obligatoire et la robustesse des tests dans une ligne courte sous « Modifié », ou les retirer.
- services/collector/tests/shared_quota.rs:61-73 (deux_pools_partagent_les_reservations_avant_envoi) : le test ne vérifie pas que la tâche terminée a renvoyé Ok. Le comptage `sent.len() == 1` le couvre en pratique, mais un `assert!` sur le résultat rendrait l'échec plus lisible. Le test ne vérifie plus non plus qu'un second pool finit par passer une fois la fenêtre écoulée. Ce glissement reste couvert par le test unitaire une_reservation_persiste_et_la_fenetre_glisse_sans_rafale : acceptable, à signaler dans la PR.

## Lane F — Conformité, imports, site

| Ticket | PR | Lien | Base | Verdict de la revue stricte | Réserves | Questions |
|---|---|---|---|---|---|---|
| #79 | [#134](https://github.com/bolitow/open-lol-companion/pull/134) docs(conformite): cahier des charges et règles recoupés sur les textes Riot d'origine (#79) | Closes | `main` | OK avec réserves | 5 | 3 |
| #102 | [#137](https://github.com/bolitow/open-lol-companion/pull/137) feat(live): élargir la projection Live Client par liste blanche et la conserver jusqu'au bilan (#102) | Refs | `docs/79-cahier-conformite` | OK avec réserves | 9 | 6 |
| #30 | [#138](https://github.com/bolitow/open-lol-companion/pull/138) docs(conformite): revue Riot des overlays et projections (#30) | Closes | `feat/102-projection-live-liste-blanche` | OK avec réserves | 10 | 4 |
| #88 | [#145](https://github.com/bolitow/open-lol-companion/pull/145) fix(imports): set d'objets limité aux objets achetables du catalogue (#88) | Refs | `docs/30-revue-conformite-overlays` | OK avec réserves | 6 | 2 |
| #20 | [#151](https://github.com/bolitow/open-lol-companion/pull/151) feat(web): site Next.js avec tierlist, page champion, profil et recherche (#20) | Refs | `fix/88-import-set-objets-blocs` | OK avec réserves | 10 | 4 |
| #39 | [#155](https://github.com/bolitow/open-lol-companion/pull/155) feat(shared): modèle de draft descriptif v1 (#39) | Refs | `feat/20-site-tierlist-champion` | OK avec réserves | 7 | 2 |

### PR #134 (ticket #79)

Questions ouvertes :
- Valider l'ancre #developer-api-policy_game-integrity dans un navigateur : la relecture a utilisé un outil de résumé de page.
- La lecture des mots « tracking enemy ultimate cooldowns » est confirmée par l'outil de résumé et par la revue ; une vérification humaine du texte exact est souhaitable avant fusion.
- Soumettre à Riot (#2, #30) les cas non tranchés : popularité seule d'un augment, overlay en sélection d'augment, premades au chargement, comparaison à l'adversaire direct.

Réserves de la revue stricte :
- docs/cahier-des-charges.md:180 : « l'app propose le meilleur pick » et docs/cahier-des-charges.md:240 « Item conseillé » (au singulier) contredisent la règle ajoutée au §2 et au §6 (mettre en évidence et proposer plusieurs choix, jamais imposer l'unique bon geste ; Developer API Policy : « give multiple choices »). Ce n'est pas bloquant, puisque le §5.2 note tous les champions. Reformulation suggérée : « propose les meilleurs picks » et « Items conseillés (plusieurs options) ».
- docs/cahier-des-charges.md:471 : le lien league-client-apis.html redirige vers docs/lol#league-client (lien périmé). La mention « relu par résumé automatique » contredit la règle posée au §2 (une lecture de seconde main ne tranche jamais). Le texte primaire existe : j'ai relu la page complète avec curl (« not officially supported », « no guarantees »), et les politiques générales imposent l'enregistrement en cas d'usage de la LCU. Correction : pointer vers https://developer.riotgames.com/docs/lol#league-client et retirer la mention du résumé automatique.
- Points du ticket #79 hors du périmètre documentaire, que l'auteur ne liste pas tous comme questions ouvertes : (1) cadrer #45 par un commentaire, (2) poser à Riot via #2 et #30 les questions sur la popularité seule d'un augment et sur l'overlay en sélection d'augment, (3) renforcer le test de projection de draft (puuid, gameName, tagLine, absence de puuid dans l'IPC). Le point (3) est cité à rules/conformite-riot.md:45, mais aucun ticket ne le suit. À reporter dans un ticket ou dans le résumé de la PR pour ne pas les perdre.
- Hors périmètre de #79, en lien avec les limites de débit collées par l'utilisateur (20 requêtes par seconde et 100 requêtes par 2 minutes, appliquées par valeur de routage) : docs/cahier-des-charges.md:349 dit « file par région ». Riot compte les quotas par valeur de routage, qu'il s'agisse d'une plateforme (euw1) ou d'un routage régional (europe, americas). Formulation suggérée : « file par valeur de routage (plateforme et région) ».
- Antérieur au diff : docs/cahier-des-charges.md:39 interdit la publicité « dans les overlays » et cite la politique de mai 2025. Le texte des politiques générales vise « in-game, loading screens, and the Riot Client ». La règle du projet (aucune publicité nulle part) reste plus stricte, mais l'attribution à Riot est approximative.

### PR #137 (ticket #102)

Questions ouvertes :
- Fixture réelle anonymisée Windows et macOS : impossible de la produire hors partie. Les noms d'événements (DragonKill, HeraldKill, BaronKill, TurretKilled, InhibKilled, FirstBlood, Ace, ChampionKill) et les champs Recipient, Acer, AcingTeam, KillerName, VictimName et Assisters viennent du ticket et de la documentation Riot. Ils sont testés sur des charges synthétiques dans tests.rs, pas sur une capture. À figer lors de la prochaine recette.
- Format réel de `KillerName` dans les événements du client actuel : riotId complet, nom court ou ancien pseudo ? Le code essaie les trois et n'attribue rien si le nom est ambigu (`ally: null`, `involvesLocalPlayer: false`). À vérifier sur capture : si le format ne correspond à aucun, la participation du joueur local sera sous-estimée.
- Événements hors liste blanche non ajoutés faute de vérification : Multikill, FirstBrick, InhibRespawningSoon, InhibRespawned, HordeKill (larves) et AtakhanKill. Le module Timers d'objectifs (§6) en a besoin pour Larves et Ancien ; à ajouter après capture réelle. DragonType, Stolen et le nom de tourelle ne sont pas projetés.
- Totaux d'or d'équipe et différence d'or non calculés (cas limite du cahier §6 et de rules/conformite-riot.md, à trancher via #30). Le score de vision adverse n'est pas agrégé non plus (affichage au tableau des scores non confirmé).
- Règle de purge du bilan : retenu jusqu'à une nouvelle partie ou la sortie des phases WaitingForStats, PreEndOfGame et EndOfGame, sans commande de purge explicite ni écriture disque. Matthieu valide-t-il cette règle, et faut-il une commande Tauri de purge côté bilan (consommateur #24 ou #36) ?
- La liste blanche élargie doit être soumise à la validation Riot prévue par #30 avant publication publique.

Réserves de la revue stricte :
- Branche : le worktree /Users/bolito/dev/olc-nuit/lane-f est sur `docs/79-cahier-conformite`, et non sur `feat/102-projection-live-liste-blanche` comme l'annonce le cadre. Il faut créer ou basculer la bonne branche avant tout commit, sans quoi le lot #102 atterrira dans la branche documentaire de #79.
- Aucune capture réelle ne valide les nouveaux champs. La fixture crates/lcu-connector/tests/fixtures/live-client-allgamedata-macos-2026-10-03.json se réduit à `activePlayer:{riotId}`, un seul joueur et GameStart/MinionsSpawning. Les noms d'événements, mais aussi tous les noms de champs (`currentGold`, `abilities.*.abilityLevel`, `team`, `position`, `isDead`, `respawnTimer`, `wardScore`, `KillerName`, `VictimName`, `Assisters`, `Recipient`, `Acer`, `AcingTeam`), ne sont exercés que sur le `full_payload` synthétique. L'auteur ne signale que les noms d'événements. Il faut les rattacher à la recette Windows/macOS et à la fixture complète anonymisée que demande le ticket.
- Le test `projette_la_capture_reelle_macos_sans_identite_et_sans_alterer_les_valeurs` (crates/lcu-connector/src/live/tests.rs, vers la ligne 445) n'a pas été étendu. Il pourrait affirmer que les champs optionnels valent None et que `teams` vaut None sur la capture réduite, ce qui figerait le comportement sur une charge réelle.
- Le « détail des objets » cité dans le constat du ticket n'est pas livré : on reste aux seuls `itemID`. Il n'apparaît ni dans la liste hors périmètre ni dans les questions ouvertes. À ajouter aux questions ouvertes.
- Le ticket recommande que les kills soient « réduits à un booléen implique le joueur local ». Le livrable ajoute un second bit par kill, `ally` (camp de l'auteur). C'est défendable, puisque le fil des kills est public, mais cela dépasse la recommandation : à inscrire explicitement dans la liste blanche soumise à Riot (#30). Même remarque pour `teams.enemies.creepScore`, proche du cas limite « comparaison à l'adversaire » du cahier (§6, ligne 239) : fusion seulement si la liste blanche est assumée en attendant #30.
- Purge du bilan (crates/lcu-connector/src/live/mod.rs:424-432) : toute phase connectée qui n'est ni « en partie » ni « après-partie » purge `postgame`. Cela inclut `None`, `TerminatedInError` et `Unknown` (nouvelle phase Riot, `#[serde(other)]`). Une phase transitoire ou inconnue entre InProgress et WaitingForStats ferait perdre le bilan. Il faudrait décider explicitement de ces cas, voire traiter `Unknown` comme neutre, et le tester.
- Sur une déconnexion LCU après la partie, `postgame` reste en mémoire sans limite, jusqu'à la partie suivante ou au redémarrage de l'app. C'est documenté dans docs/live-overlay.md et ce n'est pas un problème de conformité, mais c'est plus long que « jusqu'au bilan ».
- `teams()` (mod.rs:165-195) agrège par ORDER/CHAOS sans tenir compte du mode. En Arena ou dans un mode à plus de deux camps, « allies » et « enemies » seraient faux. Aucun consommateur ne l'utilise encore, mais il faudrait renvoyer `null` hors des modes à deux équipes, ou documenter la limite.
- Couverture : le cas où `ability_levels` est partiellement présent (Q seul) n'est pas testé, et l'helper `optional_time` est réutilisé pour l'or et la vision alors que son nom ne l'indique pas (lisibilité mineure).

### PR #138 (ticket #30)

Questions ouvertes :
- Objets, sorts d'invocateur (sans recharge) et niveau des adversaires, visibles au tableau des scores : j'ai choisi « à soumettre / à confirmer en recette, non projetés aujourd'hui » plutôt que refus ni autorisation. À valider.
- Vision adverse : scindée entre refus (wards et vision non vus) et à confirmer (score de vision au tableau des scores). À valider.
- La source de la différence d'or (valeur des items de chaque joueur, cahier §6) reste à soumettre car le Live Client n'expose pas l'or adverse ; l'or exact adverse reste refusé. Les événements nominatifs sont déplacés de refus à « à soumettre » : à valider avec le cahier §6.
- Questions héritées de la revue : mention légale dans le panneau, exclusion de prototype.html du build de production, filtre défensif des bans adverses, confirmation en partie réelle de la visibilité du CS adverse.

Réserves de la revue stricte :
- docs/revues/2026-10-04-conformite-overlays.md:10-11 : les citations « GP-approuvé » (données statiques avant la partie, (self) player stats, agrégats sans joueur précis) viennent, selon le rapport lui-même, d'une « lecture assistée » à recouper. Ce sont pourtant elles qui fondent une douzaine de lignes « Conforme » (§1 l.19-24, §2, §4 l.71-76 et 83), alors que rules/conformite-riot.md:7 dit qu'une lecture de seconde main ne tranche jamais. Le fond est déjà acquis par le cahier §2 « Autorisé » (recoupé le 04/10/2026, #79) : il faut ancrer ces décisions sur cette ligne du cahier et laisser les citations anglaises comme « à recouper ».
- docs/revues/2026-10-04-conformite-overlays.md:29 et :117 : « voir questions ouvertes » renvoie à une section qui n'existe pas. Le §8 utilise « Non corrigés, listés » et « Décisions à valider par Matthieu ». Il faut corriger le renvoi.
- rules/conformite-riot.md:34 : « Poste ou intention de pick adverse en sélection, avant verrouillage » peut se lire comme si le poste adverse devenait autorisé après verrouillage. Le code le force à vide en permanence (draft.rs, `position` = None si `enemy`) et le rapport l.50 le dit sans réserve. Reformulation proposée : « Poste adverse (à tout moment) et intention de pick adverse avant verrouillage ». La section Anonymat (l.63) couvre déjà ce cas, d'où un risque limité.
- rules/conformite-riot.md:48-49 : « buffs d'objectifs » figure dans deux lignes des cas à soumettre (événements nominatifs, puis timers d'objectifs). Il faut garder une seule ligne. L'overlay « Rappels » du cahier §6 (sort à maxer, trinket) n'est pas dans la liste des cas à soumettre : il n'est pas implémenté, mais le cahier §2 renvoie ces usages à #30.
- Bans adverses (rapport l.55) : ils sont affichés dans DraftBoard via `enemyBans` (apps/desktop/src/app/draft.ts:11) sans filtre défensif en Rust, et classés « Conforme, à confirmer ». Ce point est bien listé au §8 comme non corrigé. Il faut le confirmer en recette avant sortie, ou ajouter un filtre testé sur le modèle des intentions adverses.
- Prototype empaqueté dans l'installeur (apps/desktop/vite.config.ts:12, entrée `prototype`) : l'auteur l'a listé au lieu de le corriger, alors que le retrait tient en une ligne. C'est conforme à la consigne « sinon le lister » mais reste à trancher.
- Mention légale absente du panneau en partie (rapport l.29) : la checklist l.15 associe la mention aux assets Data Dragon / CommunityDragon, et le panneau affiche des icônes de champions et d'objets. Le rapport en fait une décision produit ; ce point reste une question ouverte, pas un sujet clos.
- Test `aucune_identite_des_joueurs_ne_sort_de_la_projection` (crates/lcu-connector/src/draft.rs:369) : la liste figée des clés de premier niveau exclut `gameId` et `queueId` uniquement parce que la fixture n'a pas de gameId et que `parse()` utilise `StandardRift(None)`. Un nouveau champ `Option` en `skip_serializing_if` resté à None échapperait à la liste. Seul l'élément [0] de chaque équipe est contrôlé. C'est une fragilité de robustesse, pas un défaut de conformité.
- Le rouge par mutation (champ `puuid` ajouté puis retiré de `DraftPlayer`) n'a pas pu être revérifié sans modifier de fichier, ce qui est interdit. L'`assert_eq!` sur la liste des clés rend l'affirmation plausible, mais elle repose sur la parole de l'auteur.
- Le §8 du rapport daté décrit l'historique des tours de revue (« Corrigé au tour 1 de revue »). C'est du bruit de processus dans un document d'audit, à retirer ou à déplacer dans la PR.

### PR #145 (ticket #88)

Questions ouvertes :
- Le ticket #88 demande un set en plusieurs blocs (départ, core, situationnel) : ce livrable garde un seul bloc en attendant les catégories d'agrégation de #81. Faut-il bloquer le merge sur #81 ou fermer #88 partiellement ?
- L'ordre réel des sets dans la boutique League reste à confirmer sur Windows (déjà noté dans docs/imports-client.md).

Réserves de la revue stricte :
- Périmètre « plusieurs blocs (départ, core, situationnel) » non livré : le set reste en un seul bloc. La seule mention se trouve dans docs/imports-client.md (vers la ligne 642), qui renvoie à #81, encore ouvert ; le résumé et la DoD de l'auteur n'en parlent pas. La recommandation du ticket place bien ces blocs dans un « Ensuite », c'est pourquoi le point n'est pas bloquant. En revanche, la future PR ne doit pas fermer #88 (écrire « Refs #88 », pas « « Closes » du ticket 88 »), ou la suite doit être tracée explicitement sur #81. L'ordre par ID de final_items, hérité de services/collector/src/aggregation/builds.rs:40 et décrit dans le constat du ticket, n'est pas traité non plus (hors périmètre déclaré) : il faut le signaler comme reste à faire.
- CHANGELOG.md:13 : la 3e ligne (« Cœur Rust de l'import d'objets : la table de conversion codée en dur disparaît… ») décrit un changement interne sans effet visible pour le joueur. rules/changelog.md demande de la fusionner avec la 1re ligne ou de la placer sous « Modifié » (changement côté contributeurs), pas sous « Corrigé ».
- docs/imports-client.md:652 (section #61) affirme toujours « L'ordre et les répétitions des achats sont conservés ». Depuis #88, plusieurs formes d'une même lignée ne produisent l'ancêtre qu'une fois. La phrase est à nuancer, ou à faire pointer vers le paragraphe Forme achetable, comme l'explique déjà la ligne 638.
- apps/desktop/src/app/itemImport.ts:70 : la déduplication est asymétrique. Avec [3042,3004], 3004 est émis par conversion puis une seconde fois tel quel, car la branche `ancestor===id` ne consulte pas `emitted`. Avec [3004,3042], il n'est émis qu'une fois. Le cas est peu probable en pratique, mais il vaut la peine de choisir une règle et de la tester.
- apps/desktop/src/app/BuildPreparation.tsx:46 : le branchement `importable={isImportableCategory(category)}` n'est pas testé au niveau du panneau (catégories item et trinket). Seuls isImportableCategory et ItemImport le sont, chacun de son côté. Le risque est faible, car le câblage tient en une ligne.
- Pas de recette sur un vrai client League : la priorité visuelle du set dans la boutique et l'ordre sous Windows restent à vérifier, comme le reconnaît la doc.

### PR #151 (ticket #20)

Questions ouvertes :
- Parmi les exigences §9 / #20 non fournies par l'API, lesquelles prioriser côté API ? Il y a les stats par champion du profil, le pic de rang et le suivi des LP (persistance des profils, cf. commentaire d'analyse de #20), les builds pro, les contres et synergies, le filtre de période et les transferts de région.
- Le filtre de période du §9 peut-il être remplacé durablement par le filtre de patch, ou faut-il une agrégation par fenêtre de dates dans l'API ?
- Qui renouvelle le jeton API du site (24 h au plus) chez l'hébergeur, et faut-il un ticket de rotation automatique ?
- Qui fait et quand la recette navigateur du site sur Windows et macOS (cases encore ouvertes de #20) ?

Réserves de la revue stricte :
- pnpm-lock.yaml : le lockfile a été régénéré avec le pnpm local (9.12.0), alors que le packageManager impose pnpm@10.28.0. Il perd ainsi 18 contraintes `libc: [glibc|musl]` sans rapport avec #20. J'ai vérifié dans une copie temporaire : `corepack pnpm@10.28.0 install --frozen-lockfile` accepte ce lockfile, la CI ne casse donc pas. Pour garder un diff propre, régénérer avec `corepack pnpm@10.28.0 install`.
- apps/web/src/lib/i18n.ts:132 et :256 : l'aide `shortcut: "Ctrl K"` s'affiche aussi sur macOS, où le raccourci habituel est Cmd K. Le raccourci fonctionne et il est testé, mais l'indication trompe sur le second OS. Adapter le libellé à la plateforme ou afficher « Ctrl/Cmd K ».
- apps/web/src/components/LanguageSwitch.tsx:18-22 : les filtres ne sont gardés qu'en modifiant le href au clic. Le href rendu par le serveur n'a pas de chaîne de requête, donc les filtres sont perdus sans JavaScript, au clic du milieu et à l'ouverture dans un nouvel onglet. Utiliser useSearchParams (dans un Suspense) pour calculer le href complet.
- apps/web/src/app/[locale]/profile/[platform]/[gameName]/[tagLine]/page.tsx:52-56 : api.matches est lancé en parallèle de api.profile, même quand le profil renvoie 404. Un Riot ID mal saisi coûte donc deux appels avec une clé limitée à 100 requêtes / 2 min. Appeler matches seulement si le profil répond.
- Séparateur « : » en dur, avec l'espace à la française, aussi affiché en anglais : profile/.../page.tsx:79, profile/.../page.tsx:23 (titre) et champions/[slug]/page.tsx:26 (titre). L'anglais affiche par exemple « Ahri : builds, runes and skills ». Mettre ce séparateur dans les dictionnaires.
- SEO de la page champion demandé au §9 : generateMetadata (champions/[slug]/page.tsx:25-28) ne donne qu'un titre et les alternates. Il n'y a ni description ni URL canonique.
- profile/.../page.tsx:44-45 : un Riot ID ou une région invalide renvoie un HTTP 200 avec un message, au lieu de notFound().
- Les pages serveur (tierlist, champion, profil) et StatsFilterForm n'ont pas de test de rendu. Leur logique est couverte par les tests de lib/* (filtres, tierlistRows, patchOptions, rankLabel…), mais pas le code JSX qui les relie (branches noPatch, erreur, vide avec « autres patches »).
- README : « aucune file Arena » vaut pour l'API, pas pour le site. queueName (profile/.../page.tsx:30-33) affiche n'importe quel queue_id renvoyé par /matches. Aucun augment n'est affiché, donc la conformité tient, mais il faut préciser la phrase ou filtrer la file.
- Restent ouverts, comme le signale l'auteur : la recette navigateur Windows (Louison) et macOS (Matthieu), le renouvellement du jeton (24 h au plus, borne confirmée dans services/api/src/auth.rs:43), et les fonctions §9 non fournies par l'API (listées dans le README, profile.notProvided, et #110 pour la période). Le message doit rester « Refs #20 ».

### PR #155 (ticket #39)

Questions ouvertes :
- Pour le poste UNKNOWN, faut-il garder le type d'entrée `Role` avec `role_share: null` (choix actuel) ou restreindre `DraftModelInput.role` à `Exclude<Role, "UNKNOWN">` au niveau du typage ?
- L'affichage du % de victoire estimé en sélection des champions doit-il attendre la réponse de Riot (#2/#30), comme la probabilité de victoire en direct ?

Réserves de la revue stricte :
- packages/shared/src/draftModel.ts:104-107 et 113-116 : rien ne déduplique les entrées `(champion_id, role)`. Si l'appelant fusionne des pages de la tierlist qui se recouvrent, on obtient des candidats en double, un dénominateur de `role_share` gonflé et des sommes tous postes confondus faussées. Le README demande « toutes les pages » mais ne précise pas « sans doublon », et aucun test ne couvre ce cas. À faire : dédupliquer ou documenter ce prérequis et le tester.
- packages/shared/src/draftModel.ts:149-154 : il y a deux seuils. Le `score` d'un candidat et le taux d'un allié à son poste reprennent le seuil publié par le collecteur (valeurs à null), alors que `anyRoleRate` applique `input.min_games` aux parties additionnées. Si l'appelant ne passe pas exactement `meta.min_games`, un champion n'est pas éligible de la même façon côté candidats et côté estimation. Seul un commentaire de type le signale, aucun test ne le couvre.
- Le ticket (§10.3 et §5.2) contient des éléments hors du périmètre annoncé qui ne figurent pas dans les réserves de l'auteur : la « prédiction de rôle » de §10.3, les filtres de §5.2 (maîtrise minimale, pool de champions, off-meta, recherche) et la comparaison d'équipes (AD/AP, radar). Il faut les nommer comme suites de #39 (ou de #4) pour que le ticket ne soit pas clos sur cette v1.
- Le commentaire du 4 octobre sur #39 recommande d'amender §10.3 avant la conception du modèle (maîtrise des dix joueurs, variable indisponible à l'inférence). Ce n'est pas fait : le tableau §10.3 annonce toujours en entrée « 10 champions + rôles, maîtrise ». Le modèle n'utilise aucune maîtrise, donc ce n'est pas bloquant, mais l'écart entre le cahier et le livrable doit être signalé ou corrigé.
- Conformité Riot : dans rules/conformite-riot.md, le tableau « Cas à soumettre à Riot » n'a aucune ligne pour l'estimation de victoire du draft en sélection, avant la partie. Sa seule ligne sur la probabilité de victoire concerne l'overlay en partie. L'obligation du §2, faire valider le draft IA par Riot, n'est donc suivie nulle part dans les règles. La réserve (3) de l'auteur est juste et doit rester : rien de `scoreDraft` ne doit s'afficher avant cette validation.
- Les réserves de l'auteur restent valables : (1) matchups et synergies absents faute d'agrégat publié (vérifié : aucun agrégat de matchup ni de synergie dans packages/shared, services ni crates) ; (2) l'appelant doit fournir toutes les pages des cinq postes et d'UNKNOWN ; (4) `role_share` vaut null pour UNKNOWN.
- La phase rouge du TDD (« Received: 400 ») n'a pas pu être revérifiée. Le test l'expliquerait pourtant : 200 parties UNKNOWN divisées par 50 parties TOP donnent 400. Il passe aujourd'hui.

## Lane G — Agrégation, seconde file (empilée sur feat/81-builds-par-etapes, worktree de la lane D libérée)

| Ticket | PR | Lien | Base | Verdict de la revue stricte | Réserves | Questions |
|---|---|---|---|---|---|---|
| #124 | [#149](https://github.com/bolitow/open-lol-companion/pull/149) feat(collector): conserver l'orientation D/F majoritaire des sorts d'invocateur (#124) | Refs | `feat/81-builds-par-etapes` | OK avec réserves | 6 | 3 |
| #104 | [#154](https://github.com/bolitow/open-lol-companion/pull/154) feat(collector): agrégats Arena sur le placement (#104) | Refs | `feat/124-sorts-df` | OK avec réserves | 7 | 2 |
| #110 | [#158](https://github.com/bolitow/open-lol-companion/pull/158) feat(api): tendances entre patchs d'un champion (#110) | Refs | `feat/104-arena-placement` | OK avec réserves | 8 | 6 |
| #80 (suite) | [#159](https://github.com/bolitow/open-lol-companion/pull/159) perf(collector): rang figé — recherche par l'index, borne exacte et recette (#80) | Refs | `feat/110-tendances-patchs` | OK avec réserves | 7 | 3 |
| #103 | [#162](https://github.com/bolitow/open-lol-companion/pull/162) feat(api,collector): fraîcheur réelle des statistiques (#103) | Refs | `perf/80-rang-fige-index-et-compat` | OK avec réserves | 7 | 1 |
| #119 | [#166](https://github.com/bolitow/open-lol-companion/pull/166) feat(collector,api): winrate par durée, côté et premiers objectifs (#119) | Refs | `feat/103-fraicheur-donnees` | OK avec réserves | 9 | 7 |
| #113 | [#169](https://github.com/bolitow/open-lol-companion/pull/169) fix(api): variantes de builds omises par groupe et catégorie, item_events plafonnés (#113) | Refs | `feat/119-winrate-duree` | OK avec réserves | 6 | 2 |

### PR #149 (ticket #124)

Questions ouvertes :
- Faut-il livrer la seconde moitié de la recommandation côté import (crates/lcu-connector/src/imports/spells.rs) : pour une paire sans Flash, garder la case d'un sort déjà équipé dans my-selection avant d'appliquer l'orientation majoritaire ? Cela demande de lire la sélection courante et un contrat à valider, donc laissé pour un ticket ou une PR séparés ; #124 ne devrait pas être fermé tant que ce point n'est pas tranché.
- Pour une paire avec Flash, l'import continue de suivre la préférence D/F du joueur (inchangé) ; l'orientation majoritaire n'est qu'affichée dans la carte. Est-ce le comportement voulu ?
- Les agrégats déjà publiés gardent l'ordre numérique jusqu'à la prochaine agrégation : une ré-agrégation sur la base de recette `olc_nuit_d` est à prévoir pour constater l'effet sur des données réelles (non lancée cette nuit).

Réserves de la revue stricte :
- Ticket #124 couvert à moitié : la partie import (lcu-connector, « garder la case d'un sort déjà équipé » pour une paire sans Flash) n'est pas faite. L'auteur le signale (Vigilance 2, open_questions). La PR ne doit donc pas fermer #124 (pas de « « Closes » du ticket 124 ») ; il faut laisser le ticket ouvert ou créer un ticket de suite.
- apps/desktop/src/app/buildCopy.ts:7 et :18 : la clé `spellHint` n'est lue nulle part. BuildPreparation.tsx:54 affiche la paire sans aucun texte d'aide, alors que purchaseHint, itemHint, finalHint et skillHint sont bien rendus. La traduction FR/EN mise à jour reste donc invisible, et rien dans l'interface n'explique que l'ordre affiché est l'orientation D/F majoritaire. Le problème existait avant ce livrable, mais la case i18n du DoD n'apporte rien. Il faudrait afficher `t.spellHint` dans la vue des sorts, ou retirer la clé, dans ce livrable ou dans un ticket de suite.
- services/api/src/stats.rs:99 trie à nouveau à effectif égal sur `selection`, qui peut maintenant être inversée ([12,6]). L'agrégation, elle, départageait sur la paire triée, en tenant compte aussi des victoires. Quand deux paires de sorts ont exactement le même effectif, leur ordre relatif peut donc changer, ainsi que la variante retenue en premier par autoImport.ts:59. L'effet reste visuel et marginal.
- Les agrégats déjà publiés gardent l'ordre numérique jusqu'à la prochaine agrégation complète. La recette sur `olc_nuit_d` n'a été lancée ni par l'auteur ni par le relecteur : `psql` est absent de la machine, donc pas de contrôle SQL sur données réelles. Seuls les tests unitaires couvrent la fonctionnalité.
- Pour une paire avec Flash, l'orientation majoritaire sert seulement à l'affichage en lecture seule. L'import applique toujours la préférence D/F du joueur (spells.rs:37-40), ce qui est conforme au §5.4 du cahier des charges. Le texte `spellHint` le dit, mais il n'est pas affiché (voir plus haut).
- Le rouge du TDD n'a pas pu être vérifié a posteriori. En revanche, les trois tests d'agrégation vérifient des sorties qui diffèrent sans la logique d'inversion ([12,6] contre [6,12], [14,4] contre [4,14]) : ce sont de vrais tests de comportement. Le test d'égalité est seulement une garde de non-régression.

### PR #154 (ticket #104)

Questions ouvertes :
- Les variantes de builds hors objets (runes, sorts, ordre de compétences) gardent leur taux de victoire en Arena, conformément au README du collecteur ; faut-il aussi les basculer sur le placement dans un ticket séparé ?
- Un instantané publié avant ce ticket garde l'ancien classement Arena jusqu'au prochain calcul d'agrégation par le collecteur : déclencher un recalcul après fusion ?

Réserves de la revue stricte :
- Variantes de builds hors objets (summoner_spells, skill_order, special_skill_order) : en Arena, elles publient encore un win_rate et une borne Wilson calculés sur le booléen `win`. Ce booléen vaut « top 3 sur 6 », et le README modifié le déclare lui-même sans valeur de première place. Je l'ai vérifié sur un vrai `aggregate --json` (16.19/TR1) : performance_available=true pour 1 218 variantes skill_order et 530 variantes summoner_spells. L'auteur l'annonce hors périmètre et le documente (services/collector/README.md, « gardent leur taux de victoire en Arena »), donc ce n'est pas bloquant. La recommandation du ticket (« Pour les scopes Arena… masquer win_rate ») reste toutefois partiellement couverte : à traiter dans un ticket de suite ou à justifier dans la PR.
- Métrique livrée différente du ticket : le ticket #104 recommande les taux de top 1 et de top 4, le livrable publie top1_rate et top2_rate. C'est conforme au périmètre de la lane (top 1/top 2), mais le résumé ne signale pas l'écart. Il faut le mentionner pour ne pas clore #104 en croyant le top 4 couvert.
- Conformité Riot, prérequis administratif : la section Conformité du ticket exige une validation écrite (cahier §2) avant d'exposer le placement moyen par champion. La DoD indique « vérifiée » sans dire que cette validation est encore à obtenir. Il faut le noter dans la PR comme prérequis avant toute exposition dans l'interface.
- Tests sur des parties 1740/1750 réelles demandés par le ticket : tous les nouveaux tests sont synthétiques. J'ai réduit l'écart en interrogeant la base olc_nuit_d (voir le résumé), mais aucune fixture tirée de vraies données n'est ajoutée.
- Identifiants en français dans les nouveaux tests de services/collector/src/aggregation/tests.rs (`parties`, `hors_a`, `ordre`, `rang`, `doublon`, `incoherent`, `hors_borne`) : AGENTS.md impose des identifiants en anglais (seuls les noms de tests sont en français, rules/architecture.md). À renommer.
- Mineur, performance : dans le comparateur de tri (model.rs, `self.arena_scopes.contains(&scope_of(&a.key))`), deux String sont clonées à chaque comparaison qui arrive jusqu'à ce départage. Le motif existe déjà dans le code, c'est négligeable hors boucle de rendu ; on pourrait précalculer un booléen par groupe.
- Instantanés publiés avant ce ticket : leurs groupes Arena gardent l'ancien classement par victoire jusqu'au prochain calcul. C'est signalé par l'auteur ; il faut relancer l'agrégation après fusion.

### PR #158 (ticket #110)

Questions ouvertes :
- Historisation durable : résumé compact par (patch, plateforme, file, rôle, palier, champion) à chaque changement de patch, nouvelle table ou extension de l'instantané ? Les rangs du patch précédent expirent (#80) : faut-il figer un résumé avant expiration ?
- Fenêtre glissante des 7 derniers jours et repli sur le patch précédent sous le seuil : à trancher côté produit avant tout calcul.
- Les tendances Arena (placement moyen, top1/top2) ne sont pas incluses (win_rate nul en Arena, point sans valeur de win) : les ajouter à TrendPoint dans un ticket séparé ?
- Les écarts comparent des patchs consécutifs de l'instantané, pas forcément adjacents dans le jeu (un patch absent est invisible) : acceptable ou faut-il exposer un indicateur de trou ?
- Valeurs avant/après dans le diff de catalogue et ticket « changements de patch et tendances » : non traités ici, à ouvrir.
- Recette sur la base olc_nuit_d (service API démarré et jeton) non exécutée cette nuit : à faire manuellement pour valider les volumes réels.

Réserves de la revue stricte :
- services/api/src/trends.rs:151 et packages/shared/src/api.ts (TrendPoint.population) : un patch observé sans ligne du champion renvoie population = 0. Or le champ est documenté comme les participations de tous les champions du même patch, rôle et rang. La vraie population de ce patch n'est pas nulle : elle n'est simplement pas chargée, parce que la lecture SQL filtre sur champion_id. Un client affichera « 0 sur 0 ». Correction attendue : passer population en Option<u64> / number | null sur les points vides (miroir dans @olc/shared dans le même diff), ou au minimum documenter que 0 veut dire « non chargé ».
- ban_rate et delta_ban_rate sont calculés au niveau patch × plateforme × file (BanStats.scope n'a ni rôle ni rang). Dans une série rank=GOLD ou role=JUNGLE, l'écart de ban est donc celui de tous les rangs et de tous les rôles. Ce n'est dit ni dans services/api/README.md (section Tendances entre patchs) ni dans la doc TS de TrendPoint. Correction : une phrase dans chacun des deux.
- services/api/tests/stats.rs (une_serie_ne_parcourt_pas_les_morceaux_des_autres_champions_ni_les_builds) : les deux jsonpath sont recopiés en dur au lieu d'être pris dans stats.rs. Si load_selection change de filtre, l'assertion de plan GIN validera une autre requête que celle de l'API. Correction : exposer population et scope en pub const dans stats.rs et les lier dans le test.
- La lecture d'une série charge les morceaux de bans de tous les champions et de tous les patchs du périmètre, puisque le filtre scope ne porte pas sur le champion. Le résumé de l'auteur dit pourtant que seuls les morceaux utiles sont lus. Le coût reste faible, mais l'affirmation va trop loin ; le test EXPLAIN ne couvre que les morceaux de groupes voisins.
- services/api/src/trends.rs:29-30 : le commentaire dit « nul si l'un des deux winrates est nul », alors que la règle vaut pour les trois écarts (win, pick, ban). À reformuler.
- TDD partiel, déclaré par l'auteur : les tests d'intégration PostgreSQL et les assertions de route (JWT, patch refusé) ont été écrits après le code. Seule la logique pure series() a eu un rouge constaté. Ce n'est pas bloquant selon rules/revue.md, puisque le comportement est testé.
- Recette : j'ai exécuté en lecture seule la requête SQL partagée avec patch nul sur olc_nuit_d (TR1, file 1750, rôle UNKNOWN, rang ALL, champion 53, docker exec psql). Elle renvoie 1 couverture et 1 groupe en stockage v2, win_rate nul en Arena. Mais l'instantané de recette ne contient qu'un patch (16.19), sur TR1 et les files 1740/1750 : tous les écarts y sont nuls. Le cas à plusieurs patchs n'est vérifié que sur des fixtures.
- Limites assumées et documentées dans le README : la série ne couvre que les patchs de l'instantané courant, par défaut 2. Il n'y a pas d'historique durable. Le pick_rate est comparé entre patchs sans correction de population. Restent hors périmètre : filtre de période, fenêtre glissante de 7 jours, valeurs avant/après dans le diff de catalogue, ticket « changements de patch ». Le diff de catalogue et le ticket n'apparaissent que dans le DoD, pas dans le README : à reporter dans le ticket #110 comme suite.

### PR #159 (ticket #80)

Questions ouvertes :
- La nouvelle requête est environ 25 % plus lente (cache chaud) sur la base de recette où chaque clé n'a quasiment qu'une observation ; le gain n'est démontré que sur un jeu synthétique à historique long. Valider qu'on la garde malgré tout (le coût ne dépend plus de la profondeur d'historique), ou attendre une mesure sur une base à longs historiques.
- Le ticket #80 n'a affiché aucun commentaire via gh ; le périmètre repose sur les consignes de la tâche (PR #136) et non sur le texte intégral du ticket.
- Les chiffres de la PR #136 (59,17 % → 24,07 %, 2 418 → 25 303, 316 s, 3,47 Go) sont repris tels que fournis, sans rejeu.

Réserves de la revue stricte :
- Performance : sur la base de recette olc_nuit_d (187 030 observations pour 185 938 clés, au plus 2 par clé, chiffres que j'ai revérifiés en lecture seule), la nouvelle forme est environ 25 % plus lente à cache chaud. Le gain n'est démontré que sur un jeu synthétique. La réécriture reste justifiée : les observations sont insérées sans upsert (services/collector/src/storage.rs:779, avec un cache de 24 h), donc l'historique par clé grossit avec le temps. L'auteur le documente honnêtement ; à remesurer sur une base à historiques longs.
- services/api/tests/stats.rs:497 : `assert_eq!(coverage[key], Value::Null)` passe aussi si la clé est ABSENTE, car l'index serde_json renvoie Null pour une clé manquante. Le test ne verrouille donc pas que l'API sérialise un null explicite, alors que le miroir TS est `number | null` (requis) dans packages/shared/src/api.ts:131-135. Correction suggérée : `assert_eq!(coverage.as_object().unwrap().get(key), Some(&Value::Null))`. Aujourd'hui la sérialisation est correcte : Option sans skip_serializing_if, dans services/collector/src/aggregation/model.rs:153-160.
- services/collector/tests/aggregation.rs:615-624 (rank_of) : `Vec::dedup()` ne retire que les doublons consécutifs. L'assertion `ranks.len() <= 1` dépend donc de l'ordre des groupes. Elle passe aujourd'hui, mais elle est fragile. Mieux : collecter dans un BTreeSet.
- TDD : seul le test de la borne ceil a un rouge (rouge démontrable : round(172800,4) = 172800 retiendrait DIAMOND). Les tests de caractérisation de la requête et de relecture API sont verts d'emblée, ce que l'auteur annonce. J'ai prouvé l'équivalence de façon indépendante : la comparaison ancienne/nouvelle requête, rejouée en lecture seule sur olc_nuit_d, donne 174 080 participations, 165 600 avec observation et 0 id différent. Le cas game_start NULL est impossible (matches.game_start NOT NULL, observed_at NOT NULL).
- CHANGELOG.md : seconde ligne #80 sous Corrigé, longue et orientée contributeur (index, recette, relecture d'instantané). Elle pourrait être fusionnée avec la ligne #80 existante ou raccourcie à l'effet visible (borne exacte).
- docs/recettes/2026-10-04-rang-fige.md : les plans EXPLAIN bruts et le script du jeu synthétique ne sont pas consignés, ce qui limite la reproductibilité de la mesure 250 ms → 3,5 ms. Les chiffres de la PR #136 sont repris sans rejeu, comme le demande le périmètre.
- CI Windows/macOS à confirmer après push. Aucun code système dans le diff, donc pas de branche OS attendue.

### PR #162 (ticket #103)

Questions ouvertes :
- La mutation indiquée par la revue (`freshness(&report.coverage)` seul) ne peut pas passer au rouge : la requête SQL filtre déjà la couverture par patch/plateforme/file (paramètre $3) avant la lecture Rust, donc `report.coverage` est déjà borné au périmètre. Le test est rouge uniquement avec la mutation combinée (SQL non filtré + `&report.coverage`). Faut-il supprimer le filtre Rust redondant (hors périmètre du ticket, hérité) ou le conserver comme défense en profondeur ?

Réserves de la revue stricte :
- Le ticket #103 n'est que partiellement traité. La PR ne doit pas porter « « Closes » du ticket 103 » mais « Réf. #103 ». Le CHANGELOG (CHANGELOG.md:9) et la ligne « hors périmètre » du DoD citent l'affichage desktop, l'alerte d'obsolescence et la dernière collecte par plateforme. Ils oublient d'autres recommandations du ticket : published_at modifié seulement si les entrées changent (empreinte des sources), parties par jour, seuils « données insuffisantes » / « obsolètes », collecte par plateforme planifiée en parallèle. Il faut les ajouter à la liste « restent à livrer » ou les reporter dans un commentaire du ticket.
- Âge des rangs : `meta.freshness` ne porte que `computed_at` et les bornes de parties. L'âge médian et maximal des observations de rang reste servi par couverture (`rank_gap_*_hours`, hérité de #80), sans agrégat dans `freshness`. L'auteur le signale dans ses Décisions, donc ce n'est pas bloquant. Mais un consommateur qui lit plusieurs périmètres (tendances, patch=None) doit agréger lui-même ces écarts. Interpréter « âge des rangs » comme l'écart partie → observation, plutôt que l'âge par rapport au calcul, doit être annoncé explicitement dans la PR.
- La lecture multi-patchs (`patch=None`, route trends via `load_selection`) n'est couverte que par le test unitaire `la_fraicheur_borne_les_parties_de_tous_les_perimetres_lus` (services/api/src/stats.rs:295). Aucun test d'intégration de l'API ne vérifie `meta.freshness` sur plusieurs patchs réels.
- La couverture est filtrée deux fois, en SQL (`$3` dans services/api/src/sql/stats_snapshot.sql, branches storage_version 1 et 2) puis en Rust (services/api/src/stats.rs:223-231). C'est une redondance héritée que l'auteur signale. J'ai vérifié le SQL : la mutation simple `&report.coverage` est bien équivalente au code actuel. En revanche, la double mutation rouge annoncée n'a pas pu être rejouée, car toute modification de fichier m'était interdite. Je l'ai seulement validée par raisonnement : avec la fixture voisine 1 / 9_999_999, le résultat diffère de 1_000_000 / 1_900_000.
- Le test d'instantané antérieur (services/api/tests/stats.rs, ~l.520) vérifie `freshness.last_game_start_ms == null` mais pas `first_game_start_ms`. C'est mineur.
- services/api/README.md:91 contient une ligne non rewrappée (« ...jusqu'au prochain calcul. Les builds incluent les étapes »). Pur détail de forme.
- `(extract(epoch FROM m.game_start)*1000)::bigint` (services/collector/src/aggregation/storage.rs:78) arrondit les microsecondes au lieu de les tronquer. Sans effet pratique, à garder en tête si une comparaison exacte avec les bornes de fenêtre ($5/$6) était un jour ajoutée.

### PR #166 (ticket #119)

Questions ouvertes :
- Redditions et parties de moins de 15 min hors remake : non comptées ici (le ticket demande de les compter avant de décider de leur inclusion). Premier point de donnée sur la base de recette, EUW1/420 depuis le 2026-10-01 : 5 117 parties, 50 remakes, 93 parties de moins de 15 min dont 43 hors remake. Décision d'inclusion ou d'exclusion à prendre par Matthieu, puis éventuel compteur de couverture dans un ticket séparé.
- Entrée du §10.3 du cahier des charges (côté et ordre de pick comme entrées du modèle Draft) : non rédigée, c'est une décision produit sur le modèle de victoire (#40, #42).
- Côté limité au rang ALL, comme le ticket le recommande : l'étendre à tous les rangs est une condition dans l'agrégateur, au prix d'effectifs plus faibles.
- Lecture de « conditionnel aux premiers objectifs » : la couverture publie à la fois le winrate de l'équipe qui prend l'objectif en premier (tous côtés) et celui du côté bleu quand il le prend (le rouge se déduit par différence). Confirmer que cela suffit pour #40 et #42.
- Le winrate après un premier objectif est une corrélation (l'équipe qui le prend était souvent déjà en avance) : à rendre visible dans l'interface au moment de l'affichage desktop, non livré ici.
- Migration 0014 vs 0010 à 0013 : si une autre lane ajoute aussi une section à la contrainte `champion_stats_snapshot_chunks_section_check`, la dernière migration appliquée doit lister toutes les sections ; à vérifier à l'intégration des lanes.
- La base de recette olc_nuit_d porte la migration 0014 sans 0010 à 0013 et un snapshot de test de 117 parties : à reconstruire ou recalculer avant toute autre recette sur cette base.

Réserves de la revue stricte :
- Course sur la contrainte CHECK des sections : services/collector/migrations/0014_snapshot_splits.sql redéfinit champion_stats_snapshot_chunks_section_check (DROP puis ADD). sqlx-core 0.8.6 (Cargo.lock) applique sans erreur une version absente inférieure à la dernière déjà appliquée : rien ne signalera donc qu'une des migrations 0010 à 0013, appliquée après 0014 (c'est le cas sur olc_nuit_d), redéfinit cette contrainte sans 'splits'. Au mieux l'ADD CONSTRAINT échouera sur les morceaux 'splits' existants, au pire le prochain calcul sera refusé. À faire avant fusion : comparer les quatre migrations sur ce nom de contrainte et s'assurer que la dernière appliquée liste les sept sections, ou reprendre 0014 après les autres lanes.
- TDD partiel, reconnu par l'auteur : les tests unitaires de services/collector/src/aggregation/tests.rs et context_tests.rs n'ont jamais été vus en rouge. rules/workflow.md (phase B) rend le TDD obligatoire pour le calcul et le mapping. Ce n'est pas bloquant, parce que ces tests vérifient de vrais comportements : bornes exactes 1199/1200/2399/2400, durée nulle, comptes exacts par côté et par objectif, données contradictoires. Le rouge a été observé pour l'intégration du collecteur et pour l'API.
- Les tranches de durée dépendent de l'issue bleu/rouge (model.rs, `if blue_won.is_some()` autour de l'ajout des splits). Une partie à deux camps dont le drapeau `win` est corrompu ou identique dans les deux camps n'a ni côté ni tranche de durée, alors qu'elle reste dans `included_matches` et dans les groupes. Le README dit seulement que Swarm et Arena n'ont pas de ligne, pas ce cas. À documenter : la somme des tranches peut être inférieure aux parties du groupe.
- Le doc-comment de Coverage.blue_side_matches dans model.rs (« équipes 100 et 200, hors Arena ») et son miroir dans packages/shared/src/api.ts (« hors Arena ») oublient la coop contre l'IA, pourtant exclue par le code (is_coop). À corriger : « hors Arena et coop contre l'IA ».
- Dans services/api/tests/stats.rs, le test builds_publie_les_tranches_de_duree_et_les_cotes_du_champion_en_v1_comme_en_v2 vérifie à la fin `tiers.get("splits").is_none()`. C'est vrai par construction (TierlistResponse n'a pas de champ splits), donc la vérification ne prouve pas ce que dit le commentaire : la tierlist ne charge pas les morceaux 'splits'. Ce point est en partie couvert par l'ajout de 'splits' dans une_lecture_de_build_ne_parcourt_pas_les_morceaux_des_autres_population. Supprimer cette vérification ou la remplacer par une vraie.
- Des éléments du ticket ne sont pas livrés, mais ils sont déclarés hors périmètre dans le DoD et le CHANGELOG : comptage des redditions et des parties de moins de 15 min, entrées côté et ordre de pick au §10.3 du cahier, affichage desktop. La PR ne doit pas porter « « Closes » du ticket 119 » sans ticket de suite, sinon ces éléments seront perdus : utiliser « Refs #119 » ou ouvrir les tickets correspondants.
- Volume de l'instantané : la liste splits compte jusqu'à six lignes par groupe pour tous les rangs, plus deux lignes de côté par groupe ALL. Ce n'est pas bloquant (découpage en morceaux, index GIN des populations), mais il faut surveiller la taille et la durée de publication sur une vraie fenêtre de données.
- La contrainte snapshot_metadata_only reste inchangée : c'est accepté. La tête v2 (struct Metadata de snapshot.rs) ne porte jamais 'splits', et un ancien écrivain v1 reste refusé à cause de 'coverage'.
- olc_nuit_d vérifié par moi-même, en lecture seule, avec docker exec psql : migrations appliquées 1 à 9 et 14 ; contrainte de section avec les sept sections ; instantané storage_version 2, included_matches 117, min_games 30 ; 4 morceaux 'splits'. psql n'est pas installé sur l'hôte ; la vérification est passée par le conteneur olc-ticket18-rehearsal.

### PR #169 (ticket #113)

Questions ouvertes :
- Le plafond de 2 000 item_events par groupe est-il la bonne valeur à la recette (il est exposé via max_item_events pour que le desktop s'adapte) ?
- La note de recette chiffrée (avant/après sur olc_nuit_d) n'a pas été rejouée : aucune agrégation n'a été relancée sur la base de recette, conformément au cadre.

Réserves de la revue stricte :
- packages/shared/src/api.ts:104 : `omitted_variants?: number | null` est optionnel alors que le `BuildStats` Rust (services/collector/src/aggregation/model.rs:118) sérialise toujours le champ (nombre ou null). Le `?` vient de la réutilisation de `BuildStats` par `BuildReport.builds` (packages/shared/src/builds.ts), dont le miroir Rust `crates/build-client::BuildVariant` ne porte pas encore le champ. Le compromis est documenté et ne casse pas le protocole, mais le miroir est plus large que l'exact. Dans le ticket de suivi desktop (Louison), soit ajouter `omitted_variants` à `BuildVariant` et retirer le `?`, soit séparer le type desktop.
- services/collector/src/aggregation/model.rs:726-736 : un second passage sur tous les `builds`, avec deux clonages de `(GroupKey, String)` par variante (insertion dans `totals`, puis lecture `totals[...]`). Comme `builds` est déjà trié par `(key, category)` (l.716), un comptage par plages consécutives éviterait la map supplémentaire et les clonages. À reprendre si les instantanés grossissent ; non bloquant.
- services/api/src/stats.rs:211-225 (`cap_item_events`) : le tri sur le seul `events` peut retirer entièrement les types d'événements rares (vente, annulation) d'un champion très joué. Il fausserait aussi tout repère de timing dérivé plus tard de ces lignes (piste du ticket). Le README annonce bien que « les plus fréquentes sont gardées », mais il faut documenter ce biais là où les repères seront calculés, ou envisager un plafond par type d'événement.
- services/api/src/stats.rs:211-225 : la liste n'est retriée par (événement, objet, minute) que si elle dépasse le plafond ; en dessous, l'ordre est celui du stockage. C'est cohérent en pratique (le collecteur émet dans l'ordre d'une BTreeMap), mais le commentaire « ordre naturel de lecture » n'est garanti que dans le cas tronqué.
- CHANGELOG.md:117 : une seule ligne réunit la correction du compteur, le détail par catégorie et le nouveau plafond des `item_events`. rules/changelog.md demande une ligne par changement, et le plafond relèverait plutôt de « Modifié » que de « Corrigé ». Il faudrait scinder en deux lignes.
- Hors périmètre, bien signalé : extension de `Page`/`BuildReport` côté client Rust et affichage desktop (Louison). La seule annonce est « Affichage desktop restant à livrer (#113) » dans le CHANGELOG ; le ticket #113 ne doit donc pas être fermé par ce livrable.

## Lane H — Agrégation, troisième file (empilée sur feat/111-filtres-parties, worktree de la lane F libérée)

| Ticket | PR | Lien | Base | Verdict de la revue stricte | Réserves | Questions |
|---|---|---|---|---|---|---|
| #86 | [#164](https://github.com/bolitow/open-lol-companion/pull/164) feat(collector): statistiques de runes par clé de voûte, arbre, rune et fragment (#86) | Refs | `feat/111-filtres-parties` | OK avec réserves | 7 | 2 |
| #87 | [#165](https://github.com/bolitow/open-lol-companion/pull/165) feat(collector): priorité de montée et trois premiers points des compétences (#87) | Refs | `feat/86-runes-par-cle` | OK avec réserves | 7 | 5 |
| #112 | [#168](https://github.com/bolitow/open-lol-companion/pull/168) feat(collector): intervalle de Wilson, écart au champion et tri par performance des variantes (#112) | Refs | `feat/87-priorite-competences` | OK avec réserves | 5 | 2 |

### PR #164 (ticket #86)

Questions ouvertes :
- Faut-il que l'API expose directement le taux conditionnel des runes d'emplacement (games / games de rune_keystone) plutôt que de laisser le calcul au consommateur ?
- Faut-il ajouter apps/web/next-env.d.ts et apps/web/tsconfig.tsbuildinfo au .gitignore pour éviter leur réapparition (hors périmètre de ce ticket) ?

Réserves de la revue stricte :
- (Traitée par l'orchestrateur : `assert_eq!(result.variants.len(), 11)` ajouté, test relancé au vert.) services/collector/src/aggregation/builds_tests.rs:104-108 : le test `une_categorie_incomplete_ne_contamine_pas_les_autres` est affaibli. Il passe de `len() == 1` à `all(|c| c == "runes" || c.starts_with("rune_"))`, qui passerait même si des catégories dérivées disparaissaient. Remplacer par `assert_eq!(result.variants.len(), 11)` (page exacte + 10 dérivées).
- Plafond `max_build_variants_per_category = 20` (model.rs:313) face au taux conditionnel documenté. `rune_slot_1..3` est indexé `[clé de voûte, rune]` : pour une clé peu jouée, ses lignes d'emplacement peuvent être coupées alors que la ligne `rune_keystone` reste (`rune_secondary_pair` peut aussi être tronquée). Le calcul « games / games de rune_keystone pour la clé » côté lecture n'est donc pas garanti complet. Documenter cette limite dans les README et dans le JSDoc de BuildRuneCategory, ou prévoir un plafond par clé de voûte dans un ticket de suivi.
- services/api/README.md:85-87 : la doc API annonce `rune_slot_1..3` « conditionnées à la clé de voûte » sans préciser que le `pick_rate` publié sur ces lignes est le taux conjoint (seuls le README collecteur et le JSDoc le disent). Un consommateur qui ne lit que la doc API peut se tromper. Ajouter une demi-phrase.
- Volumétrie mesurée par `aggregate --json` sur olc_nuit_a2 (build debug, sortie de 2,07 Go) : environ 875 000 lignes de builds `rune_*` ajoutées aux 1,52 million existantes (+57 % de lignes de builds) ; `omitted_build_variants` = 425 228. La réponse API par champion reste bornée par le filtre SQL (au plus 10 × 20 lignes de plus par groupe). La taille de l'instantané et le nombre de morceaux augmentent nettement, sans mention dans la section Vigilance. À signaler et à surveiller.
- builds.rs:85-89 : la branche `else { return BTreeMap::new() }` de `derive_rune_choices` est inatteignable en pratique, car `extract_runes` renvoie toujours exactement 11 identifiants. Défense inoffensive, mais le test `une_page_de_runes_incomplete...` passe par `extract_runes`, pas par cette branche.
- services/collector/src/aggregation/tests.rs:540 : la borne Wilson n'est vérifiée que par `is_some()`, pas par sa valeur. Le calcul est déjà couvert ailleurs pour les étapes, donc ce n'est pas bloquant.
- Hors périmètre déclaré (DoD : « aucune UI ») mais demandé par la recommandation du ticket : assemblage de la page par choix successifs, import de la page exacte la plus jouée pour la clé retenue, correction de LiveBuildSummary (clé de voûte de la première page). Ouvrir un ticket de suivi pour l'UI, pour que #86 ne soit pas fermé par ce livrable seul.

### PR #165 (ticket #87)

Questions ouvertes :
- skill_start : trois points (périmètre de la nuit) ou quatre comme dans la recommandation du ticket (3 à 4) ?
- skill_order_15 (séquence tronquée au niveau 15) de la recommandation du ticket n'est pas livré : à créer en ticket séparé ou à ajouter ?
- Biais résiduel : skill_priority n'existe que pour les parties où deux sorts de base atteignent le rang 5, donc plutôt des parties longues. Accepter ce compromis ou publier aussi une priorité approchée (par points investis) pour les parties courtes, au risque de la deviner ?
- Champions à mécanique particulière (rang maximal ou sorts spéciaux différents, par exemple Udyr, Jayce, Elise, Nidalee) : le rang 5 est supposé pour tous ; validation sur timelines réelles à faire avant d'afficher, et éventuelle exclusion par champion.
- Les consommateurs (build-client, BuildPreparation.tsx, rappel « sort à maxer » #27) restent à brancher dans un ticket client.

Réserves de la revue stricte :
- services/collector/src/aggregation/builds_tests.rs, test les_points_speciaux_ne_comptent_ni_dans_le_depart_ni_dans_la_priorite : le nom annonce que la priorité est vérifiée, mais la séquence ([1,2,EVOLVE 3,3]) est trop courte pour maximiser un sort et rien n'est vérifié sur skill_priority. Il faudrait une séquence où un EVOLVE ferait passer un sort au rang 5 s'il était compté, puis contrôler skill_priority, ou bien renommer le test. Ce n'est pas bloquant : par construction, les EVOLVE ne passent jamais dans `skills`.
- La publication dans l'instantané et dans l'API ne repose que sur le mécanisme générique (write_section "builds", l'API sert les builds sans filtrer par catégorie). Aucun test d'intégration PostgreSQL (services/collector/tests/aggregation.rs ou services/api/tests/stats.rs) ne vérifie skill_start ni skill_priority de bout en bout. C'est la même couverture que #86, donc acceptable, mais la preuve reste indirecte.
- La population de skill_priority est biaisée vers les parties où au moins deux sorts atteignent le rang 5 (en gros niveau 13 ou plus, donc les supports et les parties courtes sont sous-représentés). C'est documenté dans le README collecteur et dans le type partagé, mais pas mesuré sur les données réelles : l'auteur n'a fait aucune vérification sur olc_nuit_a2, et moi non plus.
- Le rang maximal 5 est supposé pour tous les sorts de base. Pour les champions à mécanique particulière (Udyr, Jayce, Elise, Nidalee, Aphelios…), la priorité publiée peut ne rien vouloir dire. C'est signalé comme à valider sur les timelines, mais aucun ticket de suivi n'est créé.
- Le ticket #87 reste plus large que ce livrable : skill_order_15, désérialisation de skill_levels dans build-client, affichage de la priorité, du départ et de special_skill_order. C'est déclaré hors périmètre dans la DoD, mais il faudra garder #87 ouvert ou créer des tickets de suivi pour ne pas le clore à tort.
- services/collector/README.md, bloc « Choix de montée (#87) » : la ligne qui finit par « validation sur timelines à faire. `skill_levels` expose… » dépasse largement la largeur de retour à la ligne du paragraphe. C'est cosmétique, d'autres lignes du fichier le font déjà.
- BuildSkillCategory (packages/shared/src/api.ts) n'est utilisé nulle part pour l'instant, comme BuildRuneCategory. C'est acceptable puisque category reste typé string côté Rust, mais l'interface ne bénéficie pas encore du typage.

### PR #168 (ticket #112)

Questions ouvertes :
- L'écart est calculé contre le winrate du groupe champion complet, pas contre la population de la catégorie (par exemple les parties à page de runes complète) : Matthieu valide-t-il ce choix pour l'affichage futur ?
- Le test du seuil a été renommé (lisibles_dans_un_ancien_instantane) plutôt qu'étendu à un cas Arena, car les deux tests Arena renforcés couvrent la publication : acceptable pour la revue ?

Réserves de la revue stricte :
- services/api/README.md:79-85 (et la ligne #112 de CHANGELOG.md:9) : `sort=performance` ne réordonne que les variantes déjà retenues par le plafond d'effectif du collecteur (`max_build_variants_per_category`, coupe faite dans services/collector/src/aggregation/model.rs après le tri par `games`). Une variante de niche plus performante, coupée à l'agrégation, reste invisible, alors que c'est l'impact cité par le ticket. Le DoD classe hors périmètre « autres parties du ticket #112 », mais sans le dire précisément. Ajouter une phrase explicite (le tri s'applique aux seules variantes retenues par le plafond d'effectif ; la fusion top N effectif plus top N borne basse recommandée par #112 reste ouverte), et garder #112 ouvert ou le découper.
- Branche : le worktree est sur `feat/87-priorite-competences` (HEAD 0f84669) et la branche `feat/112-variantes-performance` annoncée n'existe pas. Le diff non commité ne modifie pas encore la base, mais le commit doit se faire sur une nouvelle branche `feat/112-variantes-performance` créée depuis HEAD, jamais sur feat/87.
- services/collector/README.md:332 : coquille « mêmes bornage », à remplacer par « même bornage ».
- Preuve TDD du tour 2 (mutation temporaire de model.rs) : je ne l'ai pas reproduite puisque je ne dois modifier aucun fichier. Les assertions ajoutées (tuple à 5 champs à None, Null sur borne basse, borne haute et écart pour les items Arena) échoueraient bien si le masquage `performance_available` était retiré.
- `pnpm lint` ne vérifie les types que de 2 projets sur 3 (apps/web exclu). Sans effet ici : apps/web n'utilise ni BuildStats ni BuildsResponse, et next-env.d.ts et tsconfig.tsbuildinfo n'ont pas été recréés.

## Lane I — Agrégation, quatrième file (empilée sur feat/111-filtres-parties, worktree de la lane E libérée)

| Ticket | PR | Lien | Base | Verdict de la revue stricte | Réserves | Questions |
|---|---|---|---|---|---|---|
| #100 | [#160](https://github.com/bolitow/open-lol-companion/pull/160) feat(collector): moyennes de performance par patch, rôle, champion et rang (#100) | Refs | `feat/111-filtres-parties` | OK avec réserves | 8 | 5 |
| #123 | [#163](https://github.com/bolitow/open-lol-companion/pull/163) feat(collector): matchups de lane par rôle avec effectif et borne Wilson (#123) | Refs | `feat/100-moyennes-performance` | OK avec réserves | 7 | 6 |

### PR #160 (ticket #100)

Questions ouvertes :
- Aucune agrégation complète n'a été lancée sur olc_nuit_bc : la migration 0012 y serait appliquée, et cette base est partagée avec les autres lanes, dont le binaire sqlx refuserait alors de démarrer (migration inconnue). À rejouer sur une copie dédiée ou après fusion pour vérifier les volumes et la taille du snapshot sur données réelles.
- Choix de définition à valider : CS/min et or/min sont la somme rapportée à la somme des durées de partie (pondérée par la durée), pas la moyenne des taux par partie. Le KDA suit la formule du ticket sur les sommes.
- Arena est incluse dans les moyennes (K/D/A, dégâts, vision ; pas d'augments). La politique Riot n'interdit que les taux d'objets et d'augments en Arena : faut-il malgré tout l'exclure par prudence ?
- Les parties de moins de 15 minutes ne sont ni exclues ni marquées. Seuls les contrôles de qualité de #111 s'appliquent (durée minimale 300 s en Solo/Flex) ; elles ne contribuent simplement pas à la frame des 15 min. Le ticket proposait de les exclure ou de les marquer : à décider.
- Restent pour plus tard : écarts-types et p25/p50/p75, dégâts par type (physiques, magiques, bruts), kill participation, frames à 5 et 20 min, appariement à l'adversaire de voie (#123). Ils demandent des tickets ou une décision de format.

Réserves de la revue stricte :
- (Traitée par l'orchestrateur : migration renumérotée 0012.) Collision de numéro de migration, non signalée par l'auteur. `services/collector/migrations/0010_snapshot_performance.sql` prend la version 10, mais les branches feat/90-budget-rangs-suite, feat/98-jeton-trousseau, feat/99-rgpd-retention et fix/122-quota-reserves-revue (aussi sur origin) portent déjà `0010_privacy_retention.sql`, et deux d'entre elles `0011_excluded_matches.sql`. main et feat/111 s'arrêtent à 0009, donc le fichier est correct sur sa base. En revanche, sqlx refuse deux versions identiques, ou signale une version déjà appliquée avec une autre somme de contrôle : la deuxième lane fusionnée devra renuméroter (0012 ou plus). C'est à arbitrer dans l'ordre de fusion.
- Arena : la section `performance` est alimentée pour toutes les clés de `groups`, scopes Arena compris (`arena_scopes` n'est pas consulté dans model.rs:486-489 ni dans performance.rs). Ce n'est pas une violation Riot (aucune donnée d'augment). Mais le comportement n'est ni testé ni documenté : `les_etapes_d_achat_arena_ne_publient_aucune_performance` ne couvre que les builds. À trancher (publier ou exclure), avec un test et une ligne dans le README du collecteur.
- Frames d'une participation incomplète : une participation dont les valeurs de fin de partie sont incomplètes (`end_of_game` = None) contribue quand même à `frames[]` (model.rs:489, performance.rs `add`). C'est cohérent avec « effectif propre à chaque minute », mais c'est en tension avec la phrase « une participation incomplète est écartée en bloc » du résumé, et aucun test ne le couvre. Il faut préciser le README ou ajouter un test.
- Vigilance (1) de l'auteur, confirmée : le drapeau `$4` de `services/api/src/sql/stats_snapshot.sql` est partagé. `/v1/performance` charge les morceaux builds, skill_levels et item_events du champion (les plus lourds) puis les ignore, et `/v1/builds` charge la ligne de performance. Un drapeau ou un paramètre de sections dédié est à prévoir.
- Vigilance (3) de l'auteur, confirmée : les frames ne sont lues que si `builds::extract_timeline` réussit (model.rs:438-445). Un événement d'objet malformé fait donc perdre aussi les valeurs à 10 et 15 min. C'est documenté, mais ce couplage n'est pas testé.
- Je n'ai pas pu reproduire le contrôle en lecture seule annoncé sur olc_nuit_bc (19 371 timelines à frameInterval 60 000 ms, 197 050 participants complets) : `psql` est absent et aucun pilote PostgreSQL Python n'est installé. Ces chiffres reposent sur la parole de l'auteur. Aucune agrégation n'a été lancée sur la base de recette (vigilance 4). Les « questions ouvertes » auxquelles l'auteur renvoie ne figurent pas dans le résumé transmis.
- Hors du périmètre fixé pour cette version, mais demandé par le ticket #100 (à suivre dans un ticket ou en V2) : dégâts par type, KP, challenges, écarts-types et p25/p50/p75, frames à 5 et 20 min, appariement à l'adversaire de voie, parties de moins de 15 min à exclure ou marquer (seule la frame à 15 min les écarte aujourd'hui).
- Tests d'intégration du collecteur faibles sur les valeurs : tests/aggregation.rs vérifie seulement qu'un morceau `performance` existe et que `performance_method` n'est pas vide. Les valeurs sont couvertes par les unitaires et par les tests de l'API (égalité JSON complète).

### PR #163 (ticket #123)

Questions ouvertes :
- Rang de partie (#80) : cette version ne publie que le rang ALL. Quelle définition retenir pour un rang de partie (médiane des dix tiers connus, tier majoritaire, seuil de couverture minimal), afin de publier des matchups par palier en gardant les deux sens complémentaires ?
- Synergies de duo (BOTTOM+UTILITY d'abord, puis JUNGLE+MIDDLE et TOP+JUNGLE) : non livrées. Il faut décider de la métrique publiée (gain par rapport aux deux winrates isolés, ou winrate brut avec effectif), du seuil propre aux paires, et de la politique Arena avant toute publication de duos.
- Appariement en file 400 (normale draft) : la validation n'y impose pas l'unicité (équipe, rôle), donc aucun matchup n'est publié hors 420/440. Faut-il valider l'appariement en 400 par un contrôle par partie (exactement un participant par équipe et par rôle) puis l'ouvrir ?
- Volume : avec environ 660 parties EUW1 par patch et aucun groupe champion au-dessus de 80 parties, presque toutes les lignes auront winrate et borne Wilson nuls au seuil actuel. Faut-il regrouper toutes les plateformes, publier des paliers cumulés (Émeraude+, etc.) ou une fenêtre multi-patch pour les matchups, ce qui changerait la clé de population de l'API ?
- Écart à l'attendu et rétrécissement : la recommandation du ticket (publier l'écart au winrate du champion, rétrécir vers ce winrate) n'est pas implémentée. Il faut valider la formule (par exemple bêta-binomiale avec a priori sur le winrate du champion au même rôle) et son nom public.
- Base de recette : la migration 0013 n'a pas été appliquée sur olc_nuit_bc et aucune agrégation réelle n'y a été lancée, pour ne pas casser les binaires des autres lanes qui partagent la base sans 0013. À faire après l'intégration des lanes, pour mesurer la couverture réelle et le volume de la section `matchups`.

Réserves de la revue stricte :
- Migration 0013 et coordination avec l'autre lane (services/collector/migrations/0013_snapshot_matchups.sql) : le risque n'est pas l'ordre d'application par sqlx, qui applique de toute façon les versions manquantes. Le vrai risque : 0010/0011 et 0013 font chacune DROP puis ADD des contraintes `champion_stats_snapshot_chunks_section_check` et `snapshot_metadata_only`, avec une liste de sections qui ignore celles de l'autre lane. Si 0010 ou 0011 changent cette liste, celle des deux qui s'applique en dernier retire les sections de l'autre et casse la publication. Il faut prévoir une migration de réconciliation à la fusion, quel que soit l'ordre. Ce n'est pas un défaut du livrable, car le cadre imposait de repartir de 0012.
- Fermeture du ticket : #123 couvre aussi les synergies de duo, le rang de partie, le pooling multi-plateformes et paliers, l'écart à l'attendu avec rétrécissement et la validation de l'appariement en file 400. La PR et le commit doivent citer #123 sans `« Closes » du ticket 123`. L'écart à l'attendu et le rétrécissement ne figurent pas dans les questions ouvertes annoncées (seulement synergies, rang de partie, pooling) : il faut les y ajouter. La file 400 n'est citée que dans le README.
- E/S inutiles (services/api/src/sql/stats_snapshot.sql:39-47) : le drapeau `$4` est commun à tous les endpoints champion. Du coup, `/v1/builds` et `/v1/performance` lisent aussi les morceaux `matchups` du champion, et `/v1/matchups` lit les morceaux builds, skill_levels, item_events et performance. Le résultat est correct mais la lecture est superflue. Choisir les sections par endpoint serait un ticket de suivi.
- Rang différent de ALL (services/api/src/stats.rs, fonction `matchups`) : la route renvoie une liste vide sans signal explicite. Seuls le README et le texte de `matchup_method` l'expliquent. Le client ne distingue pas « aucune donnée » de « non publié à ce rang ». Un champ explicite, par exemple `rank_scope`, ou un refus pourrait suivre.
- Aucune mesure de volume sur données réelles : aucune agrégation sur `olc_nuit_bc` et migration 0013 non appliquée. L'auteur l'a signalé et ne pas toucher à la base partagée était le bon choix. Le nombre de lignes au-dessus du seuil et le poids des morceaux `matchups` restent à mesurer en recette.
- Preuve TDD (cycles rouges) : non vérifiable par le relecteur, qui ne peut attester que l'état vert. Les tests ciblent bien le comportement : appariement dans les deux sens, exclusion des files non classées et des rôles inconnus ou en double, complémentarité des victoires, seuil sans taux, couverture, absence d'identifiant, isolation par population, pagination, morceaux v2 et instantané antérieur.
- Test d'intégration services/collector/tests/aggregation.rs:154-160 : il vérifie seulement que la section `matchups` existe et n'est pas vide. L'aller-retour complet passe par la comparaison du rapport publié, ce qui suffit. Une assertion sur un couple précis (effectif et victoires complémentaires après publication) renforcerait le test.

## Lane J — Agrégation, cinquième file (empilée sur feat/109-ban-rate-par-palier, worktree de la lane I libérée) : plancher de fiabilité détaché de la lane A

| Ticket | PR | Lien | Base | Verdict de la revue stricte | Réserves | Questions |
|---|---|---|---|---|---|---|
| #91 | [#167](https://github.com/bolitow/open-lol-companion/pull/167) feat(collector): plancher de fiabilité et intervalles de Wilson (#91) | Refs | `feat/109-ban-rate-par-palier` | OK avec réserves | 8 | 2 |
| #83 | [#170](https://github.com/bolitow/open-lol-companion/pull/170) feat(collector): paliers de rang cumulés (X et plus) à l'agrégation et dans l'API (#83) | Refs | `feat/91-plancher-fiabilite` | OK avec réserves | 7 | 6 |

### PR #167 (ticket #91)

Questions ouvertes :
- Le seuil du plancher de fiabilité est fixé à 30 parties et la fiabilité est binaire (low / sufficient) : Matthieu valide-t-il ces valeurs, ou faut-il un niveau intermédiaire ?
- Affichage de la fiabilité et des intervalles dans le desktop : volontairement hors périmètre de ce livrable, à traiter dans un ticket d'interface séparé.

Réserves de la revue stricte :
- services/collector/README.md (~l.248-260) : le paragraphe « Plancher de fiabilité (#91) » a été inséré entre « Sous le seuil, taux champion/build et classement sont nuls… » et « Le taux de ban demande au moins ce nombre de drafts complètes. ». Ce « ce nombre » suit désormais une phrase qui se termine sur `reliability_floor` à 0, donc on ne sait plus à quoi il renvoie. Il faut remonter cette phrase juste après « …les comptes restent visibles. » ou nommer `min_games` explicitement.
- La base de `reliability` change selon l'entité : `games` pour un champion ou une variante (le dénominateur du winrate), `draft_matches` pour un ban (le dénominateur du ban rate). Pour le pick rate d'un champion, `games` n'est pas le dénominateur (`bucket_matches`). Exemple : un champion à 10 participations sur 5 000 parties est marqué `low` alors que l'intervalle de son pick rate est très étroit. C'est documenté, mais `reliability` décrit en pratique le winrate. Il faut le préciser dans la doc, ou prévoir une fiabilité propre au pick rate dans un ticket de suite.
- Les intervalles sont masqués avec le taux quand l'effectif est sous `min_games`. Seul le plancher (`reliability`) est indépendant de `min_games`. Le périmètre dit « intervalles … indépendants du seuil min_games », ce qui peut se lire des deux façons. Le choix est assumé dans le CHANGELOG et les README, donc rien n'est caché, mais le produit doit confirmer cette lecture.
- packages/shared/src/builds.ts (`BuildReport.meta`) et crates/build-client/src/lib.rs (`BuildMeta`) ne portent pas `reliability_floor`. Les deux miroirs sont alignés, mais le desktop reçoit `low` sans connaître le seuil qui l'explique, alors que le commentaire de `Reliability` dans @olc/shared renvoie à `meta.reliability_floor`. À relayer quand l'affichage côté client sera fait.
- Points du ticket #91 hors du périmètre annoncé et non traités : alerte côté client indépendante de `meta.min_games` (`lowSample` dans autoImport.ts), refus ou marquage « test » d'un instantané de production sous le plancher (le README dit explicitement que cela relève du client et du produit), seuils par catégorie, effectif du meilleur candidat dans le message d'import vide. Le livrable ne doit donc pas fermer #91 : la PR doit le citer comme partiel (« Refs #91 ») et ouvrir ou garder une suite.
- crates/build-client/src/lib.rs : `check()` vérifie que chaque borne reste dans 0..=100, mais pas que la borne basse est inférieure ou égale à la borne haute. Durcissement mineur possible.
- CHANGELOG.md : l'entrée #91 tient sur une seule ligne très longue qui regroupe environ six faits. Elle respecte « une ligne = un changement », mais elle se lit mal. À raccourcir pour un joueur ou un contributeur.
- services/api/README.md : le nouveau passage est mal coupé (une ligne très longue au milieu d'un paragraphe à environ 90 colonnes). Purement cosmétique.

### PR #170 (ticket #83)

Questions ouvertes :
- Faut-il garder IRON_PLUS (tous les paliers connus, sans UNKNOWN ni UNRANKED, donc différent de ALL) ? La recommandation du ticket liste SILVER+ à MASTER+, son constat IRON_PLUS à MASTER_PLUS : j'ai retenu 8 clés. À arbitrer.
- GRANDMASTER_PLUS et CHALLENGER_PLUS ne sont pas publiés (effectif trop faible, MASTER_PLUS couvre les trois paliers apex) : à confirmer.
- Défaut du desktop : rank=EMERALD_PLUS comme défaut du desktop et menu des paliers (BuildPreparation.tsx, ChampionProfile.tsx) sont des décisions produit et d'interface FR/EN, non faites ici ; la validation côté client de builds est prête.
- Volume du rapport : chaque participation au palier Emerald alimente 8 populations au lieu de 2. Le run `aggregate` sur olc_nuit_bc en profil debug a dépassé 10 minutes et a été interrompu : la taille du snapshot et le temps de calcul sont à mesurer avec `cargo run --release` avant mise en production.
- Hors de cette livraison : population platform=ALL (arbitrage de l'exigence §9 « pas de mélange de régions ») et fenêtre multi-patch pondérée.
- La liste des rangs est dupliquée entre collecteur/API (CUMULATIVE_RANKS en Rust), le client de builds (crates/build-client, sans dépendance au collecteur) et packages/shared : aucun test croisé automatique entre Rust et TypeScript. À faire si on veut un garde-fou.

Réserves de la revue stricte :
- Volume et mémoire non mesurés sur données réelles (l'auteur le signale, je n'ai pas pu le mesurer non plus : psql est absent et je n'ai pas lancé de build release). Ordre de grandeur : une participation Émeraude tombe dans 8 populations au lieu de 2. Les sections groups, builds, skill_levels et surtout item_events (sans plafond de variantes) peuvent donc grossir d'environ 4 à 5 fois sur les files classées. Dans finish(), cumulative::extend (services/collector/src/aggregation/cumulative.rs:37-56) construit en plus une BTreeMap parallèle avec des clés clonées avant append. Le cahier §10.2 demande un recalcul horaire. Avant la fusion, lancer `cargo run --release -p olc-collector -- aggregate` sur olc_nuit_bc et relever la durée, le pic mémoire et la taille par section (avant et après).
- services/collector/src/aggregation/model.rs:86 (doc de GroupKey) et :147-149 (doc de BanStats.rank) : les commentaires de contrat côté Rust n'ont pas été mis à jour, alors que le miroir @olc/shared (packages/shared/src/api.ts) l'a été. Aucun champ sérialisé n'a changé, mais les deux côtés du contrat documentent désormais des ensembles de valeurs différents. Ajouter la mention des paliers cumulés (#83) côté Rust.
- services/collector/src/main.rs:570-575 : la ligne de synthèse de `aggregate` (« N groupes dont M classés ») compte maintenant les groupes cumulés, ce qui gonfle fortement le chiffre et trompe l'opérateur. Distinguer les groupes cumulés ou le préciser dans le message.
- La liste des paliers cumulés est recopiée en dur dans crates/build-client/src/lib.rs:57-65 et dans son test, sans test croisé avec CUMULATIVE_RANKS du collecteur (l'API, elle, réutilise la constante). Le constat est déjà noté par l'auteur. Un test croisé ou une constante partagée éviterait que les listes divergent.
- AggregationReport ne déclare pas qu'il publie des paliers cumulés (aucune métadonnée comme rank_scope ou ban_rank_basis). Un instantané antérieur à #83 et un palier sans données renvoient tous deux une liste vide, sans qu'on puisse les distinguer. Le comportement est documenté dans les README. Une métadonnée serait souhaitable, sans être bloquante.
- Le ticket #83 reste ouvert sur trois points : le défaut du desktop sur un palier cumulé, platform=ALL et la fenêtre multi-patch. La PR doit référencer `Refs #83`, pas `« Closes » du ticket 83`. IRON_PLUS et BRONZE_PLUS ont été ajoutés en plus des valeurs recommandées (Silver+ à Master+) : le choix est assumé et documenté.
- TDD partiel, comme déclaré : le test d'intégration API (PostgreSQL) et le test de constante de packages/shared ont été écrits après l'implémentation. La validation de requête qu'ils couvrent avait déjà eu un passage rouge dans query.rs. Le test les_paliers_cumules_sont_classes_comme_les_autres_populations ne vérifie pas l'attribution du tier S/A/B/C/D (il faut au moins 5 champions éligibles).

## Lane K — Agrégation, sixième file (empilée sur feat/109-ban-rate-par-palier, worktree de la lane H libérée) : population honnête détachée de la lane A

| Ticket | PR | Lien | Base | Verdict de la revue stricte | Réserves | Questions |
|---|---|---|---|---|---|---|
| #82 | [#172](https://github.com/bolitow/open-lol-companion/pull/172) feat(collector): répartition des paliers et étiquette honnête de la population ALL (#82) | Refs | `feat/109-ban-rate-par-palier` | OK avec réserves | 7 | 6 |

### PR #172 (ticket #82)

Questions ouvertes :
- Repondération de ALL : faut-il une post-stratification (poids = part du palier dans le ladder / part dans l'échantillon) ? Elle exige une source de taille du ladder par palier et par plateforme, que nous ne collectons pas aujourd'hui.
- Collecte : faut-il des quotas de seeds proportionnels à la taille des paliers, un tirage sur toutes les pages league-v4 et un plafond de parties par seed et par champion ? C'est une décision produit et une question de budget de clé Riot.
- Indicateur de biais : faut-il un signal dérivé de la répartition (par exemple part de Master+ au-dessus d'un seuil) ? Je ne l'ai pas codé faute de seuil validé. Sur la copie de recette, Master+ représente environ 92 % des participations classées EUW1 16.19 Solo.
- Desktop (Louison) : renommer ALL en « Échantillon collecté » / « Collected sample » à partir de `population_label`, afficher `tier_participations`, et choisir si ALL reste le défaut ou s'il est remplacé par un palier cumulé (#83).
- Bans : la partie sans palier calculable (`UNKNOWN`) partage l'étiquette `unknown_rank` avec les joueurs sans rang observé. Faut-il une étiquette dédiée (`unknown_match_tier`) ?
- Granularité : la répartition est publiée par périmètre (patch, plateforme, file), tous rôles confondus. Faut-il aussi la ventiler par rôle ?

Réserves de la revue stricte :
- services/api/src/stats.rs:287 et services/api/src/stats.rs:107 : sur `/v1/tierlist` avec un palier (ex. GOLD), `meta.population_label` vaut `observed_tier`. Pourtant, le tableau `bans` renvoyé dans la même réponse suit le palier de la partie (`match_tier`, #109). L'étiquette décrit les entrées de la page, pas toute la charge utile. Il faudrait le préciser dans services/api/README.md (ou dans le commentaire TS de `SnapshotMeta.population_label`) pour que l'interface ne l'applique pas aux bans de la tierlist.
- Pour les bans, `UNKNOWN` (partie sans palier calculable) reçoit la même étiquette `unknown_rank` que pour un joueur. L'auteur l'a signalé ; c'est acceptable puisque `ban_rank_basis` lève l'ambiguïté.
- `tier_participations` est publié par périmètre (patch, plateforme, file), tous rôles confondus. Il ne décrit donc pas la composition de `ALL` pour un rôle donné. L'auteur l'a signalé ; le documenter si l'interface affiche la répartition sur une page filtrée par rôle.
- Recette : la comparaison entre « ≈ 92 % de Master+ » (EUW1, 16.19, Solo seulement) et les 22,8 % du ticket (toutes plateformes et files, données du 1er octobre) n'est pas homogène. Elle ne doit pas servir à conclure que le biais s'est aggravé. Il faut aussi tracer que `aggregate` a publié un nouvel instantané dans la copie jetable `olc_nuit_a2`.
- La repondération et les quotas de seeds proportionnels ne sont tracés que dans le cahier des charges (« restent à décider ») et dans la ligne hors périmètre du DoD. Le dépôt n'a aucun artefact `open_questions` structuré, donc je ne peux pas vérifier leur report dans les questions ouvertes de la lane. L'orchestrateur doit s'assurer qu'elles y figurent.
- services/api/README.md (lignes 103 à 112) : le nouveau paragraphe « Population (#82) » s'insère au milieu de la phrase « Un instantané antérieur à #109 sert ses bans sous `ALL` seulement ». La phrase se retrouve coupée après « Un instantané », ce qui gêne la lecture (cosmétique).
- Le libellé desktop « Échantillon collecté » (recommandation « Immédiat » du ticket) n'est pas livré. Il est hors périmètre de cette lane (Louison), mais #82 ne peut pas être fermé avec ce seul livrable.

## Lane L — Agrégation, septième file (empilée sur feat/109-ban-rate-par-palier, worktree de la lane G libérée) : agrégation incrémentale détachée de la lane A

| Ticket | PR | Lien | Base | Verdict de la revue stricte | Réserves | Questions |
|---|---|---|---|---|---|---|
| #89 | [#173](https://github.com/bolitow/open-lol-companion/pull/173) feat(collector): agrégation incrémentale par lots, résultat identique au recalcul complet (#89) | Refs | `feat/109-ban-rate-par-palier` | OK avec réserves | 10 | 3 |

### PR #173 (ticket #89)

Questions ouvertes :
- Faut-il activer `--incremental` par défaut pour le recalcul horaire (`--watch`) ? Le cahier des charges indique que ce choix reste à décider.
- Empreinte du catalogue : j'ai pris un hachage du classement des objets plutôt que le `publication_id`. Une republication sans effet sur les étapes réutilise donc les lots. À valider si tu préfères relire le patch à chaque republication, même sans effet.
- Le lot dominant (16.19 EUW1 Solo, la moitié de la base) fixe le plafond de mémoire. Le découper, par rôle par exemple, demanderait de modifier l'accumulateur : faut-il un ticket séparé ?

Réserves de la revue stricte :
- Périmètre « mémoire bornée des variantes de builds » : atteint seulement par le découpage en lots. Le plus gros lot (16.19/EUW1/420, la moitié de la base de recette) reste le plafond de mémoire : 1,39 Gio pour un seul lot modifié. L'auteur le signale (recette « Lecture », cahier « reste à décider », réserves du DoD), donc ce n'est pas bloquant. En revanche, ce livrable ne doit PAS fermer #89 : pas de « « Closes » du ticket 89 » dans la PR. Il faut une suite pour borner la mémoire dans un lot (découpe par rôle, ou plafond de variantes pendant l'accumulation) et le ticket de dimensionnement que le ticket recommande.
- Granularité incrémentale : un lot entier est relu dès qu'il reçoit une partie ou une observation de rang proche. En --watch, les lots classés du patch courant sont donc relus à chaque heure (112 s pour le lot dominant, contre 298 s en recalcul complet). Le gain porte sur les anciens patches et les lots inactifs. C'est documenté (README, recette), mais ce n'est pas un traitement des seules « parties nouvelles ».
- Chemins non couverts par un test : (a) dans previous_lots (services/collector/src/aggregation/incremental.rs:262-264), un écart entre `chunks` et les morceaux réellement présents doit forcer le recalcul ; (b) le repli de binary_identity (incremental.rs:159-163, identité propre au processus). Logique simple, mais jamais exercée.
- binary_identity (incremental.rs:142-165) repose sur chemin + taille + date de modification. Un redéploiement qui garde ces trois valeurs (build reproductible au même chemin, date figée, même taille) réutiliserait en silence des lots calculés par l'ancien code. Le risque est rare. Une constante de compilation (version + empreinte git via build.rs) serait plus sûre.
- Plafond d'échelle de l'empreinte (incremental.rs:186-187) : md5(string_agg(...)) concatène tous les identifiants d'un lot. Vers 25 M de parties dans un même lot, on atteint la limite de 1 Gio d'un texte PostgreSQL, et l'échec est franc. C'est très au-delà du run 18, mais à noter pour le dimensionnement.
- Point à surveiller au rebase : la garde de compilation (LotCounts::of, append, ItemCatalog::fingerprint) n'empêche pas une section d'une autre lane (tiers, fiabilité, splits…) de calculer une règle qui croise des périmètres (relative à d'autres patches ou plateformes). L'égalité avec le recalcul complet serait alors rompue sans erreur, sauf si le corpus de le_cumul_des_lots_reproduit_exactement_le_recalcul_complet exerce cette règle. Le README l'exige déjà ; l'intégrateur doit enrichir ce corpus à chaque section ajoutée.
- Migration 0018 : l'index matches_lot_idx est créé sans CONCURRENTLY (migration sqlx transactionnelle) et bloque les écritures de la collecte pendant sa construction sur une grosse base. C'est documenté ; il faut planifier l'application hors collecte.
- Recette (docs/recettes/2026-10-04-agregation-par-lots.md:31-34) : l'écart entre l'instantané préexistant et le recalcul complet (section groups et en-tête) n'est expliqué que par une hypothèse (#84), non vérifiée. Ce n'est pas lié au mode par lots, puisque complet et par lots sont identiques entre eux.
- Cosmétique : README services/collector/README.md (paragraphe « Un lot n'est relu que si… »), coupures de ligne irrégulières (« est\ninchangé ; les\nparamètres »).
- Windows : aucun code propre à un OS (std::env::current_exe et fs::metadata existent sur les deux OS), mais les tests n'ont tourné que sur macOS. La CI Windows doit être verte avant fusion.

## Ordre de fusion conseillé (branches empilées)

Chaque lane est une pile : fusionner dans l'ordre indiqué. La première PR d'une pile repose soit sur `main`, soit sur une branche d'une autre lane (indiquée).

- Lane A : #131 → #136 → #143 → #152 → #156 → #161 → #171 ; base de départ : `main`
- Lane D : #132 → #135 → #141 → #144 → #146 → #148 ; base de départ : `main`
- Lane E : #133 → #139 → #140 → #142 → #147 → #150 → #153 → #157 ; base de départ : `main`
- Lane F : #134 → #137 → #138 → #145 → #151 → #155 ; base de départ : `main`
- Lane G : #149 → #154 → #158 → #159 → #162 → #166 → #169 ; base de départ : `feat/81-builds-par-etapes` (PR #143)
- Lane H : #164 → #165 → #168 ; base de départ : `feat/111-filtres-parties` (PR #152)
- Lane I : #160 → #163 ; base de départ : `feat/111-filtres-parties` (PR #152)
- Lane J : #167 → #170 ; base de départ : `feat/109-ban-rate-par-palier` (PR #161)
- Lane K : #172 ; base de départ : `feat/109-ban-rate-par-palier` (PR #161)
- Lane L : #173 ; base de départ : `feat/109-ban-rate-par-palier` (PR #161)

## Tickets du lot #80–#130 non lancés cette nuit

16 ticket(s) ouvert(s) du lot n'ont pas de PR de cette nuit (assigné entre parenthèses).

- #92 (LouisonRaymond) : Préparation : dériver région, file et rôle de la draft au lieu d'EUW1 / Solo / Mid
- #94 (LouisonRaymond) : Profil : « Afficher plus de parties » du compte actif mène à une erreur
- #95 (LouisonRaymond) : Profil : signaler les remakes et définir la fenêtre et la file du winrate récent
- #101 (LouisonRaymond) : Bilan : capturer le bloc de fin de partie LCU et attendre la partie match-v5
- #105 (LouisonRaymond) : Préparation : ne pas proposer de filtre de rang en Normal Draft
- #106 (LouisonRaymond) : Profil : division des rangs Master, Grandmaster et Challenger mal affichée
- #108 (LouisonRaymond) : Draft : projeter la phase et l'ordre de pick (bans, picks, premier pick)
- #114 (LouisonRaymond) : Profil : filtrer l'historique par file, champion et période
- #115 (LouisonRaymond) : En partie : CS/min, ratio KDA et participation aux kills
- #117 (LouisonRaymond) : Collection : complétion, RP et disponibilité
- #120 (LouisonRaymond) : Desktop : rapports de plantage et télémétrie anonymes avec consentement
- #126 (LouisonRaymond) : Desktop : rotation gratuite et files ouvertes ou fermées
- #127 (LouisonRaymond) : Compte actif : un changement d'icône est pris pour un changement de compte
- #128 (LouisonRaymond) : Profil : « Joué récemment avec » pour son propre compte
- #129 (LouisonRaymond) : Profil : multi-recherche et historique de recherches
- #130 (LouisonRaymond) : Amis : statut en jeu, rang et dernière connexion

## Points d'attention avant les fusions

### Conflits attendus
- `CHANGELOG.md` : chaque PR ajoute ses lignes sous `[Non publié]`. Conflits triviaux mais systématiques entre lanes.
- Piles d'agrégation parallèles (G, H, I, J, K, L) toutes issues de la lane A (`feat/81` ou `feat/111` ou `feat/109`) : `services/collector/src/aggregation/model.rs`, `snapshot.rs`, `services/api/src/stats.rs`, `services/api/src/sql/stats_snapshot.sql`, `packages/shared/src/api.ts` et les README sont touchés par presque toutes. Fusionner lane par lane, en commençant par A, puis résoudre les autres piles l'une après l'autre.
- PR #167 (#91) et PR #168 (#112) ajoutent toutes deux `win_rate_upper_bound` aux variantes de build (`model.rs`, `api.ts`, `crates/build-client/src/lib.rs`) : garder une seule définition.
- PR #131 (#90) et PR #153 (90b) définissent chacune `RANKED_QUEUE_IDS` : doublon trivial.
- PR #161 (#109) publie les bans par palier de partie ; PR #170 (#83) cumule aussi les bans : vérifier la cohérence après fusion.
- #89 (agrégation incrémentale, lane L) touche le cœur de `model.rs` sur la base `feat/109` : il devra être rebasé après la fusion des autres piles d'agrégation.

### Migrations SQL (sqlx, `services/collector/migrations`)
- Numéros uniques attribués cette nuit : 0010 et 0011 (lane E, PR #150 et #157), 0012 (`performance`, PR #160), 0013 (`matchups`, PR #163), 0014 (`splits`, PR #166), 0018 (lots de l'agrégation incrémentale, PR #173). sqlx applique une version manquante même après des versions supérieures : l'ordre de fusion n'est pas bloquant.
- PR #173 (#89) : l'index `matches_lot_idx` est créé sans `CONCURRENTLY` (migration transactionnelle) et bloque les écritures de la collecte pendant sa construction sur une grosse base ; prévoir la migration hors collecte. Les tests n'ont tourné que sur macOS : attendre la CI Windows avant fusion.
- Trois migrations réécrivent la même contrainte `champion_stats_snapshot_chunks_section_check` : 0012, 0013 (repart de 0012) et 0014 (ignore 0012/0013). Après fusion des deux piles, la dernière migration appliquée doit lister toutes les sections : coverage, groups, bans, builds, skill_levels, item_events, performance, matchups, splits. Le plus simple : corriger 0014 pour inclure `performance` et `matchups` lors de la fusion de la pile G.

### Validation Riot à faire par Matthieu
- PR #155 (#39, modèle de draft) et PR #151 (#20, site web) : relire la section 2 du cahier des charges avant de sortir du brouillon.
- PR #154 (#104) : la note de conformité de la PR est à confirmer.
- Plusieurs PR publient des agrégats pour Arena (K/D/A, vision, matchups exclus) : la politique n'interdit que les taux d'objets et d'augments, mais la revue a demandé une décision explicite (PR #160, #166).

### Hors périmètre laissé à Louison
- Toutes les PR d'agrégation laissent l'affichage desktop hors périmètre : fraîcheur (#103), tranches de durée et côtés (#119), performance (#100), matchups (#123), fiabilité et intervalles (#91, #112), paliers cumulés (#83), variantes omises (#113), bans par palier (#109), runes et compétences (#86, #87).

### Environnement de la nuit
- Worktrees sous `/Users/bolito/dev/olc-nuit/lane-{a,d,e,f}` (chacun avec un `.env` local et un `.cargo/config.toml` hors index) : à supprimer après les fusions avec `git worktree remove`, ils portent chacun un dossier `target` de plusieurs Go.
- Bases de recette dans le conteneur Docker `olc-ticket18-rehearsal` (port 55418) : `olc_nuit_a1` (instantané republié par #109), `olc_nuit_a2` (agrégation de #82), `olc_nuit_d` (schéma 0014, instantané de #119), `olc_nuit_bc` (intacte, schéma 0009), `olc_nuit_l` (copie de a1 pour #89). Toutes jetables.
- Les processus lancés le 2 octobre (`olc-api serve` sur 3030 et `olc-collector aggregate --watch` sur `olc18`) et le `tauri dev` de 00 h 44 tournent sur les binaires d'avant la nuit : rebuild nécessaire pour voir les nouveautés.
- Rapport d'analyse `docs/analyses/2026-10-04-ecarts-stats-concurrents.md` toujours non commité dans le dépôt principal (branche `codex/collection-video-reliability`) : à committer ou à déplacer.
- Clé Riot de développement : uniquement dans `.env` du dépôt principal, jamais écrite ailleurs, aucune collecte Riot lancée cette nuit. Elle expire 24 h après sa création.


## Décisions de Matthieu sur les questions ouvertes (4 octobre, matin)

Les questions ouvertes des 43 PR ont été posées une par une côté joueur ; les réponses et les décisions techniques prises par défaut sont dans `docs/analyses/2026-10-04-decisions-questions-ouvertes.md`. À reporter dans chaque PR avant fusion.
