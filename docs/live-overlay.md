# Données en partie et panneau minimal — #22, #23, #63

Le parcours conserve le champion réellement verrouillé dans LoL, puis affiche ses
informations en partie. Le panneau est activable dans **Paramètres → Overlay en
partie**. Les imports au prépick restent indépendants ; les deux sorts d’invocateur
ne changent pas. Les suggestions adaptées aux événements et rappels de #27 ne
font pas partie de ce livrable.

## Utilisation

1. Démarrer l’API locale et l’app avec la configuration de développement habituelle
   (voir [imports automatiques](imports-automatiques.md)).
2. Dans les paramètres, activer le panneau et enregistrer. **Ctrl + Alt + O**
   active/désactive aussi le panneau. Un conflit avec un autre raccourci est signalé
   comme indisponibilité ; les paramètres restent utilisables.
3. Tester **Aperçu pendant 30 secondes** hors partie. Cet aperçu n’invente aucune
   statistique. Choisir son écran, la position relative, la largeur et l’opacité.
4. Jouer en mode fenêtré ou sans bordure. Le panneau apparaît lorsque la source
   locale est prête et que le jeu est au premier plan. Les clics traversent le
   panneau ; il ne prend pas le focus. Retour bureau, fin de partie, données
   indisponibles ou désactivation le masquent au prochain contrôle (environ une
   seconde après changement d’état, hors blocage système). Détecter une coupure
   Live peut ajouter la cadence de lecture (1 s) et son délai d’expiration (2 s).
5. Cocher **Je joue en plein écran exclusif** pour supprimer le panneau dans ce
   mode. La détection automatique de l’exclusif et les alertes avancées ne sont pas
   implémentées ; aucune compatibilité avec ce mode n’est promise.

Le cadre du panneau est relatif à la fenêtre du jeu ; la hauteur suit le contenu
avec un maximum de 80 % de celle-ci, avec fond transparent hors contenu. Les coordonnées sont bornées
pour garder ce cadre dans le jeu. Sur macOS, CoreGraphics fournit le cadre de la
fenêtre, qui peut inclure ses décorations ; sur Windows, la surface cliente est
utilisée. L’écran choisi sert uniquement à l’aperçu. Débrancher cet écran n’empêche
pas de désactiver le panneau ; relancer l’aperçu requiert un écran existant.

## Contenu et provenance

- Champion local, niveau, K/D/A, CS et temps de jeu : données locales effectives.
- Inventaire final le plus joué et fondamentale de la page de runes la plus jouée :
  catégories statistiques séparées, avec effectifs, taux publiés, patch, poste et
  population. L’inventaire ne représente pas un ordre d’achat et ne s’adapte pas à
  la situation. L’échantillon insuffisant et les données absentes restent visibles.
- Les informations complètes sont consultables dans l’écran **En partie**, en
  lecture seule, sans commandes d’import.

Le poste et la file proviennent de la dernière draft réelle, jamais du champion
consulté dans l’application. En personnalisée Faille, le poste assigné a priorité,
avec la préférence manuelle des imports en repli ; la population Solo/Duo (420)
est explicitement indiquée. La clé du champion Live est recoupée avec le catalogue.
Un contexte inconnu ou divergent ne déclenche pas de requête statistique. Démarrer
l’app au milieu d’une partie permet de voir les données Live, mais pas de reconstruire
un poste ou une file absents. Un redémarrage du client LCU peut également perdre ce
contexte : aucun poste par défaut n’est inventé.

## Architecture

- `crates/lcu-connector/src/live` : GET fixe
  `https://127.0.0.1:2999/liveclientdata/allgamedata`, sans authentification ni proxy,
  redirections refusées, délai de 2 s, corps limité à 4 Mio. Une seule requête à la
  fois, délai de 1 s après réponse, aucun rattrapage en rafale. Activé uniquement
  pendant GameStart/InProgress/Reconnect. Un changement LCU annule la lecture.
- Le producteur mémorise la dernière draft et les transitions de partie avant
  regroupement des événements. Sortie de partie ou recul significatif de l’horloge
  invalident l’ancien contexte. Les instantanés ont une révision et une génération.
- Projection Rust : seulement le joueur local identifié sans ambiguïté ; aucune
  identité ou donnée adverse transmise. Événements retenus : GameStart,
  MinionsSpawning, GameEnd, sans acteur, dédupliqués par ID dans l’instantané et
  bornés à 32. Il ne s’agit pas encore d’un bus de notifications de #27.
- Le front écoute avant de lire l’état initial et rejette les révisions périmées.
  Changer de partie/contexte/langue démonte les anciennes requêtes de catalogue et
  de builds. Les réponses tardives ne réaffichent pas l’ancien champion.
- Fenêtre Tauri unique, cachée au démarrage, transparente et non focalisable.
  macOS : NSPanel non activant, CanJoinAllSpaces + FullScreenAuxiliary, opérations
  sur le thread principal. Sur macOS 26+, `NSGlassEffectView` fournit Liquid Glass ;
  macOS 13–25 utilise `NSVisualEffectView` HUD natif. Le fond CSS devient transparent
  et l’opacité est appliquée une seule fois à la fenêtre native. Windows : fenêtre Tauri/tao transparente, topmost et
  non activante. Aucune injection ni interaction dans le processus de jeu.
- Reconnaissance du jeu actif par exécutable/PID et cadre natif. macOS installé :
  `LeagueofLegends`, bundle `com.riotgames.LeagueofLegends.GameClient` ; couches
  natives 0 et 1000 (sans bordure LoL observé), panneau aux niveaux 3 et 1001.
  Le cache inclut le niveau pour suivre un changement de mode à taille identique. Windows :
  `League of Legends.exe`. Le client LeagueClientUx n’est pas le jeu.
- Préférences dans `app_config_dir/overlay.json`, via Rust. Échec d’écriture signalé.
  Fermeture de la fenêtre principale termine aussi le panneau.
- ACL : la fenêtre passive n’a accès qu’aux événements et lectures Live/overlay/builds.
  Elle peut aussi communiquer sa propre hauteur de contenu (bornée), sans déplacer
  ou activer le jeu. Les commandes d’import, de profils et de configuration sont réservées à la fenêtre
  principale. Le mot de passe LCU et la clé Riot ne quittent jamais Rust.

Sources vérifiées : [API officielle Riot](https://developer.riotgames.com/docs/lol#game-client-api_live-client-data-api),
[NSPanel](https://developer.apple.com/documentation/appkit/nspanel),
[tauri-nspanel 2.1](https://github.com/ahkohd/tauri-nspanel/tree/v2.1),
[raccourcis Tauri](https://v2.tauri.app/plugin/global-shortcut/),
[GetClientRect](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclientrect).

## Validation et limites

Les tests automatisés couvrent projection, données incomplètes, identité ambiguë,
transport borné, timeout, coalescence draft→partie, révisions, réponses tardives,
lecture seule, placement relatif et masquage. Les builds et tests ne prouvent pas
la recette réelle : voir [recette du 2 octobre](recettes/2026-10-02-live-overlay.md).

Avant clôture de #22/#23/#63 : Matthieu valide une vraie partie macOS ; Louison
valide Windows. Il faut notamment confirmer clics/clavier traversants, focus,
Spaces macOS, résolution/scaling, parties successives et mesure FPS/CPU. La cible
< 2 % de perte FPS et < 3 % CPU n’est pas encore mesurée. La validation Riot avant
publication publique prévue au cahier des charges reste applicable.

## Précision des CS

Le compteur provient directement de `allPlayers[].scores.creepScore` pour le joueur actif.
Le compagnon lit la Live API chaque seconde après la réponse précédente et transmet
chaque entier reçu sans arrondi ni seuil de publication. La valeur peut néanmoins
évoluer par paliers de dix côté LoL : [signalement Riot #416](https://github.com/RiotGames/developer-relations/issues/416), toujours ouvert au contrôle du 2 octobre 2026.
Ce signalement développeur ne constitue pas une garantie contractuelle Riot.

L'interface FR/EN indique cette limite. Aucun CS intermédiaire n'est déduit de l'or,
du temps ou d'une lecture mémoire. Les tests garantissent la transmission de 9, 10,
11, 19 et 20 si la source les fournit. Au dernier diagnostic le jeu était fermé :
la nouvelle comparaison des réponses brutes en partie reste à effectuer.
