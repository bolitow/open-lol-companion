# Sprint 1 — Fondations (2 semaines)

**Objectif** : à la fin du sprint, l'app s'installe sur Windows et macOS, détecte le client League of Legends, suit la phase de jeu en direct et change d'écran toute seule ; le projet a son nom, sa clé Riot de développement et ses maquettes.

## Déjà fait (PR de lancement)

- Monorepo pnpm + Cargo, CI Linux / Windows / macOS, détection de secrets — #5
- Choix de Tauri 2 appliqué (squelette de l'app) — #3, à confirmer
- Connecteur LCU : lecture du lockfile Windows/macOS, secours par les arguments du processus, en-tête d'authentification, phases de jeu, tests — première partie de #7

## Au programme

| Ticket | Tâche | Profil | Taille |
| --- | --- | --- | --- |
| #7 | Connecteur LCU, suite : client HTTPS (certificat auto-signé de Riot), WebSocket WAMP, reconnexion automatique | Rust | L |
| #8 | Suivi de la phase de jeu via le WebSocket et navigation automatique entre écrans (`screenForPhase` existe déjà dans `@olc/shared`) | Rust + React | M |
| #21 | Mettre en place l'i18n FR/EN dans l'app (toutes les chaînes via des clés) | React | S |
| #4 | Maquettes : dashboard, champion select, page build | Design | M |
| #1 | Nom définitif du projet et disponibilité (INPI, domaine) | Produit | S |
| #2 | Compte Riot Developer Portal + clé de développement, puis demande de clé de production | Produit | S |
| #17 | Prototype du collecteur : récupérer 1 000 parties Ranked EUW avec la clé de dev, stockage PostgreSQL local | Backend | M |

Tailles : S ≤ 1 jour, M ≤ 3 jours, L ≤ 1 semaine.

## Définition de « terminé »

Le bloc « Definition of Done » de [`AGENTS.md`](../AGENTS.md) §4 est rempli dans chaque PR. En particulier :

- Le code est fusionné dans `main` par pull request, CI verte sur les trois OS.
- Testé à la main sur Windows **et** macOS (ou signalé dans la PR si un seul OS était disponible).
- Aucune chaîne de texte en dur dans l'interface une fois #21 fusionné.

## Sprint suivant (aperçu)

Vue draft en direct (#12), page build (#13) et imports runes / sorts / items (#14, #15, #16) : le premier vrai service rendu au joueur.
