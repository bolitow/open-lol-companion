# Estimation descriptive de la draft — #181 / #187

Le desktop affiche dans la sélection classée Solo/Duo (420) ou Flex (440) une estimation des deux camps, leur couverture `éligibles / champions visibles` et la population (patch, plateforme, file, rang). Les équipes suivent leur côté bleu/rouge. Ce résumé décrit l'échantillon publié, pas une probabilité calibrée de gagner ; matchups et synergies ne sont pas pris en compte.

La région vient du compte actif et la file de la draft réelle, pas d'un filtre de consultation. Le rang suit le filtre de préparation. Le patch est résolu par le mécanisme client/manifeste existant ; une ancienne version n'est pas utilisée silencieusement. Modes personnalisés, non classés et Arena : pas de calcul dans ce lot. Il faudra définir explicitement leur population avant extension.

## Données et durée de vie

- React invoque `community_draft_stats` avec `{patch, platform, queue, rank}`. Seule la webview principale en a la permission. Aucun champion choisi, nom, identifiant joueur ou secret ne part au service.
- Rust lit `GET /v1/tierlist` pour TOP, JUNGLE, MIDDLE, BOTTOM, UTILITY, UNKNOWN ; pages de 200, plafond 2 000 par rôle / 12 000 au total, 8 Mio par réponse, 60 s sur l'opération. Query retournée, population de chaque ligne, effectifs et taux sont contrôlés. Les doublons `(champion_id, role)` sont refusés ; des métadonnées ou totaux changeants annulent la lecture entière.
- Le front ne conserve qu'une population en mémoire. Les picks et bans recalculent `scoreDraft` localement ; aucun appel réseau par pick. Les vérifications de patch déclenchées ailleurs ne rechargent pas la tierlist ; une invalidation de publication ou un changement de population le fait.
- Alliés : champion visible et poste affiché dans le client. Adversaires : champion verrouillé uniquement, aucun poste attribué. Les deux listes de bans sont passées au modèle.
- Valeur nulle ou échantillon absent : « Données insuffisantes », jamais 50 % de remplacement. Erreur : message et rechargement, pas de statistiques périmées. Les réponses tardives après changement de population/désactivation sont ignorées.
- Le réglage `showDraftWinEstimate` est actif par défaut, enregistré dans `olc.app.preferences`, accessible via la recherche des paramètres et réversible. Désactivé : le composant n'est pas monté, donc aucun nouveau chargement/calcul/rendu. Un transport Rust déjà parti peut finir dans sa limite de temps, sans réaffichage. `DRAFT_ESTIMATE_ENABLED` est l'interrupteur produit distinct.

## Vérification et limites

Les tests couvrent la pagination six rôles, les doublons, le changement de publication, la population invalide, le seuil d'éligibilité, les réponses tardives, le réglage, les prépicks/verrouillages, les bans, les camps, la couverture et les textes FR/EN. Le composant réel a été inspecté en navigateur dans une fixture isolée à 960 × 600 : ordre des camps, couverture et avertissement lisibles, dimensions de page 960 × 600 sans débordement. Cette fixture utilise des chiffres explicitement fictifs et ne prouve pas une draft réelle. La recette Windows et une sélection classée réelle restent à réaliser.

Conformité : décision produit du 4 octobre 2026 consignée dans `rules/conformite-riot.md`, cas toujours soumis à Riot via #2/#30. Aucun affichage en partie ni dans l'overlay, aucune automatisation de décision. Validation explicite dans le ticket avant fusion.
