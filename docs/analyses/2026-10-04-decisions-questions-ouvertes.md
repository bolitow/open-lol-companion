# Réponses de Matthieu aux questions ouvertes (4 octobre 2026, matin)

Décisions prises via Ask User Question, formulées côté joueur. À reporter dans les PR / tickets concernés.

## Lot 1 — rang des parties et filtres

- **Fenêtre de rang d'une partie : 7 jours** (168 h). Au-delà, le joueur compte en « rang inconnu ». → PR #136 (#80) : garder 168 h.
- **Palier médian entre deux paliers : arrondi vers le bas.** On ne surclasse jamais une partie. → PR #161 (#109).
- **Filtre de rang par défaut : le palier du joueur** détecté depuis le client, repli sur Émeraude+ si inconnu. → PR #172 (#82) / #170 (#83) côté API : le défaut serveur reste Émeraude+ ; le choix « palier du joueur » est à faire côté desktop (Louison).
- **Paliers cumulés proposés : 8 choix, Fer+ à Master+.** Pas de Grand maître+ ni Challenger+. → PR #170 (#83).

## Lot 2 — tierlist, fiabilité, builds

- **Lettres de tierlist : seuils fixes** en points de winrate (S ≥ +2,5, A ≥ +1, B ≥ −1, C ≥ −2,5, D sinon). → PR #171 (#85) : garder les seuils absolus.
- **Fiabilité : 30 parties, 2 niveaux** (low / sufficient). → PR #167 (#91) : valeurs validées.
- **Écart d'une variante de build : par rapport au winrate du champion** (groupe complet). → PR #168 (#112) : choix validé.
- **Runes secondaires : taux conditionnel à la clé de voûte, calculé par le serveur.** → PR #164 (#86) : exposer le taux conditionnel dans l'API (suite à faire).

## Lot 3 — fiches champion (performance, Arena, tendances)

- **CS/min et or/min pondérés par la durée** (somme / somme des durées). → PR #160 (#100) : définition validée.
- **Arena : K/D/A, dégâts et vision publiés.** Pas d'exclusion par prudence. → PR #160 (#100), #166 (#119) : Arena reste dans les moyennes.
- **Arena : runes et sorts affichent le placement moyen**, comme les objets. → PR #154 (#104) : ticket de suite à ouvrir (variantes hors objets sur placement).
- **Tendances : patch à patch**, pas de fenêtre glissante de 7 jours. → PR #158 (#103).

## Lot 4 — matchups, côtés, compétences

- **Matchups : paliers cumulés, par région**, pas de mélange de régions. → PR #163 (#123) : suite à faire (rang de partie médian de #109 + paliers cumulés de #83), publication ALL seule en attendant.
- **Matchups : Solo et Flex seulement**, pas de Normale Draft. → PR #163 (#123) : comportement actuel validé.
- **Winrate par côté : tous rangs seulement.** → PR #166 (#119) : comportement actuel validé.
- **Ordre de compétences : 3 points de départ.** → PR #165 (#87) : validé ; `skill_order_15` en ticket séparé.

## Lot 5 — sélection des champions, fin de partie, import, patch

- **% de victoire estimé en sélection : afficher dès maintenant**, sans attendre la réponse de Riot. → PR #155 (#39) : activer l'affichage (à faire côté desktop) ; garder le point dans la soumission Riot (#30) et prévoir de pouvoir le désactiver si Riot refuse.
- **Bilan de fin de partie : purge automatique, pas de bouton Effacer.** → PR #137 (#102) : règle validée, pas de commande Tauri de purge.
- **Sorts d'invocateur à l'import : préférence D/F du joueur conservée** pour Flash ; l'orientation majoritaire n'est qu'affichée. → PR #149 (#124) : comportement validé.
- **Numéro de patch affiché comme le jeu (26.19)**, conversion depuis 16.19 côté app. → PR #146 (#93) : libellé public = source d'affichage, numéro technique en interne.

## Lot 6 — collecte et coulisses

- **Files collectées par défaut : Solo et Flex seulement** (420/440). → PR #153 (90b) : `campaign_queues` par défaut = 420, 440 ; PR #132 (#97) : les cibles ARAM/Arena restent optionnelles.
- **Collecte équilibrée par palier** (seeds tirés de chaque palier, Fer à Challenger). → PR #172 (#82) : ticket collecte à ouvrir (quotas de seeds par palier, tirage sur toutes les pages league-v4, plafond par seed) ; indicateur de biais à afficher en attendant.
- **Quota Riot : 20 % réservés à l'interactif**, 80 % collecte. → PR #133 (#122) : valeurs validées, y compris sur les fenêtres par méthode.
- **Jeton de l'app : enregistrement automatique par installation**, jeton long par appareil. → PR #150 (#98) : ticket serveur à ouvrir (endpoint d'enregistrement + rafraîchissement) ; `OLC_API_URL`/`OLC_API_TOKEN` réservés à `pnpm dev`.

## Lot 7 — données personnelles, site, recalcul

- **Conservation : 30 jours** (PUUID, Riot ID, rangs observés) **/ 90 jours** (parties brutes). → PR #147 (#99) : valeurs par défaut validées.
- **Liste d'opposition : oui**, registre haché des effacements pour bloquer la recollecte. → PR #147 (#99) : suite à faire.
- **Site web : laissé de côté pour le moment.** → PR #151 (#20) : aucune priorité serveur pour le site ; la PR reste en brouillon jusqu'à nouvel ordre.
- **Agrégation incrémentale activée par défaut dans `--watch`.** → PR #173 (#89) : à activer avant fusion.

## Lot 8 — en partie et sélection

- **Infos des adversaires (objets, sorts, niveau) : ne jamais afficher** dans le panneau en partie. → PR #138 (#30) : passer ces lignes de « à soumettre » à « refus » ; PR #137 (#102) : ne rien projeter côté adversaires.
- **Écart d'or estimé entre équipes : afficher l'estimation** (valeur des objets visibles). → PR #137/#138 : à implémenter côté bilan/panneau ; reste listé dans la soumission Riot (#30). Point d'attention : l'estimation lit les objets adverses, que Matthieu ne veut pas afficher individuellement — seule la somme agrégée par équipe sera montrée.
- **Suggestions de bans en sélection : oui, ticket à ouvrir** (pires matchups + bans populaires du palier). → PR #161 (#109), #163 (#123).
- **Parties de moins de 15 min (hors remake) : exclues des moyennes de performance.** → PR #160 (#100) : ajouter l'exclusion (durée < 900 s) avant fusion ou en suite immédiate.

## Décisions techniques prises par Fable (pas de choix joueur, à contester si besoin)

- #131 (#90) : reste de #90 (ordre détails avant rangs, cache négatif, campagne queue 0) en PR de suite ; unification de `rank_for` avec `RANKED_QUEUE_IDS` dans un ticket de nettoyage (aussi #153) ; jobs `participant_rank` en attente fermés via la maintenance de #153, en simulation d'abord.
- #132 (#97) : 1740/1750 traités comme Arena dans le catalogue des files ; 710 et 3130 restent exclues ; cibles 1000 parties / 20 000 appels gardées, à revoir avec une clé de production.
- #133 (#122) : libellé « Riot occupé » (riot_busy) à ajouter côté desktop (Louison) ; mesure de contention FOR UPDATE reportée ; pas de test de câblage.
- #134 (#79) : Matthieu relit l'ancre et le texte exact de la politique Riot dans un navigateur avant de sortir du brouillon.
- #135 (#116) : régénération du catalogue desktop à la prochaine publication ; correspondance des modes CommunityDragon en ticket.
- #136 (#80) : départage à écart égal = observation d'avant la partie ; table de rang à la collecte en ticket séparé ; part UNKNOWN et écarts à afficher (ticket desktop).
- #137 (#102) : noms d'événements et format de `KillerName` à vérifier sur capture réelle Windows et macOS ; liste blanche élargie soumise à Riot (#30).
- #139 (#125) : branche 401/403 conservée ; handshake temps réel en 401 reporté.
- #140 (#96) : copies de `ProfileRank` alignées dans la partie desktop (Louison) ; cache 5 min accepté.
- #141 (#107) : jeton `%i:…%` conservé et signalé ; source cdragon Arena épinglée acceptée.
- #142 (#121) : issue automatique par patch acceptée comme mémoire ; premier déclenchement à constater après fusion.
- #143 (#81) : tolérance côté client (valeur négative infime ramenée à 0) ajoutée en petite suite pour ne pas rejeter les anciens instantanés.
- #144 (#118) : typage des autres balises Data Dragon en ticket ; statut `derived` conservé.
- #145 (#88) : un seul bloc pour l'instant, blocs multiples après #81 (déjà `Refs`).
- #146 (#93) : version LCU par WebSocket plus tard ; libellés FR/EN côté interface.
- #147 (#99) : `purge --watch` en processus supervisé séparé ; migration 0010 (index GIN) hors créneau de collecte ; exécution en pause > 30 jours perd ses travaux (accepté).
- #148 (#116b) : repli `all_items` dès un objet illisible conservé, journalisation suffisante.
- #149 (#124) : seconde moitié (case d'un sort déjà équipé) en ticket.
- #150 (#98) : keyring 3.6.3 conservé ; état « jeton refusé » à montrer dans les réglages (Louison).
- #152 (#111) : ordre `short_game` avant `afk` conservé ; `early_departure` gardé ; parties sans `wasAfk` comptées normalement.
- #153 (90b) : purge de `excluded_matches` alignée sur `raw_match_days` ; timelines avant détails conservé.
- #154 (#104) : recalcul d'agrégation après fusion.
- #155 (#39) : `role: UNKNOWN` conservé avec `role_share: null`.
- #156 (#84) : pick rate = parties distinctes où le champion apparaît / parties du compartiment ; masquage existant conservé ; `pick_rate_definition` à afficher (Louison).
- #157 (#90b) : attentes chronométrées non allongées.
- #158 (#103) : indicateur de trou entre patchs exposé ; tendances Arena et historisation durable en tickets.
- #159 (#80b) : requête conservée (coût indépendant de l'historique).
- #161 (#109) : minimum 6/10 reste une constante ; ancien instantané servi avec `rank=` renvoie zéro ban jusqu'au recalcul (accepté) ; libellé distinct `unknown_match_tier` pour les bans (aussi #172).
- #162 (#103) : aucune décision à prendre ; la mutation demandée par la revue n'est pas applicable (la requête SQL borne déjà la couverture au périmètre).
- #169 (#113) : plafond de 2 000 `item_events` par groupe conservé et exposé via `max_item_events` ; note de recette chiffrée à rejouer après fusion.
- #163 (#123) : synergies de duo et rétrécissement bêta-binomial en tickets.
- #164 (#86) : `next-env.d.ts` et `tsconfig.tsbuildinfo` ajoutés au `.gitignore` en suite.
- #165 (#87) : biais de `skill_priority` accepté ; champions à mécanique spéciale à valider sur timelines réelles avant affichage.
- #166 (#119) : redditions comptées normalement ; avertissement « corrélation » à afficher côté desktop (Louison).
- #170 (#83) : pas de population `platform=ALL` (cahier §9).
- #171 (#85) : constantes (200, 0,02, 0,5 %, 20) à calibrer sur la base de recette avant publication ; fixture conservée.
- #172 (#82) : badge « échantillon surtout Master+ » et `tier_participations` à afficher en attendant la collecte équilibrée.
- #173 (#89) : empreinte du catalogue par hachage conservée ; découpage du lot dominant en ticket.
- #158 (#103) : pas de repli sur le patch précédent sous le seuil ; badge « peu de données » comme partout (règle des 30 parties).
- #147 (#99) : `summonerLevel` retiré aussi à la pseudonymisation.
- #138 (#30) : mention légale dans le panneau, `prototype.html` exclu du build de production, filtre défensif des bans adverses, visibilité du CS adverse à confirmer en partie réelle : tous retenus.
- #172 (#82) : répartition des paliers publiée tous rôles confondus, pas de ventilation par rôle pour l'instant.
- #166 (#119) : la lecture « premier objectif : winrate de l'équipe qui le prend + côté bleu » suffit.
- #143 (#81) : la correction Wilson reste une ligne « Corrigé » séparée dans le CHANGELOG.

## Cohérence et points de vigilance

- **Arena** : les décisions du lot 3 (performance publiée, runes et sorts sur placement) ne s'appliquent que si Arena est ajoutée aux files collectées. Avec « Solo et Flex seulement » (lot 6), les fiches Arena restent vides par défaut.
- **Écart d'or estimé (lot 8)** : `rules/conformite-riot.md` le classe en cas limite qui exige une validation explicite dans le ticket avant fusion, et la PR #138 le classe « à soumettre à Riot ». La réponse de Matthieu vaut décision produit, mais rien ne l'implémente tant que la règle et le ticket #30 ne sont pas mis à jour.
- **% de victoire estimé en sélection (lot 5)** : ce n'est pas un cas interdit par la règle (données publiques, avant la partie), mais la PR #155 l'avait lié à la soumission Riot (#2/#30). À garder dans la liste soumise, avec la possibilité de le désactiver.
- **Infos adverses jamais affichées (lot 8) vs écart d'or affiché** : l'écart d'or se calcule à partir des objets adverses visibles au tableau des scores. Seule la somme par équipe sera montrée, jamais l'objet d'un adversaire.
