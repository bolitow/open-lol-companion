# Recette Joueurs desktop — 2 octobre 2026 — #64

## Périmètre

Recherche Riot ID/région dans le front réel, commandes Tauri vers l’API #19,
profil/rangs et historique public paginé, compte favori de l’accueil et retour
depuis Champions. Aucune modification du site. Aucun lancement de League.

## Vérifications

- Tests écrits avant code : import du store et méthodes Rust absents, puis verts.
- 6 tests TypeScript nouveaux : identité, restauration locale limitée au nom/tag/région,
  indépendance accueil/consultation, conservation position, pagination après erreur,
  page vide avec suite, réponses tardives, réponse d’un autre compte, erreur privée
  normalisée et refus du stockage local.
- 6 tests Rust nouveaux : encodage URL par segment, identité invalide, profil officiel,
  pagination vide, cohérence identité/curseur, erreurs HTTP, LP négatifs.
- Revue indépendante : un P2 sur les LP négatifs accepté par l’API #19, corrigé avec
  nouveau test rouge puis vert ; aucun autre P1/P2. L’alerte documentaire sur le lien
  de recette est résolue par ce fichier.
- `pnpm test` : **510 tests passants** (155 TypeScript + 355 Rust), deux recettes
  externes déjà ignorées par la suite restent explicitement ignorées.
- `pnpm lint` : typage, formatage Rust et Clippy à zéro.
- Build natif `pnpm --filter @olc/desktop tauri build --debug --bundles app` réussi,
  bundle macOS **68,70 Mio** dans `target/debug/bundle/macos/Open LoL Companion.app`.
  Avertissement Vite préexistant sur la taille de certains chunks.

## Recette visuelle

Front réel vérifié en aperçu navigateur, transport injecté uniquement dans un
fichier de recette temporaire marqué « DONNÉES FICTIVES ». Ce fichier est retiré
des entrées de l’application et conservé avec les preuves locales dans
`work/players-2026-10-02/` ; aucune fixture de profil embarquée en production.

- Recherche globale au clavier, formulaire Riot ID/région et erreur introuvable.
- Profil Solo/Flex, résultat/KDA/durée et pagination 10 → 20 parties.
- Champion ouvert depuis la fin de la liste, retour : 20 parties et position conservées.
- Compte A choisi pour l’accueil, consultation B : accueil toujours sur A.
- Réglages puis retour : contexte conservé, FR/EN et sombre/clair.
- 960×600 et 1280×800 : dimensions du document égales à la fenêtre, défilement
  interne aux panneaux. Pas d’erreur console dans la recette.
- Le front sans injection indique explicitement « transport natif indisponible »
  dans le navigateur ; aucune réussite fictive du chargement réel.

Captures locales : `players-dark-960.png`, `players-dark-1280.png`,
`players-light-en-1280.png`. L’aperçu normal reste disponible sur le serveur local
1421 ; l’onglet de recette et son viewport temporaire sont fermés/réinitialisés.

## Limites conservées

Pas de validation avec un service Riot de production configuré ni de recette
native Windows/macOS de ce nouveau parcours. Le build natif n’est pas une preuve
d’appel API réel. Recettes avec League et Windows reportées par l’utilisateur.
Le code ajouté n’utilise pas d’API système spécifique à un OS.

Le favori local n’authentifie pas le propriétaire. Détection d’identité LCU,
multi-comptes, amis et analyses restent hors périmètre. Les rangs sont actuels à
la date affichée et ne sont pas attribués rétrospectivement aux parties ; pas de
MMR estimé ni d’historique de parties privées.

## Préparation de livraison Git

Autorisation de commits locaux reçue pendant ce lot. Premier snapshot : socle
desktop accumulé (prototype, référentiel, compagnon, LCU/imports, préparation et
Champions), vérifié isolément sans #64 : **498 tests**, lint et build frontend verts.
Deuxième snapshot : ce parcours Joueurs, reposant sur ce socle. Aucun push demandé.

Exclusions : `work/`, tous les doublons de synchronisation suffixés « 2 »/« 3 »,
fichiers temporaires de recette ; aucune suppression des fichiers préexistants.
Les tickets avec recettes natives incomplètes restent ouverts.

Incident de vérification : après réutilisation du cache Cargo pour le snapshot
isolé, la suite globale a réutilisé un artefact build-client antérieur au module
profils (la suite ciblée utilisait un autre jeu de features et passait). Une
recompilation forcée de la crate, sans modification de contenu, a rétabli les
**510 tests puis le lint**. Les validations des deux snapshots sont désormais
terminées ; aucune compilation de la copie isolée n’est relancée en parallèle.
