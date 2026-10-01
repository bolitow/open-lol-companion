# Recette locale du premier lot #18 — 1er octobre 2026

Compte rendu historique du prototype, antérieur à l’extension multirégion.
Le contrat et les résultats actuels sont dans la [recette étendue](2026-10-01-agregation-complete.md).

Périmètre : diff local de `feat/agregation-statistiques` à partir de `7b7aae3`,
non committé. Rust 1.98.0 sur macOS, PostgreSQL 17 en conteneur local. Aucun appel
Riot ; `RIOT_API_KEY` vide pour les commandes de recette.

## Tests automatisés

- `pnpm test` avec `OLC_TEST_DATABASE_URL` pointant sur PostgreSQL jetable :
  **99 réussites** (5 TypeScript, 19 LCU, 50 unitaires collecteur/agrégations,
  18 intégrations collecteur et 7 intégrations agrégations). Aucun test PostgreSQL
  ignoré lors de cette exécution.
- `pnpm lint` : typage TS, formatage Rust et Clippy du workspace à zéro erreur.
- Cycle TDD observé : 8 tests de calcul rouges sur fonctions vides puis verts ;
  4 tests PostgreSQL rouges puis verts ; 3 tests de cadence rouges puis verts ;
  test CLI rouge sur sous-commande absente puis vert. La revue a ensuite ajouté
  des contrôles complémentaires de concurrence, annulation et classement mixte.

Les tests utilisent des parties synthétiques. Ils couvrent les comptes exacts,
les exclusions complètes, les seuils, le classement, les liens multi-runs,
la relance sans double comptage, le rollback à l'échec du commit, un second calcul
concurrent, la publication plus récente, l'annulation avant commit et sa libération
du verrou, ainsi que la CLI sans clé et la cadence à horloge contrôlée.

## Données collectées réelles

Une copie locale jetable de la base des 1 000 parties de #17 a été utilisée.
La copie complète a été terminée et le compte des 1 000 lignes vérifié avant les
mesures ci-dessous. La base source n'a pas été modifiée. Aucun détail de joueur
n'a été exporté dans le dépôt ni affiché dans le rapport.

Commande : `olc-collector aggregate --min-games 100 --json`, exécutée deux fois.

| Mesure | Résultat |
| --- | ---: |
| Parties lues | 1 000 |
| Parties retenues | 968 |
| Remakes exclus | 29 |
| Parties invalides exclues | 3 |
| Observations de participants | 9 680 |
| Victoires / défaites | 4 840 / 4 840 |
| Groupes champion/rôle/patch | 1 990 |
| Groupes atteignant 100 parties | 0 |
| Durée CLI debug, migrations/connexion/sortie comprises | 3,815 s puis 3,781 s |
| Rapports JSON des deux calculs | Identiques |

Les patches observés vont de `16.13` à `16.19`. Les trois parties invalides ont
chacune un participant dont le rôle n'est pas exploitable. Elles sont entièrement
exclues selon le contrat du lot. Une requête SQL indépendante sur les détails
confirme **968 parties, 1 990 groupes et zéro écart** sur comptes et victoires.

Aucun groupe ne franchit le seuil : tous les taux publiés et positions sont
`null`, avec conservation des comptes. Cette recette valide donc aussi l'absence
de classement sur un échantillon insuffisant. Les tests synthétiques couvrent les
taux et positions lorsque le seuil est atteint. Ces durées ne sont pas un benchmark
de production.

## Revue et limites

Auto-revue : diff limité à l'agrégation, migration, commande, tests et docs.
Pas de dépendance ajoutée, de commande Tauri, d'action sur le jeu ou de secret.
Revue stricte indépendante : **OK avec réserves documentées**, aucun blocage.

- Windows n'a pas été exécuté pendant cette recette ; CI existante et recette
  Windows à faire lors de la livraison. Aucun comportement conditionnel propre
  à un OS dans ce nouveau module.
- Le minimum Rust 1.77 déclaré reste non validé avec les dépendances actuelles.
- Un COMMIT déjà envoyé peut aboutir malgré Ctrl+C ou une perte de connexion :
  le résultat est à vérifier en relisant la publication.
- Lecture complète à chaque calcul et groupes en mémoire : pas de validation
  de dimensionnement en production.
- #18 reste ouvert pour les rangs, autres régions/files et données statiques.
