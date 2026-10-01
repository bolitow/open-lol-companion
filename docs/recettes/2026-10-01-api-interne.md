# Recette API interne #19 — 1er octobre 2026

Service Rust/Axum de cette livraison, exécuté sur macOS 27.0 avec PostgreSQL 17.
Base du collecteur réel, instantané publié en morceaux après le correctif #18.
Le serveur de recette utilise un port local distinct et un JWT temporaire, puis
s’arrête proprement ; le serveur de développement de l’utilisateur reste indépendant.

## Contrôles exécutés

- Santé PostgreSQL : HTTP 200 ; route protégée sans JWT : HTTP 401.
- Tierlist EUW1 / 420 / 16.19 / BOTTOM : réponse sur les données réelles,
  limite d’une entrée respectée, pages suivantes avec champions distincts.
- Builds Jinx : résumé et variantes présents, lecture du snapshot en morceaux.
- Statiques françaises 16.19.1 : HTTP 200 puis 304 sans corps avec `If-None-Match`.
- WebSocket authentifié : publication réelle et disponibilité PostgreSQL reçues.
- Profil par Riot ID actuel : HTTP 200, Riot ID et niveau cohérents avec le compte
  connecté au client. Aucun identifiant ou résultat nominatif conservé ici.
- Historique officiel : HTTP 200, pagination de deux entrées au maximum.
- Arrêt du serveur de recette via SIGTERM : sortie normale.

Le compte a été résolu côté Rust depuis le client, version jeu 16.19.8230722.
Le mot de passe LCU n’a jamais quitté Rust. Les jetons et la clé Riot ne figurent
ni dans les commandes affichées, ni dans les journaux de preuve.
La page de profil utilise les identifiants retournés par l’API publique ; la
vérification de correspondance porte sur le Riot ID actuel et le niveau.

## Vérifications automatisées et limites

Lot isolé : `pnpm test` avec PostgreSQL réel, **208 tests Rust et 5 TypeScript** ;
`pnpm lint` et `git diff --check` réussis. Revue stricte indépendante : OK.
Les tests couvrent aussi erreurs Riot synthétiques, JWT invalides/expirés,
publication cohérente v1/v2, quotas partagés et limites de ressources.

Recette réelle effectuée sur macOS ; Windows reste à valider sur machine.
Ces essais ne valident pas un déploiement HTTPS/CDN, une charge de production,
la connexion utilisateur/RSO ni tous les modes de jeu. La campagne #18 conserve
son échéance initiale et son bilan reste distinct.
