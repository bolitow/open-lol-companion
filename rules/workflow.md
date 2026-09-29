# Workflow `implementation`

Source unique des phases citées dans [`AGENTS.md`](../AGENTS.md) §3.

## Phase A — avant d'écrire

1. Identifier le ticket GitHub (`#…`) et la section du [cahier des charges](../docs/cahier-des-charges.md) concernés. Pas de ticket pour une fonctionnalité → en demander un.
2. Lire le code existant du périmètre (et les tests) avant de proposer quoi que ce soit.
3. Vérifier la conformité Riot si la tâche touche au jeu : [`conformite-riot.md`](conformite-riot.md).
4. Planifier (capacité `planification`) : fichiers touchés, types partagés, tests à écrire, impact Windows/macOS. Plan court, affiché si la tâche dépasse ~3 fichiers.

## Phase B — pendant

- **TDD** : écrire le test qui échoue, le voir échouer, puis coder. Obligatoire pour toute logique de parsing, calcul, mapping, état.
- **Diff chirurgical** : ne toucher que le périmètre. Refactor opportuniste = ticket séparé.
- **Types alignés** : toute structure renvoyée par une commande Tauri (`#[derive(Serialize)]`) a son miroir exact dans `packages/shared` ; les deux sont modifiés dans le même diff.
- **Librairies** : vérifier la version et l'API réelle (capacité `docs-librairies`) avant d'utiliser une crate ou un paquet npm ; ne jamais supposer une API Tauri 1 dans un projet Tauri 2.
- **Endpoints LCU / Live Client** : ne jamais inventer un endpoint ; citer sa source (doc communautaire, capture du client) dans le commentaire ou la PR.
- **Deux OS** : tout code système passe par `cfg!(target_os = …)` ou `#[cfg(...)]`, avec les deux branches écrites.

## Phase C — avant la réponse finale (bloquant)

1. `pnpm test` au vert.
2. `pnpm lint` à 0 (typage TS, `cargo fmt --check`, `cargo clippy -D warnings`). Si une commande ne peut pas tourner localement (outil ou réseau manquant), le dire et s'appuyer sur la CI.
3. Auto-revue du diff complet (`git diff`) : code mort, logs de debug, TODO sans ticket.
4. Revue stricte (capacité `revue-stricte`) selon [`revue.md`](revue.md) ; verdict dans le DoD.
5. Doc à jour ([`documentation.md`](documentation.md)) et entrée `CHANGELOG.md` ([`changelog.md`](changelog.md)).
6. Bloc DoD recopié et rempli.

## Phase D — livraison Git (uniquement sur demande explicite)

1. Ré-exécuter la Phase C sur le diff final.
2. Recopier le bloc DoD dans la réponse.
3. `git add` ciblé (jamais `git add -A` sans relire `git status`), commit au format de [`documentation.md`](documentation.md).
4. `git push` puis PR vers `main` avec le modèle de PR ; lier le ticket (`Closes #…`).
5. Suivre la CI (Linux, Windows, macOS) ; corriger jusqu'au vert avant de déclarer la livraison faite.
