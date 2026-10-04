# packages/shared

Types et utilitaires partagés : phases du client LoL, URLs Data Dragon et contrats
JSON de l'API interne (#19, section 10.4 du [cahier des charges](../../docs/cahier-des-charges.md)).

`src/api.ts` décrit les réponses tierlist/builds, profils et historiques paginés,
le manifeste statique, les erreurs et les messages WebSocket. Les types statistiques
reprennent les objets sérialisés du collecteur #18 : dimensions aplaties, compteurs
de couverture et valeurs nulles conservés. Les horodatages `fetched_at` des profils
sont en secondes Unix ; `game_start_ms`, les bornes des filtres et les temps moyens
des compétences sont en millisecondes.

Le client WebSocket envoie `WsAuthenticate` ; le premier message serveur est
`Publication`, également utilisé pour les publications suivantes. Ces interfaces
décrivent les données attendues et ne valident pas les réponses à l'exécution.

Le cœur Rust du desktop est ce client WebSocket (#125). Il expose son état avec
`PublicationState` (commande `publication_state`, événement `PUBLICATION_STATE_EVENT`) :
l'interface relit ses builds quand `revision` change.

Les contrats Tauri et les erreurs traduites FR/EN des imports du client LoL sont
dans `src/imports.ts`. Le [contrat des imports](../../docs/imports-client.md)
décrit leurs préconditions et les recettes à exécuter sur macOS et Windows.

`ClientPatch` et `ClientPatchError` (`src/gameflow.ts`) décrivent la version du jeu
lue dans le client par la commande `client_patch` ([contrat](../../docs/patch-client.md)).

## Modèle de draft, première version (#39)

`src/draftModel.ts` expose `scoreDraft`, un calcul pur sur les entrées de tierlist
déjà publiées (une seule population patch/plateforme/file/rang, les autres sont
ignorées). Il ne collecte rien, n'appelle aucun service et ne choisit rien à la
place du joueur.

- **Score d'un candidat** : borne basse de Wilson à 95 % publiée pour le poste
  demandé (`win_rate_lower_bound`), sans recalcul ; `null` sous le seuil
  d'échantillon. Tous les candidats sont rendus, triés par score, effectif puis
  identifiant ; aucun n'est marqué comme « le » choix. Bans, champions adverses
  et champions des autres alliés sont exclus.
- **Rôle** : part des parties du champion jouées au poste, parmi les cinq postes
  connus de la même population ; `null` si le poste demandé est `UNKNOWN`.
- **Estimation du draft** : taux de victoire moyen publié de chaque camp, puis
  part de chaque camp dans la somme des deux moyennes. C'est une description de
  l'échantillon, pas une probabilité calibrée ; elle reste nulle si un camp n'a
  aucun champion au-dessus du seuil, et `eligible`/`champions` en donnent la
  couverture.
- **Postes** : un allié est compté à son poste visible ; un adversaire, ou un
  allié sans poste visible, avec son taux tous postes confondus. Aucun poste
  n'est attribué aux adversaires.
- **Non inclus** : matchups et synergies (`matchups_available` et
  `synergies_available` à `false`, aucun agrégat publié), maîtrise des joueurs.
  La chaîne `method` décrit la formule pour les contributeurs ; l'interface doit
  la traduire et non l'afficher telle quelle.
- **Entrées** : la tierlist API est paginée et filtrée par poste ; l'appelant
  fournit toutes les pages des cinq postes (et `UNKNOWN`) de la population, sinon
  la part du rôle et les taux tous postes confondus portent sur un sous-ensemble.
