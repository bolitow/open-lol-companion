# Contribuer à Open LoL Companion

Merci de votre intérêt ! Le projet est en phase de cadrage : les premières contributions utiles sont les maquettes, les prototypes du connecteur client LoL et la collecte Riot API.

## Avant de coder

1. Lisez [`AGENTS.md`](AGENTS.md) : ce sont les règles de travail du projet (workflow, Definition of Done, anti-patterns), identiques pour les humains et les assistants de code.
2. Lisez le [cahier des charges](docs/cahier-des-charges.md), en particulier la section 2 (conformité Riot).
3. Choisissez une issue (idéalement `good first issue`) et signalez en commentaire que vous la prenez.
4. Pour une nouvelle fonctionnalité, ouvrez d'abord une issue pour en discuter.

## Règles non négociables

- **Aucune clé API Riot, aucun secret** dans le code, les issues ou les PR. Utilisez des variables d'environnement (`.env`, jamais commité).
- Aucune fonctionnalité qui affiche une information absente du client de jeu, automatise une décision ou injecte du code dans le jeu.
- Aucune publicité.
- Aucun asset copié d'un autre produit (logos, visuels, textes). Les assets du jeu viennent de Data Dragon / CommunityDragon.

## Workflow

1. Forkez le dépôt et créez une branche : `feat/draft-lane-swap`, `fix/overlay-dpi`…
2. Commits au format [Conventional Commits](https://www.conventionalcommits.org/fr/), en français (détail : [`rules/documentation.md`](rules/documentation.md)).
3. Ouvrez une pull request vers `main` en liant l'issue (`Closes #12`).
4. Remplissez le bloc « Definition of Done » du modèle de PR et ajoutez vos lignes au [`CHANGELOG.md`](CHANGELOG.md).
5. La CI doit passer (Linux, Windows, macOS) et une revue est requise avant la fusion.

## Tester sur les deux OS

Tout ce qui touche l'app desktop doit être vérifié sur Windows **et** macOS. Si vous n'avez qu'un des deux, dites-le dans la PR : un autre contributeur testera l'autre.

## Langue

Issues et PR en français ou en anglais.
