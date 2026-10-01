# Changelog

Toutes les évolutions notables du projet. Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versionnage [sémantique](https://semver.org/lang/fr/). Règles : [`rules/changelog.md`](rules/changelog.md).

## [Non publié]

### Ajouté

- Commande `import_runes` : validation selon le catalogue du client, secondaires de lignes distinctes et remplacement de la page réservée à l'app (#14).
- Commande `import_spells` : import en sélection des champions avec Flash sur D/F, sans modifier le skin ni ajouter Flash à un build qui ne le contient pas (#15).
- Référentiel de jeu FR/EN : objets et statistiques enrichies par CommunityDragon, champions/compétences, runes/fragments et catalogues ; sources archivées, couverture explicite, reconstruction hors ligne et API de recherche/diff par patch (#61, sous-ticket de #18).
- API interne Rust/Axum : tierlist et builds filtrés, profils par Riot ID actuel et historique paginé, accès JWT, notifications WebSocket et statiques FR/EN revalidables par CDN (#19).
- Quotas Riot PostgreSQL partagés entre l'API et le collecteur, y compris après annulation d'un appel ou réponse 429 (#19).

- Agrégats par patch, région, file, rôle et rang observé : winrate, pickrate, bans, tiers avec seuils, builds, objets, sorts, runes et chronologie des compétences ; publication atomique ponctuelle ou horaire (#18).
- Synchronisation Data Dragon FR/EN pour les patches récents : champions standard et Classic, compétences et catalogues, cache versionné réutilisable conservé en cas d'échec (#18).
- Collecte sur les 15 plateformes Riot et toutes les files accessibles, rangs Iron à Challenger, observations Solo/Flex des participants et campagne reprenable jusqu'à 24 heures (#18).
- Règles de développement communes aux contributeurs et aux assistants de code : `AGENTS.md` et dossier `rules/` (workflow, Definition of Done, conformité Riot, revue, documentation).
- Commandes `pnpm lint` (typage, `cargo fmt`, `cargo clippy`) et `pnpm format`, vérifiées en CI.
- Connecteur LCU : client HTTPS, WebSocket WAMP et reconnexion automatique au client League of Legends, avec les événements `connected`, `disconnected` et `phaseChanged` pour l'app (#7).
- Collecteur Riot API (`services/collector`, prototype) : joueurs de départ league-v4, parties Ranked Solo/Duo EUW et timelines match-v5 stockées dans PostgreSQL, gestionnaire de quotas Riot, arrêt et reprise sans doublon, bilan de collecte (#17).

### Modifié

- Documents de planification `docs/superpowers/` exclus du suivi Git (#18).
- CI allégée : elle ne tourne plus qu'à l'ouverture et à la mise à jour des PR (plus au push ni après fusion) ; builds Windows et macOS seulement si l'app, le connecteur ou `@olc/shared` changent, et pas en brouillon ; caches d'une PR supprimés à sa fermeture ; plus d'artefact gitleaks.
- `CLAUDE.md` renvoie désormais vers `AGENTS.md`.
- `pnpm test` et la CI lancent aussi les tests du collecteur (PostgreSQL 17 en CI Linux, compilation et tests sous Windows et macOS quand le collecteur change) (#17).
- Code Rust formaté avec `cargo fmt`.

### Corrigé

- Agrégats : publication atomique des grandes listes en morceaux bornés, pour dépasser la limite d’un objet JSONB unique tout en conservant le schéma JSON public (#18).

- Collecteur : une ouverture de transaction PostgreSQL annulée ne peut plus rendre au pool une connexion encore dans une transaction ; les connexions saines restent réutilisées (#18).
- Collecteur : un travail refusé par Riot (401/403) ne peut plus être réservé une seconde fois pendant la suspension ; ce refus prime sur les limites de durée ou de budget (#18).
- Collecteur : une exécution dont le dernier appel consomme exactement le budget est terminée normalement, au lieu d'exiger une reprise inutile (#17).

## [0.1.0] — 2026-09-29

### Ajouté

- Monorepo pnpm + Cargo, CI Linux / Windows / macOS avec recherche de secrets (#5).
- Squelette de l'app desktop Tauri 2 + React qui indique si le client League of Legends est détecté.
- Crate `lcu-connector` : lecture du lockfile sur Windows et macOS, secours par les arguments du processus, authentification, phases de jeu (#7).
- Paquet `@olc/shared` : types partagés et utilitaires Data Dragon.
- Cahier des charges, guide de démarrage et plan du sprint 1.
