# Revue de code (revue stricte)

Relire le diff complet comme un relecteur qui cherche à le refuser. Verdict : **OK**, **OK avec réserves** (listées) ou **À corriger** (bloquant).

## Bloquant

- Violation de la [checklist Riot](conformite-riot.md).
- Secret, mot de passe LCU ou clé API exposé (code, log, interface, test, fixture).
- Comportement modifié sans test.
- Code système écrit pour un seul OS sans branche pour l'autre.
- Types Rust et `@olc/shared` désalignés.
- `unwrap()`/`expect()` hors tests et `main` sur un chemin atteignable.
- Chaîne d'interface en dur (hors FR/EN).

## À vérifier

- Diff limité au ticket ; pas de refactor caché.
- Erreurs utiles pour l'utilisateur (message clair, pas de panique).
- Performance : pas de polling agressif (< 1 s) sans raison, pas d'allocation dans une boucle de rendu d'overlay.
- Noms explicites, commentaires en français là où le *pourquoi* n'est pas évident.
- Doc et `CHANGELOG.md` à jour.
