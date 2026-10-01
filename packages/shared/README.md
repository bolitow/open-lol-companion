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
