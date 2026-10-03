# Intégration desktop — 3 octobre 2026

Tickets : #4 (interface), #11 (réglages), #22/#23/#63 (live/overlay), #64/#65 (compte), #71 (amis).

## Périmètre et plan avant intégration

Réunir `origin/main` (`31c8d79`) et le travail d’interface (`c3186b3`) sur `codex/desktop-system`. Conserver le shell compact, l’avatar de connexion, le compagnon et les effets, la bibliothèque Champions et ses vidéos, les dropdowns partagés et les réglages recherchables. Raccorder les données déjà livrées par Matthieu, puis vérifier les contrats, tests et rendu natif. Ne pas reprendre la conception des recommandations avant le moteur de calcul.

## Raccords

- Accueil : vrai composant Amis ; navigation vers un profil seulement lorsque son identité est complète. Aucun champion ou état de partie déduit de la présence sociale.
- Compte : profil/historique source-aware, avatar mémorisé conservé ; le DTO envoyé au transport exclut les métadonnées locales. Pas de mélange de pages LCU/API.
- Partie : écran Live existant, builds consultatifs sans imports.
- Réglages : overlay dans les résultats de recherche FR/EN et la catégorie League ; thème partagé et défilement interne. Le formulaire reste monté quand la recherche le masque, pour conserver une saisie non enregistrée. Son bouton Enregistrer reste indépendant de l’annulation des préférences générales.
- Natif : plugins et états réunis, fermeture en barre système et réouverture macOS conservées, sortie explicite après destruction de la fenêtre principale. Synchronisation du poste personnalisé avant les imports.
- Tests Rust entrants adaptés au champ public d’avatar ajouté sur notre branche.

## Vérification

Deux tests de recherche/rendu des réglages overlay ont échoué avant raccordement puis passé. Les recettes de `docs/amis-client.md` et `docs/live-overlay.md` décrivent la version précédente de main : elles ne valent pas recette de cette intégration. Les résultats de cette passe sont consignés ci-dessous après exécution.

## Travail suivant recommandé

Finir le parcours **Accueil → profil joueur → retour** avec les données désormais présentes : densité identité/rangs, historique et amis, états chargement/hors connexion/erreur, conservation du contexte et fenêtres compactes. Cela ne dépend pas du moteur de recommandations. Ne pas redévelopper le backend Amis ou Live. Les tests en partie et Windows restent un lot distinct, non déclenché pendant cette intégration.

## Résultats de cette passe

- `pnpm test` : 849 tests réussis (390 desktop, 10 partagés, 449 Rust), deux tests Rust existants ignorés.
- `pnpm lint` : typage, formatage et Clippy au vert.
- `pnpm --filter @olc/desktop tauri build --debug --bundles app` : bundle macOS construit.
- Régression permissions : test rouge puis vert sur les quatre commandes de réglages/diagnostic. Elles sont autorisées dans `main` uniquement ; le panneau passif garde sa liste limitée.
- Navigateur isolé, 1 000 × 650 : document exactement à la taille du viewport, défilement interne des résultats ; aucun `<select>` natif. Contrôle visuel sombre/clair.
- Bundle natif : accueil avec dernier compte/avatar conservé, client déconnecté, amis explicitement indisponibles. Le service public de profils n’est pas configuré dans cette installation : pas de profil distant ni d’historique réel validés sur cette passe.
- Réglages natifs lus, overlay retrouvé par recherche ; opacité provisoire 85 conservée après filtre « langue » puis « overlay ». Sortie de la page sans sauvegarde, retour à la valeur enregistrée 90 confirmé.
- Diagnostic : boîte système de sauvegarde ouverte puis annulée, message « Aucun diagnostic enregistré » confirmé. Aucun fichier League sélectionné.
- Fermeture de fenêtre en arrière-plan et réouverture vérifiées, contexte Réglages/recherche conservé. Pas de modification des préférences système pour cette recette.
- Revue stricte indépendante : **OK après correction du manifeste**. Aucun paquet perdu dans le lockfile par rapport aux deux parents.

Captures et journaux de contrôle locaux : `work/integration-main-2026-10-03/` (non versionnés). Application native laissée ouverte. Aucun serveur de recette nécessaire après contrôle. Pas de test Windows, ni de partie LoL, ni de validation FPS/clics de l’overlay en jeu. Aucune publication du site ni push GitHub pendant ce lot.
