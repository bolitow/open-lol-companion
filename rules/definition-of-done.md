# Definition of Done — allégements par type de tâche

Le bloc de [`AGENTS.md`](../AGENTS.md) §4 est toujours recopié. Selon le type de tâche, certaines lignes peuvent être `N/A` **avec justification** :

| Type de tâche | Lignes pouvant être N/A |
| --- | --- |
| Doc seule (`.md`) | TDD, types, Windows/macOS, i18n (tests et lint restent exécutés si des fichiers de config sont touchés) |
| CI / configuration | TDD, types, i18n ; la preuve est un run CI vert (lien du run) |
| Interface React sans logique | TDD (mais i18n **obligatoire**) |
| Code Rust système (lockfile, capture, overlays) | Aucune : tests unitaires + Windows/macOS + conformité Riot obligatoires |
| Correctif d'une ligne | Aucune ligne supprimée ; le test de non-régression reste obligatoire si le comportement change |

Jamais N/A : « Aucun secret », « Revue stricte », « Auto-revue », « CHANGELOG.md » dès qu'un commit est livré.
