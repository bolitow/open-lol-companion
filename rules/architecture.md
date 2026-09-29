# Architecture, tests et sécurité

## Découpage

| Couche | Emplacement | Responsabilité |
| --- | --- | --- |
| Crates Rust réutilisables | `crates/*` | Logique pure et testable, sans dépendance à Tauri (ex. `lcu-connector`) |
| Cœur de l'app | `apps/desktop/src-tauri` | Commandes Tauri fines qui appellent les crates ; fenêtres, overlays, capture |
| Interface | `apps/desktop/src` | Affichage et interactions ; aucun accès système direct |
| Types partagés | `packages/shared` | Miroir TS des structures Rust exposées, utilitaires purs |
| Backend | `services/*` | Seul endroit où vit la clé API Riot (variable d'environnement) |

Une commande Tauri reste mince : validation d'entrée, appel à une crate, conversion en type sérialisable. La logique va dans `crates/`.

## Rust

- Édition 2021, `cargo fmt`, `cargo clippy -D warnings`.
- Erreurs typées avec `thiserror` ; pas de `unwrap()`/`expect()` hors tests et `main`.
- Code dépendant de l'OS isolé derrière `cfg`, les deux branches écrites et testées si possible.
- Aucune opération bloquante longue sur le thread principal de Tauri : `async` ou thread dédié.

## TypeScript / React

- `strict` + `noUncheckedIndexedAccess` ; pas de `any` non justifié.
- Appels Rust uniquement via `invoke` typé avec les types de `@olc/shared`.
- Composants fonctionnels, état local d'abord ; store global (Zustand) seulement pour l'état partagé entre écrans.

## Tests

- Rust : tests unitaires dans le module (`#[cfg(test)]`), noms de tests en français descriptifs.
- TS : Vitest, fichiers `*.test.ts` à côté du code.
- Données du client LoL : utiliser des **fixtures** anonymisées (JSON réels du client, sans pseudo ni PUUID réels) dans `tests/fixtures/`.
- Tests dépendant du système (lockfile, processus) : via la variable `OLC_LOL_LOCKFILE` ou des fichiers temporaires, jamais le vrai client.

## Sécurité

- Mot de passe LCU : jamais sérialisé vers l'interface, jamais loggé (`Debug` masqué).
- Certificat auto-signé du client LoL : l'accepter **uniquement** pour `127.0.0.1` et le port du lockfile, jamais globalement.
- Pas de clé API Riot dans l'app : l'app appelle notre backend.
- CSP Tauri à resserrer avant la première release publique (ticket à ouvrir).
- `gitleaks` tourne en CI ; un secret commité = révocation immédiate (voir `SECURITY.md`).
