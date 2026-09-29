# Capacités externes et replis

Les capacités ci-dessous peuvent être fournies par un skill, un serveur MCP ou un outil du client. Si l'une manque : appliquer le repli, le signaler en une ligne, **continuer**.

| Capacité | Usage | Repli si absente |
| --- | --- | --- |
| `planification` | Plan structuré avant code | Liste numérotée dans la réponse : fichiers, tests, risques |
| `docs-librairies` | Doc à jour d'une crate / d'un paquet (docs.rs, npm, v2.tauri.app) | Lire le code source de la dépendance dans `~/.cargo/registry` ou `node_modules`, ou la doc officielle via le web ; sinon ne pas utiliser l'API et le signaler |
| `revue-stricte` | Revue indépendante du diff | Relire le diff avec la checklist de [`revue.md`](revue.md), point par point, en se plaçant en relecteur hostile |
| `git-commit` | Commit normé | Format de [`documentation.md`](documentation.md), à la main |
| `navigateur` | Tester le site ou l'interface | `pnpm dev:ui` + description précise de ce qu'il faut vérifier à la main |
| `ci` | Lire les résultats GitHub Actions | Demander à l'humain le lien ou le log du job en échec |

Contraintes connues des environnements distants (sessions cloud) : les registres npm / crates.io peuvent être inaccessibles. Dans ce cas, les tests et lints Rust complets s'exécutent en CI ; le dire dans le DoD (« commande : exécutée en CI, run #… »).
