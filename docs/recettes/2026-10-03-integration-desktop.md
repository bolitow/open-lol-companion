# Intégration desktop et vérification des collectes — 3 octobre 2026

## Périmètre

À la demande de Matthieu, réunir les lots #18, #22, #23 et #63 avec la MR #68 de Louison (`965b917`), puis livrer sur `main` après CI. Aucune nouvelle campagne ni infrastructure externe n’est créée par cette livraison.

Les conflits conservent les services amis/Live/overlay, le raccourci, les plugins système, la réouverture macOS et les commandes de diagnostic. La fermeture masque uniquement si le tray existe et que le masquage réussit ; une vraie destruction de la fenêtre principale termine aussi l’overlay.

Flash utilise le fournisseur de réglages commun : page générale, imports automatiques et panneau manuel. Les clés existantes sont conservées. Un choix explicite reste utilisable en mémoire si sa sauvegarde échoue, avec message et reprise ; aucun choix implicite en cas de lecture absente. Les gardes empêchant de réécrire un import déjà envoyé sont inchangées.

Le panneau overlay reste présent dans les résultats défilants des réglages et accessible par « overlay » ou « panneau partie ». Le moteur d’import reste monté pendant la navigation.

## Vérifications automatiques

- Le premier lancement a révélé que les nouvelles commandes système étaient absentes du manifeste de permissions, introduit après la branche de Louison. Quatre autorisations sont ajoutées à la fenêtre principale, sans les exposer à l’overlay. Le test reproduit le refus avant correction puis valide les deux périmètres.
- Deux tests de régression overlay et un test Flash chargé depuis la préférence partagée ont échoué avant correction, puis réussi.
- `pnpm test` : 296 tests TypeScript, 459 tests Rust, zéro échec. Les tests PostgreSQL conditionnels sans environnement local sont couverts par la CI Linux ; ceci ne prétend pas les exécuter contre la base de collecte.
- `pnpm lint` : typage, format et Clippy sans avertissement.
- Auto-revue et revue stricte indépendante : OK.
- Les recettes réelles antérieures restent dans leurs rapports respectifs ; les vérifications natives Windows, autostart après ouverture de session et performances restent distinctes de la compilation CI.

## Audit en lecture seule des collectes

Contrôles PostgreSQL réalisés le 3 octobre entre **08:41 et 08:44 UTC**, par transactions `REPEATABLE READ READ ONLY`, puis lectures de l’API existante. Aucun runner n’a été démarré ni relancé et aucun secret n’a été exporté. Les relevés successifs ne constituent pas un seul instantané, car la collecte courante continue.

- Campagne multirégion historique #1 : `paused`, `riot_auth_rejected`, depuis le 2 octobre à 07:11:59 UTC. Son rapport final est [publié séparément](2026-10-01-agregation-complete.md).
- Collecte courante #18 : **EUW1, Solo/Duo 420**, fenêtre du 7 août au 2 octobre, cible configurée de 1 000 000 parties. À un relevé : **8 592 retenues**, dont **8 562 nouvelles et 30 réutilisées**. Les compteurs et horodatages progressent entre les relevés ; elle n’est pas terminée.
- Au relevé de couverture brute : **18 263 matchs**, **18 263 timelines disponibles**, aucune timeline absente ou marquée indisponible. Un travail timeline peut naturellement être en vol dans un relevé suivant.
- Les travaux courants sont terminés, en attente ou en cours ; aucun état `failed` ou `retry_wait` observé. Plus de 453 000 téléchargements candidats restent en attente, ce qui ne garantit pas autant de nouvelles parties incluses.
- Publication utilisée par l’app : source du **3 octobre 08:10:04 UTC**, publiée à **08:16:15 UTC**, limitée à EUW1/420 et aux patchs 16.17, 16.18, 16.19. **8 636 sources = 8 537 incluses + 99 remakes exclus**.
- Somme des couvertures : **8 537 parties, 85 370 participations**, toutes avec timeline. Les **5 027 groupes** respectent `games = wins + losses` ; les groupes `rank=ALL` totalisent les mêmes 85 370 participations.
- Rangs : 76 886 participations classées, 8 475 de rang inconnu et 9 non classées. La présence des parties ne signifie donc pas une couverture parfaite des rangs.
- Catalogue statique : versions 16.18.1 et 16.19.1 complètes ; version courante servie 16.19.1.
- API : santé, manifeste, tierlist et builds renvoient HTTP 200. Pour Ahri Mid/EUW1/420/16.19/ALL, les **107 variantes** et les effectifs maximaux par catégorie correspondent à PostgreSQL : runes 260, sorts 412, objets finaux 5. La pagination est explicitement élargie à 200 pour couvrir toutes les catégories.

### Limites à conserver avant le futur serveur

Le seuil publié est actuellement **1**, adapté aux essais : il ne prouve pas une représentativité statistique. Le seuil d’import de l’application reste une préférence distincte. Les variantes sont bornées à 20 par catégorie ; 328 129 variantes supplémentaires sont omises dans cette publication et cette limite est déclarée par l’API. La publication contient moins de parties que la base brute en raison des filtres et de sa date source. L’audit confirme la présence, la progression et des invariants de cohérence, pas l’exactitude indépendante de chaque événement ou la fin de la campagne.

Le choix du serveur externe, le transfert de PostgreSQL, les secrets, quotas et modes de reprise seront définis séparément avec Matthieu.
