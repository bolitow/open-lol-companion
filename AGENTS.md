# AGENTS.md — Règles de développement d'Open LoL Companion

> **Règle absolue** : une tâche d'implémentation n'est **jamais** terminée tant que le
> bloc « Definition of Done » (§4) n'est pas recopié et rempli dans la **dernière**
> réponse à l'humain. Répondre « c'est fait » sans ce bloc = **violation** de ce fichier.

Ce fichier s'adresse aux assistants de code (Claude, Codex, Cursor, Copilot…) **et** aux
contributeurs humains : ce sont les mêmes règles pour tout le monde.

## 0. Le projet en 30 secondes

- App compagnon League of Legends **gratuite et open source**, **Windows + macOS**.
- Monorepo pnpm (`apps/*`, `packages/*`) + workspace Cargo (`crates/*`, `apps/desktop/src-tauri`).
- App desktop : **Tauri 2** (cœur Rust) + **React 19 / TypeScript / Vite**.
- `crates/lcu-connector` : connexion au client LoL. `packages/shared` : types partagés.
- Spec : [`docs/cahier-des-charges.md`](docs/cahier-des-charges.md) · Démarrage :
  [`docs/DEMARRAGE.md`](docs/DEMARRAGE.md) · Sprint : [`docs/sprint-1.md`](docs/sprint-1.md).

### Règles non négociables (bloquantes en revue)

1. **Conformité Riot** (section 2 du cahier des charges) : n'afficher que ce qui est
   visible dans le client de jeu ; aucune automatisation de décision ; aucune injection
   dans le processus du jeu ; aucune publicité.
2. **Secrets** : jamais de clé API Riot ni de mot de passe du client LoL dans le code,
   les logs, l'interface, les issues ou les PR. Le mot de passe LCU ne quitte pas le
   cœur Rust.
3. **Deux OS** : tout ce qui touche au système est pensé et testé pour Windows **et**
   macOS (`cfg!(target_os = …)`, chemins, commandes).
4. **Frontière Rust / interface** : fichiers, processus, réseau local et secrets en Rust ;
   l'interface passe par des commandes Tauri (`invoke`).

## 1. Compatibilité (tous LLM)

- Fichier **autonome** : markdown pur, chemins relatifs, aucune syntaxe propre à un outil.
- `CLAUDE.md` ne fait que renvoyer ici : **ce fichier est la source unique**.
- Les capacités externes (skills, serveurs MCP) sont **optionnelles** : si l'une manque,
  appliquer le repli de [`rules/capacites.md`](rules/capacites.md), le signaler et
  **continuer**.
- **Concision** : pas de préambule ni de reformulation ; ne jamais réafficher de code
  inchangé ; résumer les sorties d'outils ; appliquer les règles sans les réexpliquer.
- Langues : **code et identifiants en anglais**, **commentaires, docs, commits et PR en
  français**, textes de l'interface en français **et** en anglais (i18n).

## 2. Classifier la demande (obligatoire, 1ʳᵉ réponse)

| Type             | Exemples                                           | Processus                                                      |
| ---------------- | -------------------------------------------------- | -------------------------------------------------------------- |
| `question`       | « Comment ça marche ? », « Pourquoi ? »            | Lire le code, répondre. **Pas de DoD.**                        |
| `implementation` | feature, fix, refactor, doc, config, CI            | Workflow §3 (phases A/B/C) + **DoD obligatoire**               |
| `commit` / `pr`  | « commit », « push », « ouvre une PR »             | **Clôture du livrable en cours** : Phase C + DoD + Phase D (§3) |

En cas d'ambiguïté → demander. **Ne pas choisir `question` pour éviter le processus.**

Toute modification de fichier suivi par git relève de `implementation`, y compris une
correction d'une seule ligne.

`commit` / `pr` n'est **pas** une opération git isolée : c'est la validation et la
livraison du livrable en cours. Si la Phase C ou le DoD manquent → les compléter
**avant** toute commande `git commit` ou `git push`.

## 3. Workflow `implementation`

Détail complet (source unique) : [`rules/workflow.md`](rules/workflow.md).

- **Phase A — avant d'écrire** : lire la section du cahier des charges et le ticket
  concernés, vérifier l'existant, planifier.
- **Phase B — pendant** : TDD, diff chirurgical, types alignés Rust ↔ `@olc/shared`,
  versions de librairies vérifiées.
- **Phase C — avant la réponse finale (bloquant)** : `pnpm test` et `pnpm lint` au vert,
  auto-revue, revue stricte, doc et `CHANGELOG.md` à jour, bloc DoD.
- **Phase D — livraison Git** (si `commit` / `pr` demandé) : ré-exécuter la Phase C sur
  le diff, recopier le bloc DoD, puis seulement `git add` / `git commit` ; `git push` ou
  PR **après** un commit réussi. La CI (Linux, Windows, macOS) doit être verte avant fusion.

**Ne pas envoyer la réponse finale tant que la Phase C n'est pas terminée.**

## 4. Definition of Done — template obligatoire en fin de réponse

Recopier ce bloc **tel quel** pour toute tâche `implementation` **et** toute livraison
`commit` / `pr` :

```markdown
## Definition of Done

- [ ] Plan avant code (ticket : #…)
- [ ] TDD : test rouge puis vert
- [ ] Tests au vert (commande : pnpm test)
- [ ] Linters à 0 (commande : pnpm lint)
- [ ] Auto-revue + résumé ci-dessous
- [ ] Diff chirurgical (hors périmètre : …)
- [ ] Types Rust ↔ @olc/shared : N/A | alignés
- [ ] Windows et macOS : N/A | testés | code conditionnel vérifié, test manuel à faire sur …
- [ ] Conformité Riot : N/A | vérifiée (section 2 du cahier des charges)
- [ ] i18n FR/EN : N/A | toutes les chaînes traduites
- [ ] Doc : N/A | fichiers touchés : …
- [ ] CHANGELOG.md : N/A | mis à jour
- [ ] Revue stricte : verdict …
- [ ] Aucun secret

### Résumé de livraison
- Fichiers : …
- Décisions : …
- Vigilance : …
```

`N/A` **doit** être justifié en une phrase. Une case vide = tâche **non terminée**.
Allégements autorisés : [`rules/definition-of-done.md`](rules/definition-of-done.md).

## 5. Orchestration (sous-agents)

Source unique : [`rules/orchestration.md`](rules/orchestration.md). Déléguer l'exploration
large et les tâches indépendantes, jamais la modification chirurgicale ; tout livrable de
sous-agent est vérifié comme du code généré.

## 6. Références (lire au besoin)

Ne lire un fichier `rules/` que si la tâche touche son sujet — jamais par précaution.

| Sujet                                          | Fichier                                                    |
| ---------------------------------------------- | ---------------------------------------------------------- |
| Phases détaillées du workflow                  | [`rules/workflow.md`](rules/workflow.md)                   |
| Capacités externes et replis                   | [`rules/capacites.md`](rules/capacites.md)                 |
| Orchestration des sous-agents                  | [`rules/orchestration.md`](rules/orchestration.md)         |
| DoD par type de tâche                          | [`rules/definition-of-done.md`](rules/definition-of-done.md) |
| Architecture, Rust/Tauri, tests, sécurité      | [`rules/architecture.md`](rules/architecture.md)           |
| Conformité Riot (checklist détaillée)          | [`rules/conformite-riot.md`](rules/conformite-riot.md)     |
| UX/UI, overlays, i18n                          | [`rules/ux-ui.md`](rules/ux-ui.md)                         |
| Documentation, branches, commits, PR           | [`rules/documentation.md`](rules/documentation.md)         |
| Revue de code                                  | [`rules/revue.md`](rules/revue.md)                         |
| Format CHANGELOG                               | [`rules/changelog.md`](rules/changelog.md)                 |

**Commandes du projet** (toujours les préférer aux commandes manuelles) :

| Commande             | Effet                                                           |
| -------------------- | --------------------------------------------------------------- |
| `pnpm install`       | Dépendances JS                                                  |
| `pnpm dev`           | App desktop en développement                                    |
| `pnpm test`          | Tests TypeScript + `cargo test -p lcu-connector`                |
| `pnpm lint`          | Typage TS + `cargo fmt --check` + `cargo clippy -D warnings`    |
| `pnpm format`        | Formate le code Rust                                            |
| `pnpm build:desktop` | Installeur de production pour l'OS courant                      |

> Règle d'or : une tâche à la fois — la suivante ne commence que si la précédente est
> « Done ». Un « commit » dans un tour suivant **continue** le livrable en cours.

## 7. Anti-patterns (NEVER)

- NEVER considérer une implémentation terminée sans le bloc DoD §4
- NEVER traiter « commit », « push » ou « ouvre une PR » comme une tâche git isolée
- NEVER exécuter `git commit` ou `git push` avant d'avoir recopié le bloc DoD §4
- NEVER committer, pousser ou ouvrir une PR sans demande explicite de l'humain
- NEVER committer sans `CHANGELOG.md` à jour
- NEVER sauter la revue stricte « parce que le diff est petit »
- NEVER modifier un comportement sans test
- NEVER inventer une API, une méthode, un endpoint LCU ou une librairie non vérifiés
- NEVER afficher une information absente du client de jeu, automatiser une décision de jeu
  ou injecter du code dans le processus du jeu
- NEVER écrire, logger ou afficher une clé API Riot ou le mot de passe du client
- NEVER livrer du code système testé sur un seul OS sans le signaler
- NEVER copier du code, des visuels ou des textes d'un autre produit (DPM.LOL inclus)
