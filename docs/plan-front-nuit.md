# Front réel — lots du 1er octobre 2026

Autorisation : enchaîner les cinq lots convenus, sans commit, push, fusion ni publication. Les parties publiques sont différées. Le checkout partagé et ses changements préexistants sont conservés ; copie des fichiers concernés avant intervention dans le dossier de travail `sal/work/night-front/baseline`.

## Ordre et preuves attendues

- [x] **1 — Objets (#13, #16).** Réutiliser exactement `imports/items.rs` de la PR #59 (cf04f75), ajouter les contrats partagés et la commande Tauri, puis importer explicitement la variante visible dans `BuildPreparation`. Tester la séquence, les doublons, le catalogue absent, les erreurs et les réponses périmées. Aucun assemblage artificiel de catégories statistiques indépendantes. Succès = requête acceptée, ordre réel dans la boutique à vérifier séparément.
- [x] **2 — Personnalisées Faille (#12–15), implémentation.** Étendre seulement les gardes et la projection nécessaires aux équipes incomplètes/bots sur carte 11 CLASSIC. Tester le refus des autres cartes/modes et des contextes périmés. Recette réelle mutualisée au lot 5, encore partielle.
- [x] **3 — Session et retour (#8, #12, #13).** Vérifier changement de champion, sortie de sélection, reconnexion, conservation de la préparation lors des allers-retours. Couvrir le reducer et les états d'éditeur ; pas de résultat ancien après changement de contexte.
- [x] **4 — Finition draft (#12, #13).** Clarifier le lien champion local / champion consulté / préparation ; états préselectionné/verrouillé et infobulles accessibles. Vérifier les petites fenêtres sans défilement global, FR/EN et mouvement réduit. Garder la direction graphique approuvée.
- [ ] **5 — Recette et traçabilité.** Tests et lint globaux, revue indépendante, build natif, contrôles Mac et coordination Test Windows sur le snapshot exact ; sélection privée sans lancer de partie et restauration après imports ; tickets, documentation et bilan des limites réelles.

## Discipline par lot

Écrire un test qui échoue avant de changer un comportement. Exécuter `pnpm test` et `pnpm lint`, relire le diff, obtenir une revue stricte et mettre à jour docs/CHANGELOG avant le lot suivant. Les tests simulés ne valent pas une validation LCU réelle. Secrets uniquement dans Rust ; messages IPC traduits, jamais bruts. Vérifier les livrables et arrêter les seuls processus temporaires créés pour la recette.

## Journal

- Cadrage : moteur objets PR #59 inspecté en lecture seule. Import possible hors draft avec client connecté ; remplacement limité au set de l'app pour le champion/la carte. Les catégories disponibles ne prouvent pas des conseils situationnels.

- Lot 1 : tests ciblés rouge puis vert ; `pnpm test` et `pnpm lint` verts ; revue indépendante OK. Recette native mutualisée au lot 5, ordre boutique différé à une partie autorisée. Collision de noms à la casse corrigée avant validation.

- Lot 2 : tests/lint globaux verts, revue indépendante OK après ajout du cas gameflow tardif puis sortie du lobby. Le mode n’est pas deviné depuis la taille des équipes. Recette réelle réservée au lot 5.

- Lot 3 : tests/lint globaux verts, revue indépendante OK. Retour Ahri + Support vérifié dans le navigateur. Limite : sans identifiant de draft, un cycle entièrement survenu hors connexion ne peut être distingué d’une reconnexion à la même sélection.

- Lot 4 : draft contrôlée à 960×600 et 1280×800 (aucun débordement document), sélection Jinx puis retour Ahri et infobulle de passif contrôlés. Revue stricte OK après correction du libellé lecteur d’écran et test rouge/vert FR/EN.

- Lot 5, état partiel au 1er octobre : 489 tests globaux et lint Mac verts, revue finale OK, bundle Mac livré ; build et tests ciblés Windows réussis. Tickets #8/#12/#13/#14/#15/#16 mis à jour. Recette native interrompue : salon non obtenu puis Mac verrouillé ; après correction CSS et reconstruction Windows, erreur WebView2 et autorisation locale en attente. À cette date, imports réels personnalisés et contraste final non validés. Reprise précise et preuves dans [la recette](recettes/2026-10-01-front-nuit.md).

- Reprise du 2 octobre : sources inchangées depuis le correctif CSS, build Mac identique. Windows relancé dans la session utilisateur normale ; contraste sombre/clair confirmé. Mac reconnecté au client réel, puis erreur de connexion League et crash de signature invalide du lanceur après redémarrage : prérequis d’installation bloquant, aucune signature contournée. Recette privée poursuivie côté Windows.

- Résultat de la reprise : import UI des sorts Windows et moteur d’objets réels réussis. Sortie de sélection échouée côté pilotage Windows, lancement automatique involontaire de la partie privée, processus arrêté, retour spontané hors partie puis restauration finale réussie. Aucun nouvel essai chronométré. Runes UI, objets UI avec API de statistiques et imports Mac restent à vérifier ; le lot 5 n’est pas clôturé.
