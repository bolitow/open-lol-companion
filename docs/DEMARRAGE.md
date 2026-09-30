# Démarrer le développement

Ce guide vous amène de zéro à l'app qui tourne sur votre machine, sous Windows ou macOS.

## 1. Prérequis

| Outil | Version | Installation |
| --- | --- | --- |
| Node.js | 20 ou plus (22 conseillé) | [nodejs.org](https://nodejs.org) |
| pnpm | 10 | `corepack enable` (fourni avec Node) |
| Rust | stable | [rustup.rs](https://rustup.rs) |
| League of Legends | à jour | Pour tester la connexion au client (facultatif au début) |

**Windows** : installez aussi les [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (charge de travail « Développement Desktop en C++ »). WebView2 est déjà présent sur Windows 10 et 11.

**macOS** : `xcode-select --install`.

Détail complet : [prérequis Tauri 2](https://v2.tauri.app/start/prerequisites/).

## 2. Installer et lancer

```bash
git clone https://github.com/bolitow/open-lol-companion.git
cd open-lol-companion
pnpm install
pnpm dev          # lance l'app desktop (Tauri) avec rechargement à chaud
```

Le premier lancement compile les dépendances Rust (quelques minutes), les suivants sont rapides.

Autres commandes :

| Commande | Effet |
| --- | --- |
| `pnpm dev:ui` | Interface seule dans le navigateur (http://localhost:1420), sans Rust |
| `pnpm test` | Tests TypeScript + tests Rust du connecteur LCU et du collecteur |
| `pnpm typecheck` | Vérification des types |
| `pnpm lint` | Typage + `cargo fmt --check` + `cargo clippy` (doit être à 0 avant une PR) |
| `pnpm format` | Formate le code Rust |
| `pnpm build:desktop` | Installeur de production pour votre OS |

## 3. Vérifier la connexion au client LoL

Lancez le client League of Legends puis l'app : l'écran d'accueil passe au vert (« Client League of Legends détecté ») en moins de 3 secondes.

Sans le jeu, vous pouvez simuler un client en pointant l'app vers un faux lockfile :

```bash
echo "LeagueClient:1234:50000:motdepasse:https" > /tmp/lockfile
OLC_LOL_LOCKFILE=/tmp/lockfile pnpm dev
```

(Sous PowerShell : `$env:OLC_LOL_LOCKFILE="C:\temp\lockfile"; pnpm dev`.)

## 3 bis. Collecteur Riot (backend, facultatif)

Le collecteur `services/collector` récupère des parties via l'API Riot et les stocke dans PostgreSQL. Il ne dépend ni de l'app ni du client LoL.

1. PostgreSQL : `docker compose -f services/collector/docker-compose.yml up -d` (ou une installation locale).
2. Copiez `services/collector/.env.example` en `.env` à la racine, renseignez `RIOT_API_KEY` (clé de développement du Developer Portal, valable 24 h) et `DATABASE_URL`.
3. `cargo run -p olc-collector --release -- run --target 50` pour un petit essai, puis `report <n°>`.

Tests PostgreSQL : définissez `OLC_TEST_DATABASE_URL` (ex. `postgres://postgres:postgres@localhost:5432/postgres`) ; sans elle, `pnpm test` les ignore. Détails : [`services/collector/README.md`](../services/collector/README.md).

## 4. Où coder quoi

| Dossier | Contenu | Langage |
| --- | --- | --- |
| `crates/lcu-connector` | Détection du client, identifiants, phases de jeu | Rust |
| `apps/desktop/src-tauri` | Cœur de l'app : commandes appelées par l'interface, overlays, capture | Rust |
| `apps/desktop/src` | Interface de l'app | React + TypeScript |
| `packages/shared` | Types et utilitaires partagés (phases, Data Dragon) | TypeScript |
| `services/collector` | Collecteur Riot API (parties, timelines) vers PostgreSQL | Rust |
| `apps/web`, `services/api` | Site et API (pas encore initialisés) | — |

Règle d'or : ce qui touche au système (fichiers, processus, réseau local, secrets) vit en Rust ; l'interface appelle des commandes Tauri (`invoke("…")`) et ne voit jamais de mot de passe.

## 5. Première contribution

0. Lisez [`AGENTS.md`](../AGENTS.md) : workflow, Definition of Done et règles bloquantes.
1. Prenez un ticket du [sprint en cours](sprint-1.md) ou étiqueté `good first issue`.
2. Branche `feat/…` ou `fix/…`, puis pull request vers `main`.
3. La CI tourne sur la PR (pas au push ni après la fusion) : Linux à chaque fois, Windows et macOS quand l'app, le connecteur ou `@olc/shared` changent (et pas en brouillon). Elle doit être verte.

Et avant tout : relisez la section 2 du [cahier des charges](cahier-des-charges.md) sur la conformité Riot.
