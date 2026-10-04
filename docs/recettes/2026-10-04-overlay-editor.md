# Éditeur d’overlay — #28 — 4 octobre 2026

## Plan et périmètre

1. Étendre le moteur natif existant avec une session temporaire et réversible.
2. Exposer les gestes de déplacement/redimensionnement dans la petite fenêtre.
3. Conserver les préférences relatives, les données locales et le click-through.
4. Tester géométrie, transactions, contrats, permissions et migration ; contrôler
   le build Mac puis documenter séparément les essais natifs non réalisés.

Branche locale `codex/overlay-editor-28`, issue de `7286b53` (PR #77), afin de
conserver le travail Collection. Livraison demandée le 4 octobre : PR empilée sur la PR #77, sans fusion automatique.
Le checkout principal préexistant n’a pas été modifié.

## Vérifications

- TDD : tests de géométrie initialement rouges (déplacement et redimensionnement),
  puis verts ; test d’affichage des poignées rouge avant leur implémentation.
- Tests automatisés : géométrie relative, limites 4K, minimum éditable 100/150/200 % DPI,
  restauration sur annulation, échec puis réussite de sauvegarde, sessions périmées,
  contexte perdu/expiration, contrat strict, migration des anciennes préférences,
  opacité zéro, rendu passif/édition FR/EN et permissions de fenêtre.
- Build `.app` debug sur Mac ; ouverture réelle et connexion au client LoL.
- Depuis les réglages : « Modifier le placement » fait passer la hauteur de 0
  (automatique) à 30 %, désactive les champs concurrents et indique « Affiché ».
  « Annuler » restaure 0 %, réactive les champs et revient à « Désactivé ».
- L’API de contrôle native expose la fenêtre principale, mais pas de sélection du
  NSPanel secondaire. La capture de la fenêtre principale exclut ce panneau :
  elle ne constitue pas une preuve de son rendu. Aucun glisser-déposer natif n’est
  déclaré validé par ce contrôle.
- L’envoi synthétique Alt+B via cet outil n’a pas produit de changement observé.
  Cela ne valide ni n’invalide le raccourci physique ; test clavier réel à faire.

## Résultats finaux de la tranche

- `pnpm test` : 497 tests TypeScript et 519 résultats Rust réussis, 2 recettes
  ignorées explicitement. Aucun test PostgreSQL sans base ne vaut recette backend.
- `pnpm lint` : typage, formatage Rust et Clippy sans erreur.
- Build debug macOS : succès. Revue indépendante : OK avec réserves natives.
- Le bouton Enregistrer de la session conserve la hauteur 30 % sur disque et
  après relance. Le réglage initial (hauteur automatique 0, overlay désactivé)
  a ensuite été restauré par l'interface et enregistré.
- Incident du contrôle : le raccourci synthétique `super+a` a fermé normalement
  le processus (sortie 0, aucun panic), reproduit indépendamment d'une session
  d'overlay. L'utilisation de l'API d'édition directe du champ a permis de
  terminer le contrôle. Ce résultat ne démontre pas un crash de l'éditeur.
- Journaux locaux : `work/overlay-editor-28/` dans le worktree dédié ; aucun
  serveur de développement créé. Bundle de test conservé dans le cache Cargo.

## Réserves avant clôture du ticket

- Windows : build/recette native de ce diff, focus jeu, premier clic, pointer capture,
  retour click-through après validation/annulation/erreur ; AZERTY physique.
- Mac : mêmes gestes dans le panneau, en partie réelle ; Option+B physique.
- Retenter après changement d’écran, facteur DPI, résolution et deux parties
  successives ; configurations Retina/multi-écrans/HDR non validées ici.
- Mesurer CPU/FPS en partie. Aucun chiffre de performance déduit des tests unitaires.
- Flou Windows non livré : Dark utilise encore un repli graphique sans flou.
- Les autres panneaux du §6 et leurs activations indépendantes restent à raccorder
  lorsqu’ils existeront. Le plein écran exclusif conserve le masquage existant.

**#28 reste ouvert.** Cette tranche apporte l’éditeur du panneau existant, pas une
validation complète de tous les overlays ni de toutes les configurations.
