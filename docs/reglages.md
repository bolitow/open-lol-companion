# Réglages de l’application — #11

## Lot disponible

La recherche locale retrouve les options par nom, préfixe, synonymes et formulations courantes FR/EN. Accents, casse et petits mots courants sont normalisés. Exemples : « moins d’animations », « réduire les effets », « fewer animations », « Flash sur F », « dark mode ». Tous les mots significatifs doivent correspondre au même réglage. Ce n’est ni une IA ni une commande : taper une recherche ne change aucune valeur et n’envoie aucune requête réseau.

Les catégories Tous, Application et League of Legends filtrent les six options implémentées : thème sombre/clair, langue FR/EN, animations, position de Flash D/F, fermeture dans la barre système et lancement au démarrage. Les contrôles restent utilisables directement dans les résultats. En l’absence de résultat, la page propose des exemples et un retour vers tous les réglages. La recherche et sa catégorie sont gardées en mémoire en quittant la page puis en revenant ; elles ne sont pas stockées sur disque.

La page, l’en-tête et le panneau de sorts utilisent le même état partagé : un choix Flash dans la draft se retrouve dans les réglages, et inversement. Flash reste sans préférence si aucun choix n’existait ; « À choisir » permet d’effacer ce choix. Sans préférence, le panneau demande un choix explicite avant un import contenant Flash. Modifier la préférence ne change ni les sorts dans League ni leur importation : un clic d’import reste nécessaire. Aucun travail sur les imports automatiques #63 dans ce lot.

## Persistance, annulation et accessibilité

Les anciennes clés restent lisibles sans migration destructive :

- `olc.app.preferences` : thème, langue et animations.
- `olc.flash-slot` : `"D"`, `"F"` ou `null`.

Une valeur illisible revient au défaut de son champ. Une erreur d’accès au stockage laisse l’application utilisable en mémoire et affiche un message avec un bouton pour réessayer. Une sauvegarde réussie d’un autre groupe ne masque pas une modification encore non sauvegardée. Aucun secret ou identifiant de compte dans ces préférences.

Chaque action différente de la valeur active remplace la dernière modification annulable. Annuler restaure seulement ce champ, sauvegarde le retour et ne modifie pas les autres options. L’historique d’annulation et les deux marqueurs de modifications récentes sont propres à la session ; aucun historique illimité. Cliquer sur la valeur déjà choisie n’efface pas la possibilité d’annuler.

Le thème et la langue ont un effet immédiat. La réduction de mouvement configurée dans le système reste prioritaire sur l’interrupteur de l’app et est signalée. Les contrôles ont un libellé, un état sélectionné et un parcours clavier ; la page utilise un défilement interne si la hauteur ne suffit plus.

## Réglages système natifs

Les deux options système utilisent des commandes Tauri. Dans un navigateur, elles sont identifiées comme indisponibles et ne simulent aucune modification.

- **Fermer dans la barre système** (activé par défaut) : la croix masque la fenêtre uniquement si le tray est disponible et si le masquage réussit. Sinon la fermeture reste normale. Réduire la fenêtre conserve le comportement de l’OS.
- **Lancer à l’ouverture de session** : désactivé lors d’une installation neuve, jamais activé implicitement. La valeur est relue depuis le système ; une erreur de lecture est un état inconnu, pas « désactivé ». L’app vérifie le résultat après chaque changement.
- Le tray propose Ouvrir, Réglages et Quitter, traduits FR/EN. Le menu Réglages ouvre la page correspondante. Quitter reste une sortie effective ; le clic sur le Dock macOS rouvre la fenêtre.
- Un démarrage avec `--autostart` masque la fenêtre seulement si le tray a pu être créé. L’autostart utilise le mécanisme natif du plugin Tauri : LaunchAgent macOS, démarrage utilisateur Windows. Les tests automatiques ne l’activent pas sur la machine de développement.

`desktop-settings.json` dans le dossier de configuration de l’app contient uniquement `close_to_tray` et `locale`. Écriture temporaire puis remplacement atomique dans le même dossier, lecture limitée à 8192 octets, schéma strict. Un fichier invalide n’est pas écrasé par la synchronisation automatique de langue ; un changement explicite de fermeture peut réparer la sauvegarde. L’état autostart n’est pas dupliqué dans ce fichier. Aucun accès au client League pour ces réglages.

Les commandes système sont sérialisées, leurs erreurs sont des codes fermés. Le contrôle affiche une opération en cours et attend la confirmation native. L’annulation restaure le dernier changement local ou natif, sans réappliquer un historique périmé après relecture du système.

## Vérification et limites

Tests : reprise des clés existantes, synchronisation entre abonnés, annulation ciblée et persistée, stockage refusé partiellement/totalement, recherche FR/EN, catégories, termes incompatibles et absence de résultat. La préparation conserve ses tests d’import ; sa fixture de rendu utilise désormais le même fournisseur de préférences que l’app.

Recette visuelle : fenêtres 960×600 et 1280×800, français/anglais, sombre/clair, recherche et remise à zéro, modification/annulation, changement de page et rechargement. Aucun client League lancé pour cette recette.

Hors lot : nouveaux accès contextuels et réglages de clips/overlays non encore implémentés. Les essais natifs Windows/macOS du parcours et des futurs réglages système restent requis avant clôture du ticket #11.


### Recette native à exécuter sur chaque OS

1. Fermer avec l’option active, rouvrir depuis le tray ; ouvrir Réglages depuis son menu.
2. Désactiver la fermeture dans le tray, fermer et vérifier la fin du processus ; tester Quitter indépendamment de l’option.
3. Activer explicitement le démarrage, vérifier l’entrée OS, annuler ; réactiver et ouvrir une nouvelle session OS pour confirmer le démarrage masqué et la réouverture. Désactiver ensuite.
4. Changer FR/EN et vérifier le menu ; redémarrer l’app pour vérifier la persistance. Sur macOS, tester la réouverture depuis le Dock et Cmd+Q.
5. Vérifier le repli visible si le tray échoue, le message si le stockage est refusé, et l’absence de faux succès lorsque l’OS refuse l’autostart.

Ces recettes restent distinctes des tests unitaires et de la compilation CI. Le ticket #11 reste ouvert jusqu’à leur exécution et aux autres fonctionnalités prévues.


## Export local des diagnostics

Le bouton « Exporter un diagnostic » ouvre un aperçu, puis le dialogue natif d’enregistrement. Il n’envoie rien à un serveur. L’archive ZIP contient trois entrées à noms fixes :

- `manifest.json` : version réelle du package Tauri, famille OS/architecture, limites et exclusions.
- `app-events.json` : au maximum 200 codes techniques de la session courante avec temps relatif (démarrage, connexion/déconnexion, changement de phase ou de réglage système, demandes/fin d’export). Journal en mémoire, perdu à la fermeture ; aucun payload, compte, port, chemin, argument de processus ou message d’erreur libre.
- `league-summary.json` : vide par défaut. L’option explicite permet de choisir au maximum cinq fichiers texte UTF-8 de 8 Mio chacun dans un dialogue système. Elle exporte uniquement le total de lignes et les nombres de lignes contenant les mots ERROR, WARN/WARNING ou INFO (sans distinction de casse). Ce sont des compteurs lexicaux, pas un diagnostic de cause. Aucun nom de fichier, ligne brute ni identité n’est conservé.

L’app n’explore pas les dossiers League automatiquement. Les fichiers non réguliers, liens symboliques, textes contenant NUL, encodages non UTF-8 et sources trop volumineuses sont refusés. La lecture est bornée même si le fichier grossit. L’archive est écrite dans un temporaire du dossier choisi puis remplacée atomiquement ; un refus laisse l’ancien fichier intact. Deux exports ne peuvent pas ouvrir des dialogues concurrents. Annuler à la sélection ou à l’enregistrement ne crée aucune archive.

Ce premier export sûr ne contient **pas les logs bruts app/League**, ni l’historique des sessions précédentes. Leur collecte détaillée reste hors périmètre tant qu’un contrat de filtrage plus riche n’est pas validé. Le résumé peut être insuffisant pour diagnostiquer certains problèmes ; cette limite figure dans l’aperçu et le manifeste.

Recette native complémentaire sur Windows/macOS : ouvrir et annuler les deux dialogues, exporter sans logs, puis avec un fichier synthétique ; inspecter les trois entrées ZIP, vérifier le message d’erreur sur fichier binaire/trop gros et destination non inscriptible. Ne pas utiliser de logs personnels pour les tests de CI.

## Intégration de la branche principale

Le panneau « Imports au prépick » de #63 reste disponible en haut à droite des réglages, dépliable avec défilement interne. Son moteur reste monté entre les pages ; la synchronisation conserve les options et gardes livrées dans #69. La recherche locale des six réglages ne filtre pas encore les contrôles internes de ce panneau.
