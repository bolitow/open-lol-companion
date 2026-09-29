# Open LoL Companion — consignes pour les assistants de code

App compagnon League of Legends gratuite et open source (Windows + macOS). Référence fonctionnelle : `docs/cahier-des-charges.md`. Démarrage : `docs/DEMARRAGE.md`. Sprint en cours : `docs/sprint-1.md`.

## Stack

- Monorepo pnpm (`apps/*`, `packages/*`) + workspace Cargo (`crates/*`, `apps/desktop/src-tauri`).
- App desktop : Tauri 2, cœur Rust, interface React 19 + TypeScript + Vite.
- `crates/lcu-connector` : connexion au client LoL (lockfile, identifiants, phases).
- `packages/shared` : types partagés ; tout type exposé par une commande Tauri a son miroir ici.

## Commandes

- `pnpm install`, `pnpm dev`, `pnpm test`, `pnpm typecheck`
- `cargo test -p lcu-connector`

## Règles impératives

- **Conformité Riot** (section 2 du cahier des charges) : n'afficher que des informations visibles dans le client de jeu ; aucune automatisation de décision ; aucune injection dans le processus du jeu ; aucune publicité.
- **Secrets** : jamais de clé API Riot ni de mot de passe du client dans le code, les logs ou l'interface. Le mot de passe LCU reste dans le cœur Rust (`Debug` le masque déjà).
- Tout ce qui touche au système (fichiers, processus, réseau local) est en Rust ; l'interface passe par des commandes Tauri.
- Toujours penser aux deux OS : chemins, séparateurs, commandes système (`cfg!(target_os = …)`).
- Textes de l'interface en français et en anglais (i18n), code et identifiants en anglais, commentaires et docs en français.
- Tests unitaires pour toute logique de parsing ou de calcul.
