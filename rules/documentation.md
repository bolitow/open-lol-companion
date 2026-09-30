# Documentation, branches, commits et PR

## Quoi documenter

| Changement | Fichier à mettre à jour |
| --- | --- |
| Nouvelle commande, prérequis, variable d'environnement | `docs/DEMARRAGE.md` |
| Changement de périmètre ou de choix technique | `docs/cahier-des-charges.md` (+ décision dans la PR) |
| Nouvelle règle d'équipe | `AGENTS.md` ou le fichier `rules/` concerné |
| Tout changement visible (utilisateur ou contributeur) | `CHANGELOG.md` |
| Fonction publique d'une crate | Doc comment `///` en français |

## Branches

`type/description-courte` depuis `main` : `feat/draft-lane-swap`, `fix/overlay-dpi`, `docs/…`, `chore/…`, `refactor/…`. Une branche = un ticket.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/fr/), message en français :

```
feat(draft): recalcul des suggestions après un lane swap

Explication du pourquoi si nécessaire.

Refs #12
```

Types : `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`, `ci`. Portée = module (`lcu`, `draft`, `overlay`, `recorder`, `web`, `collector`…).

## Pull requests

- Titre au format commit, description via le modèle (`.github/pull_request_template.md`) avec le bloc DoD.
- `Closes #…` pour chaque ticket terminé.
- CI verte (Linux ; Windows et macOS si l'app, le connecteur ou `@olc/shared` changent) + une revue avant fusion ; fusion par *merge commit* ou *squash* selon la taille. Une PR en brouillon ne lance pas les builds Windows et macOS.
