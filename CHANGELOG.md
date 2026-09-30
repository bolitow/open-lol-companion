# Changelog

Toutes les évolutions notables du projet. Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versionnage [sémantique](https://semver.org/lang/fr/). Règles : [`rules/changelog.md`](rules/changelog.md).

## [Non publié]

### Ajouté

- Règles de développement communes aux contributeurs et aux assistants de code : `AGENTS.md` et dossier `rules/` (workflow, Definition of Done, conformité Riot, revue, documentation).
- Commandes `pnpm lint` (typage, `cargo fmt`, `cargo clippy`) et `pnpm format`, vérifiées en CI.
- Connecteur LCU : client HTTPS, WebSocket WAMP et reconnexion automatique au client League of Legends, avec les événements `connected`, `disconnected` et `phaseChanged` pour l'app (#7).

### Modifié

- `CLAUDE.md` renvoie désormais vers `AGENTS.md`.
- Code Rust formaté avec `cargo fmt`.

## [0.1.0] — 2026-09-29

### Ajouté

- Monorepo pnpm + Cargo, CI Linux / Windows / macOS avec recherche de secrets (#5).
- Squelette de l'app desktop Tauri 2 + React qui indique si le client League of Legends est détecté.
- Crate `lcu-connector` : lecture du lockfile sur Windows et macOS, secours par les arguments du processus, authentification, phases de jeu (#7).
- Paquet `@olc/shared` : types partagés et utilitaires Data Dragon.
- Cahier des charges, guide de démarrage et plan du sprint 1.
