# Orchestration des sous-agents

## Quand déléguer

- **Oui** : exploration large (« où est géré X ? »), recherche documentaire, tâches indépendantes et parallélisables (ex. traduire des chaînes, écrire des tests pour un module stable), revue stricte indépendante.
- **Non** : modification chirurgicale d'un fichier, décisions d'architecture, tout ce qui touche aux secrets ou à la conformité Riot.

## Contenu obligatoire d'une délégation

1. **Contexte** : ticket, section du cahier des charges, fichiers concernés, règles applicables (AGENTS.md §0).
2. **Objectif mesurable** : ce qui doit être vrai à la fin (« les 5 tests passent », « liste des endpoints avec source »).
3. **Livrable** : format exact attendu (diff, liste, rapport de revue).

## Niveau de raisonnement

| Niveau | Tâches |
| --- | --- |
| `léger` | Recherche de fichiers, renommage, traduction de chaînes, mise en forme |
| `standard` | Écriture de tests, composant React, commande Tauri simple |
| `fort` | Capture vidéo, overlays natifs, concurrence (WebSocket, buffers), modèles IA, revue stricte |

## Vérification

Le livrable d'un sous-agent est traité comme du code généré : relu, testé, soumis à la même Phase C. Un sous-agent ne commit ni ne pousse jamais.
