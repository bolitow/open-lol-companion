# Recette macOS — catalogue cosmétique #93

Date : 4 octobre 2026. Branche d’intégration `codex/catalog-assets-93`.

## Périmètre et dépendances

La branche réunit #191 (catalogue selon le patch client) et #175 (présentation de l’accueil), qui dépend de #174 et #77. Le volume du diff inclut leurs interfaces et catalogues publics déjà développés. Aucune fusion dans main ni mise en production.

## Résultats

- `pnpm test` et `pnpm lint` : réussis sur le diff final ; 77 tests natifs desktop réussis. Tests rouge puis vert pour contrat cosmétique, routage local, URLs de collection et délai de la file d’images.
- Compilation Tauri macOS debug réussie. Copie de recette avec identifiant et stockage distincts, variables d’API locales et jeton fictif ; application utilisateur et session League préservées.
- Export réel du patch 16.19.1 : 2 005 fichiers, 173 champions, 2 159 entrées de skins, 5 050 icônes et 229 séries. Les skins de base peuvent figurer dans les métadonnées ; ces totaux ne représentent pas l’inventaire possédé du joueur.
- Snapshot activé : `a257ff0b59c5d764cb4cf27ff400d04504511fcc401aa0ad82772f0e45a40e97`.
- Icône de profil, galerie et fiche Aatrox de l’éclipse lunaire visibles dans la vraie fenêtre native ; série Éclipse affichée. Images chargées à la demande.
- Serveur de catalogue arrêté puis application relancée : message « Catalogue précédent conservé », collection et images visibles. Le réseau CDN restait disponible : ceci valide l’indisponibilité du serveur de catalogue, pas une coupure réseau complète. Les tests unitaires vérifient séparément les hits disque sans nouvelle requête réseau.
- Aperçu overlay puis édition/annulation : état « Affiché », commandes de réglage désactivées pendant édition, retour « Désactivé » après annulation ; fenêtre principale toujours utilisable. L’outil de capture n’a pas exposé la fenêtre overlay séparée : ni son rendu complet ni une transition de génération simultanée visuellement prouvée ne sont revendiqués.
- Revue stricte indépendante favorable après correction du droit d’écriture LRU Windows, du délai hors attente et de la validation des champs absents. Couverture CI ajoutée pour le cache et l’exporteur.

## Réserves

- Exécution Windows, CI distante et validation de production à faire avant fusion ; aucun test Windows lancé dans ce lot.
- Le hash local détecte une corruption du cache image, pas une correction du CDN à URL identique. Une image versionnée identique en URL reste jusqu’à éviction.
- Les captures et journaux locaux restent sous `work/assets-93/` et ne sont pas publiés. Le serveur, la copie native temporaire et son stockage isolé sont retirés après recette.

Cache observé en fin de recette : 34 fichiers, 2122318 octets, sous les plafonds de 4 096 entrées et 128 Mio.


## Stabilité de la recette automatisée — 6 octobre 2026

La CI Windows de la PR #197 a révélé une limite du test de file : son délai de
250 ms incluait les accès disque. La recette maintient désormais explicitement
la file verrouillée, avance le temps virtuel au-delà du délai, puis reprend le
temps réel avant les accès disque. Elle échoue si le timeout englobe la file
(mutation contrôlée vérifiée), et réussit avec le comportement actuel. Aucun
délai ni comportement du cache de production n’est modifié.
