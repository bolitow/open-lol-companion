# Page Champions desktop — plan d’implémentation

> Exécution directe, selon superpowers:executing-plans ; revue indépendante finale. Autorisation utilisateur du 2 octobre 2026. Aucun commit ni push.

**Objectif :** consulter les champions hors draft, ouvrir leur fiche depuis la recherche globale et retrouver le contexte après navigation.
**Ticket :** #13 (page builds desktop), complément de #4 ; #19 fournit les statistiques, #20 reste le site de Matthieu.
**Architecture :** état de consultation distinct dans le reducer de l’app ; index compact dérivé du catalogue local ; compétences à la demande ; composants de préparation réutilisés en lecture seule. Aucun nouvel endpoint ni dépendance.
**Spécification :** cahier des charges §2/5.3 ; parcours Champions approuvé dans la conversation.

## Contraintes
- FR/EN, thèmes existants, cartes illustrées, sidebar absente avant sélection.
- Pas de défilement global à partir de 960×600 ; défilement interne aux listes et à la fiche.
- Les classes officielles filtrent la grille ; les postes filtrent les builds. Le catalogue ne contient pas de postes fiables.
- Ne pas inventer maîtrise, matchs récents, recommandations ou données pro : services non raccordés.
- Conserver recherche, classe, tri, sélection, onglet, filtres builds et défilement pendant la session.
- La consultation ne modifie ni la draft, ni les imports, ni les préférences de sorts.
- Recettes natives/parties reportées à la demande de Louison. Tests automatiques et parcours navigateur exécutés pour ce lot.

## Vérifications prioritaires
Réponse tardive d’un autre champion/langue ; résultat vide ; données indisponibles/réessai ; retour après changement automatique de page ; clavier et petite fenêtre.

## Tâche 1 — index, recherche et contexte
- [x] Tests rouges : noms FR/EN, accents/ponctuation, classes, tri, limite globale ; isolation de la préparation ; retour et changement de phase.
- [x] Générer un index compact reproductible à partir des fiches normalisées (classes/titres/version), sans charger les 173 fiches à l’ouverture.
- [x] Implémenter le modèle et l’état ; tests verts.
Fichiers : championDirectory.ts/.test.ts, state.ts, championNavigation.test.ts, script et index dérivé.

## Tâche 2 — parcours visuel
- [x] Grille et fiche : compétences, builds avec filtres et variantes, catalogue runes/objets sans recommandation implicite.
- [x] Réutilisation readonly des panneaux ; tests SSR bloquant les actions d’import/préférences.
- [x] Recherche globale champions/pages au clavier ; conservation du contexte ; FR/EN et CSS borné.
Fichiers : ChampionsScreen.tsx, ChampionProfile.tsx, ChampionSearch.tsx, championsCopy.ts, champions.css, BuildPreparation.tsx, App.tsx, copy.ts.

## Tâche 3 — validation et documentation
- [x] Parcours réel dans le navigateur, sombre/clair/anglais, 960×600 et 1280×800, clavier et erreurs.
- [x] pnpm test, pnpm lint, build frontend ; auto-revue et revue stricte indépendante.
- [x] CHANGELOG, intégration et ticket #13 : preuves et limites ; aucun ticket clôturé sur une recette reportée.

## Journal
- 2 octobre : baseline des fichiers existants conservée dans work/champions-2026-10-02/baseline. Studio désactivé : workflow direct. Checkout existant conservé pour préserver les lots non commités et la collaboration en cours.

- Validation : 498 tests, lint et build Mac verts ; recette navigateur et revue indépendante OK. Détails et limites dans docs/recettes/2026-10-02-champions-desktop.md.
