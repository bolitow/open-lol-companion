# Changelog

Toutes les évolutions notables du projet. Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versionnage [sémantique](https://semver.org/lang/fr/). Règles : [`rules/changelog.md`](rules/changelog.md).

## [Non publié]

### Ajouté

- Commande `import_runes` : validation selon le catalogue du client, secondaires de lignes distinctes et remplacement de la page réservée à l'app (#14).
- Règles de développement communes aux contributeurs et aux assistants de code : `AGENTS.md` et dossier `rules/` (workflow, Definition of Done, conformité Riot, revue, documentation).
- Commandes `pnpm lint` (typage, `cargo fmt`, `cargo clippy`) et `pnpm format`, vérifiées en CI.
- Connecteur LCU : client HTTPS, WebSocket WAMP et reconnexion automatique au client League of Legends, avec les événements `connected`, `disconnected` et `phaseChanged` pour l'app (#7).
- Collecteur Riot API (`services/collector`, prototype) : joueurs de départ league-v4, parties Ranked Solo/Duo EUW et timelines match-v5 stockées dans PostgreSQL, gestionnaire de quotas Riot, arrêt et reprise sans doublon, bilan de collecte (#17).

### Modifié

- CI allégée : elle ne tourne plus qu'à l'ouverture et à la mise à jour des PR (plus au push ni après fusion) ; builds Windows et macOS seulement si l'app, le connecteur ou `@olc/shared` changent, et pas en brouillon ; caches d'une PR supprimés à sa fermeture ; plus d'artefact gitleaks.
- `CLAUDE.md` renvoie désormais vers `AGENTS.md`.
- `pnpm test` et la CI lancent aussi les tests du collecteur (PostgreSQL 17 en CI Linux, compilation et tests sous Windows et macOS quand le collecteur change) (#17).
- Code Rust formaté avec `cargo fmt`.

### Corrigé

- Collecteur : une exécution dont le dernier appel consomme exactement le budget est terminée normalement, au lieu d'exiger une reprise inutile (#17).

## [0.1.0] — 2026-09-29

### Ajouté

- Monorepo pnpm + Cargo, CI Linux / Windows / macOS avec recherche de secrets (#5).
- Squelette de l'app desktop Tauri 2 + React qui indique si le client League of Legends est détecté.
- Crate `lcu-connector` : lecture du lockfile sur Windows et macOS, secours par les arguments du processus, authentification, phases de jeu (#7).
- Paquet `@olc/shared` : types partagés et utilitaires Data Dragon.
- Cahier des charges, guide de démarrage et plan du sprint 1.
