# Parcours joueur desktop — #64

Périmètre validé : recherche Riot ID/région, profil officiel, historique paginé,
compte choisi pour l’accueil, retour depuis la fiche champion. Cahier des charges
§4.4 et §10, API #19 ; aucune modification du site #20.

## Plan avant code

- [x] Tests rouges : validation et encodage des identités, erreurs HTTP, pagination,
  réponses tardives, indépendance du compte d’accueil, restauration locale.
- [x] Transport dans `crates/build-client/src/profiles.rs`, commandes Tauri minces,
  contrats miroirs `@olc/shared` (profils et historique existants réutilisés).
- [x] État joueur au niveau de l’application : conserver le profil consulté et
  l’historique lors des navigations ; stocker uniquement l’identité choisie.
- [x] Page Joueurs, recherche globale et modules accueil FR/EN ; liens vers Champions.
- [x] Recette visuelle aux petites/grandes fenêtres, sombre/clair, clavier et retour.
- [x] `pnpm test`, `pnpm lint`, build desktop, auto-revue et revue stricte ; documentation.

## Limites explicites

Le favori d’accueil ne prouve pas la propriété d’un compte. Pas d’authentification,
de liaison multi-comptes ou de synchronisation d’identité LCU dans ce lot. Aucun MMR,
aucune partie privée ; pas d’action de jeu. Aucune donnée fictive dans le produit.
Les essais avec League et sous Windows restent reportés par décision utilisateur.
Rust utilise le même transport HTTP sur Windows et macOS, sans branche système nouvelle.
Le navigateur est un aperçu du front desktop et indique l’absence du transport natif.
Le service réel exige une URL et un jeton, comme les builds : variables `OLC_API_URL`
et `OLC_API_TOKEN`, ou **Réglages → Accès à l’API** (trousseau du système, #98).

Preuve attendue : recherche → profil → historique → champion → retour à la même
position ; consulter B conserve A à l’accueil ; fermer League conserve A.

## Résultat

Implémentation et validation locale terminées : [recette et limites](recettes/2026-10-02-joueurs-desktop.md). L’utilisateur a ensuite autorisé les commits locaux des travaux accumulés ; le premier snapshot du socle est vérifié isolément avant le commit #64. Aucun push.
