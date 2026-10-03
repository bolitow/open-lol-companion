# Collection de skins — premier lot #47

La page **Collection**, accessible depuis le menu de l’application, lit le catalogue du client League connecté. Recherche, filtre par champion, filtres Tous/Possédés/Manquants/Souhaits et fiche de skin utilisent les données réelles. Les skins de base, chromas et versions de champions de modes auxiliaires sont exclus par intersection des identifiants avec le catalogue `skins.json` du même client. Aucun rapprochement par nom. Deux entrées strictement identiques du même identifiant sont réduites à une ; des données contradictoires pour le même identifiant font refuser la réponse. Une location ou un accès temporaire ne compte pas comme une possession permanente ; une possession absente ou illisible reste inconnue.

Les souhaits sont locaux, propres au compte et à cet appareil. Ils sont confirmés dans l’interface après écriture réussie. Un souhait n’achète rien et ne modifie pas le client League. Les filtres, la sélection et le défilement restent en place en naviguant entre les pages ; un changement de compte réinitialise cette vue.

## Données et confidentialité

- Rust lit `/lol-champions/v1/inventories/{summonerId}/skins-minimal`, après vérification du compte courant, lit aussi `/lol-game-data/assets/v1/skins.json` pour identifier les véritables skins, puis revérifie l’identité et l’identifiant après la lecture. Le cycle est borné à 15 secondes.
- Le schéma a été confronté à une réponse réelle du client macOS le 3 octobre 2026. Référence de contrat : [schéma LCU communautaire](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json).
- Les chemins publics d’images sont validés puis convertis en URL HTTPS CommunityDragon suivant sa [documentation des ressources](https://communitydragon.org/documentation/assets). Aucun mot de passe ni URL authentifiée du client ne traverse IPC.
- Le nombre total représente le catalogue retourné par ce client, pas une promesse d’exhaustivité de toutes les éditions historiques du jeu. `stillObtainable` n’est pas une preuve de disponibilité actuelle en boutique : le champ brut reste dans le contrat mais aucun libellé d’achat/disponibilité n’est présenté dans ce premier lot.
- Aucun montant de RP dépensés, série ou rareté n’est déduit du prix théorique ou du nom du skin.

## Contrat et cycle de vie

`CollectionState` est défini dans `packages/shared/src/collection.ts` et projeté depuis Rust : révision, statut, identité publique, skins, souhaits, erreur de stockage, indicateur de données périmées.

Les commandes `collection_state`, `collection_refresh` et `collection_set_wish` ainsi que l’événement `collection-state` sont réservés à la fenêtre principale. Le panneau overlay ne reçoit pas ces droits. Une mutation transmet la révision, l’identifiant du skin et l’état souhaité ; Rust rejette une révision obsolète, un skin absent et une collection non disponible.

Une génération annule toute lecture ancienne lors d’une actualisation ou d’un changement de compte, y compris A → B → A. Le contrôleur TypeScript ordonne les événements, le GET initial et les réponses de mutation selon la même révision.

À la fermeture de League, la dernière collection reste visible en mémoire et clairement signalée hors connexion. Pendant une actualisation échouée, elle est signalée à actualiser. Dans ces deux cas les souhaits sont en lecture seule. Une nouvelle connexion charge le compte actif. Le catalogue n’est pas sauvegardé sur disque : redémarrer l’application sans client connecté ne restaure pas l’inventaire.

## Stockage

Dossier `collection-wishes` dans `app_config_dir()` de Tauri ; un fichier JSON versionné par plateforme et identifiant numérique du compte. Cette clé interne ne quitte pas Rust. Sous macOS, ce dossier est sous `~/Library/Application Support/io.github.bolitow.openlolcompanion/` ; sous Windows, il est résolu par Tauri dans le dossier de configuration de l’utilisateur.

Les lectures et mutations sont sérialisées. Écriture atomique via les primitives communes existantes ; taille bornée (128 Kio, 10 000 souhaits), identifiants positifs uniques. Un fichier invalide ou d’une version inconnue est préservé et produit un état indisponible, jamais écrasé comme une liste vide.

## Validation et périmètre restant

Tests : parsing de possession et d’images, catalogue invalide, changement de compte pendant lecture, persistance/rechargement/erreurs, isolation des comptes, révisions et événements tardifs, réinitialisation de vue, permissions natives, filtres.

La recette manuelle Windows reste à faire pour les nouveaux filtres et passages vidéo. La couverture vidéo exhaustive, les chromas et la finition visuelle avancée restent hors de ce lot. La disponibilité actuelle en boutique ne sera pas déduite de `stillObtainable`. Ce lot ne clôture pas le ticket complet.

### Recette et garde-fous

- La recette réelle a révélé un nom fourni par Riot avec un espace de bord : normalisation testée, sans relâcher les bornes de taille ni la validation des caractères.
- L’inventaire réel a été chargé dans le build Tauri macOS ; images distantes affichées, aucun inventaire fictif injecté.
- L’isolation multi-compte et les coupures pendant les requêtes sont testées par fixtures ; aucun changement forcé du compte League de l’utilisateur.
- La grille monte 60 cartes au départ, puis ajoute des lots au défilement ou via « Afficher plus ». Recherche et compteurs portent toujours sur le catalogue complet ; la limite déjà affichée est conservée en revenant sur la page.
- Recette native : ajout d’un souhait, redémarrage complet de l’application, souhait retrouvé, retrait puis actualisation ; le souhait de test a été retiré. Retour Accueil → Collection : recherche et fiche sélectionnée conservées.
- Validation finale du lot : `pnpm test` (899 tests réussis, 2 tests Rust ignorés préexistants), `pnpm lint`, build Tauri macOS et revue stricte indépendante OK.
- Traçabilité TDD : rouge/vert observé pour parseurs, stockage, contrôleur et régressions ; les tests d’intégration du runtime Tauri ont été ajoutés après son écriture. Ce point est un écart à la méthode TDD complète exigée par le dépôt.

## Images progressives et SkinSpotlights

Les huit premières cartes et le splash sélectionné sont prioritaires ; les autres cartes restent en chargement différé. Le cadre garde ses dimensions pendant le décodage asynchrone. La miniature déjà chargée reste affichée jusqu’à l’arrivée du splash, avec transition respectant les mouvements réduits et repli en cas d’erreur. Un survol/focus de 150 ms prépare le splash : deux chargements simultanés, huit références maximum, expiration à 12 secondes et nettoyage au changement de compte ou départ de la page. Le cache HTTP demeure géré par la WebView ; aucun préchargement de toute la collection ni de vidéo.

`apps/desktop/public/game-data/skin-spotlights.json` contient les correspondances vérifiées par **identifiant Riot de skin**, avec contrôle du champion et rejet des doublons. Couverture vérifiée : **23 skins**, Ahri (9), Lux (6), Garen (4), Jinx (1), Caitlyn (1), Jhin (1), Ezreal (1), vérifiés le 3 octobre 2026. Les noms EN exacts du catalogue DataDragon 16.19.1 ont servi à vérifier les références hors ligne ; l’application ne recherche jamais une vidéo par nom traduit. Chaque entrée conserve le titre, les dates de publication/vérification, la source Riot et le lien YouTube de la chaîne officielle. Les neuf vidéos Ahri retenues sont postérieures ou contemporaines à son ASU de 2023. La date de publication est accessible dans l’infobulle du crédit vidéo : une vidéo référencée ne constitue pas une garantie de mise à jour pour tout patch futur.

Pour ajouter une association : vérifier le skinId et championId Riot, la chaîne `UC0NwzCHb8Fg89eTB5eYX17Q`, la version PC finale (pas PBE/Wild Rift), la dernière refonte visuelle et l’intégration autorisée. Enregistrer la source et les dates puis lancer les tests du catalogue. Une association absente affiche « Vidéo non référencée » ; une erreur de chargement du catalogue permet de réessayer.

L’aperçu « Lire la vidéo » ouvre la visionneuse intégrée décrite ci-dessous. Le même lecteur peut être détaché puis rattaché sans recréer la vidéo. Lecture volontaire avec les commandes YouTube, aucune vidéo téléchargée/réhébergée. « Ouvrir sur YouTube » reste disponible dans la fiche en cas de blocage réseau, suppression ou refus d’intégration. La chaîne reste identifiée. La commande n’accepte qu’un identifiant vidéo ASCII de onze caractères et reconstruit des URL YouTube fixes ; aucune URL arbitraire ne vient du front.

Le contenu distant ne possède aucune capability IPC. Navigation limitée au lecteur et nouvelles fenêtres bloquées ; seule la fenêtre principale peut appeler la commande. Le lecteur utilise une session privée et les API WKWebView/macOS ou WebView2/Windows pour envoyer l’identité installée en Referer, conformément aux [exigences YouTube pour lecteurs embarqués](https://developers.google.com/youtube/terms/required-minimum-functionality#embedded-player-api-client-identity). L’acceptation d’une navigation native ne prouve pas la lecture effective : celle-ci doit être vérifiée visuellement. La lecture de ce lot reste à recetter sur Windows ; le résultat macOS ne valide pas WebView2.

### Recette du lot médias — 3 octobre 2026

- `pnpm test` : **929 réussis**, deux tests Rust ignorés préexistants ; `pnpm lint` et build Tauri macOS au vert.
- Lecture réelle Arcade Ahri observée dans la fenêtre native : progression de 0:03 à 0:41, puis pause à 1:08 ; contenu du jeu visible, pas seulement une miniature ou un état de navigation.
- Repli « Ouvrir sur YouTube » : navigateur par défaut ouvert sur la bonne URL et vidéo lue ; onglet de test fermé ensuite.
- Images de la galerie et du détail affichées ; changement Ahri → Aatrox sans référence → Ahri vérifié, avec disparition/réapparition correcte des actions vidéo. Aucun souhait modifié.
- Revue stricte indépendante : OK avec réserves — tests du composant image limités au rendu initial SSR (les erreurs/file du préchargeur sont testées), pas d’injection de panne CDN dans la WebView, recette Windows restante. Le formatage signalé pendant la revue est corrigé et le lint final confirme.
- Captures locales et journaux regroupés dans `work/collection-media-47/` ; build natif laissé ouvert. Aucun commit ni push pour ce lot.

## Séries, raretés et accès directs — 3 octobre 2026

La rareté et les séries sont lues par identifiant exact dans le `skins.json` du client, pendant la lecture déjà existante. Le contrat transmet `rarity` (valeur connue ou `null`) et `series_ids` (liste dédupliquée). Un skin peut appartenir à plusieurs séries. Aucune classification n’est inférée de son nom ou de son prix. Les filtres intersectent recherche, champion, possession, rareté et série ; leur modification remet le défilement au début. La fiche affiche les métadonnées disponibles.

Les noms de **228 séries**, en français et en anglais, proviennent du catalogue public CommunityDragon **16.19** ; les URLs sources sont conservées dans `skinLines.json`. Une série inconnue du catalogue local n’est pas inventée. Actualisation reproductible :

```sh
python3 -B scripts/update-skin-lines.py --patch 16.19
python3 -B -m unittest discover -s scripts -p 'test_update_skin_lines.py'
```

Le script valide les IDs, rejette les doublons et exige la même couverture FR/EN avant remplacement atomique. Il ne s’exécute pas dans l’interface et n’ajoute aucune dépendance réseau à l’application.

### Passages SkinSpotlights

Le lecteur natif unique reçoit maintenant un début et une fin optionnels. Les boutons Passif / A / Z / E / R / Rappel / Emotes (et autres passages explicitement identifiés) s’affichent uniquement lorsqu’ils sont référencés. **Six vidéos disposent de 51 passages vérifiés** : Arcade Ahri, K/DA Ahri, Spirit Blossom Ahri, Snow Moon Caitlyn, Empyrean Jhin, HEARTSTEEL Ezreal. Les autres conservent la vidéo entière.

Les débuts sont les timestamps publiés dans la description SkinSpotlights ; la fin correspond au chapitre suivant. La provenance et l’empreinte SHA-256 de la description sont conservées. Cela délimite un chapitre, pas un montage à l’image près. Les chapitres ambigus (par exemple le passif du E de Jhin) ne deviennent pas artificiellement le passif du champion. Aucun découpage, téléchargement ou réhébergement de média.

Le catalogue rejette les segments superposés, inversés, dupliqués, hors durée et les vidéos associées deux fois. Rust valide aussi les bornes (maximum deux heures) et reconstruit les URLs canoniques. Le [lecteur YouTube](https://developers.google.com/youtube/player_parameters#start) peut démarrer à l’image clé précédente : une légère approximation est attendue. La lecture reste volontaire. Le repli externe conserve le début du dernier passage, y compris après une erreur ; la vidéo entière ou un changement de skin réinitialise ce choix. YouTube externe ne garantit pas l’arrêt à la fin du segment.

Les tests couvrent également les événements tardifs, le changement de compte, les états déconnectés et la persistance des souhaits. Une page temporaire de recette utilisant les vrais composants a confirmé dans un navigateur : image et miniature indisponibles, miniature conservée après erreur du splash, récupération de l’image en changeant de source et message de repli vidéo. Ce test de panne CDN n’est pas une injection de panne dans la WebView native.

### Validation de cette passe

- `pnpm test` : **947 tests réussis**, deux tests Rust ignorés préexistants. Générateur de séries : **6 tests Python réussis** en complément.
- `pnpm lint` : typage, formatage et Clippy sans erreur ; remplacement d’une méthode trop récente pour respecter le MSRV Rust 1.77.
- Revue stricte indépendante du code et de la documentation : **OK** ; générateur et catalogue relus séparément par un autre relecteur.
- Build Tauri macOS réussi ; application relancée et reconnexion réelle au client observée. Le Mac s’est verrouillé au moment d’ouvrir Collection : **les nouveaux filtres et la lecture effective des nouveaux passages n’ont pas encore reçu leur recette visuelle native**. La lecture complète de l’ancien lot reste la preuve décrite plus haut, sans couvrir ces nouveaux comportements.
- Recette des nouveaux passages également à faire sur Windows. Aucun résultat Windows ne doit être déduit de la compilation Mac ou des tests unitaires.
- Preuves et préparation de livraison locale : `work/collection-completion-47/`. Aucun commit/push effectué ; les modifications précédentes d’autres tickets restent préservées. #47 reste ouvert.


## Recherche enrichie et visionneuse intégrée — 3 octobre 2026

La recherche associe nom du skin, champion, rareté et série (libellés FR et EN), sans accents ni distinction de casse. Chaque mot saisi doit correspondre : `Ahri legendaire` et `Ahri Spirit Blossom` peuvent être combinés avec les filtres existants. La classification reste celle des identifiants Riot ; Spirit Blossom est une série, pas une rareté. Autocorrection, capitalisation automatique, vérification orthographique et autocomplétion du champ sont désactivées.

La fiche latérale présente un aperçu visuel « Lire la vidéo ». La grande visionneuse affiche la vidéo, le passage actif, les accès directs et les flèches précédent/suivant (boucle sur les passages réellement référencés, plus la vidéo entière). Les contrôles Détacher / Rattacher et Fermer sont intégrés. Dans l'application, clic extérieur et Échap ferment la modale lorsque le focus appartient aux contrôles locaux ; les touches du lecteur YouTube gardent leur comportement propre.

Le contrat `SpotlightState` et les commandes `skin_spotlight_state`, `skin_spotlight_control`, `skin_spotlight_layout` utilisent une révision partagée. Rust résout skin/champion et passages dans le catalogue embarqué. Seul le shell propriétaire peut modifier la session ; les événements périmés sont ignorés. La destruction tardive d'une ancienne fenêtre ne ferme pas une nouvelle session. Les événements sont ciblés sur les Webviews locales, jamais diffusés au lecteur distant.

Le lecteur distant est une Webview enfant isolée, sans permissions IPC. Le shell local détaché ne reçoit que les trois commandes du lecteur et l'écoute d'événements. La capability principale cible désormais la **Webview main**, pas tous les enfants de sa fenêtre. Le déplacement conserve la même instance YouTube et sa position de lecture ; changer de passage charge l'URL canonique correspondante. La lecture reste volontaire via YouTube, et son chargement réseau peut prendre quelques secondes. Aucune vidéo n'est préchargée en masse, téléchargée ni réhébergée.

L'implémentation utilise la fonctionnalité `unstable` de Tauri 2.12 pour les Webviews enfants et leur déplacement : elle devra être revérifiée lors d'une mise à jour Tauri. Sur macOS, les coordonnées incluent l'inset automatique de WebKit sous la barre de titre opaque, calculé avec les API publiques AppKit. Sur Windows, le rectangle reste relatif à la Webview et le Referer est transmis par WebView2. Le shell reprend le thème et les préférences de mouvement de l'application.

### Recette native de la visionneuse

- `pnpm test` : **967 tests réussis**, deux tests Rust ignorés préexistants ; `pnpm lint` au vert après correction de l’ordre des fonctions demandé par Clippy. Revue stricte indépendante : **OK avec réserves de recette**.

- Recherche réelle : `Ahri Spirit Blossom` et `Ahri legendaire` donnent chacun trois résultats ; `Ahri arcade` en donne un.
- macOS : cadrage corrigé, lecture Emotes Arcade Ahri observée à 0:42 puis 0:54 ; détachement et rattachement conservant la pause à 0:54. Flèche suivante : Rappel sélectionné et lecteur positionné à 0:57.
- Un écran noir a été reproduit sur l'ancien lecteur avant cette passe. Le nouveau lecteur a effectivement lu Emotes, mais cela ne démontre pas que tout blocage réseau YouTube est éliminé. Réessayer et ouvrir sur YouTube restent disponibles.
- Le Mac s'est verrouillé pendant la suite de la recette : fermeture/réouverture, redimensionnement, thème clair et fin automatique du passage restent à vérifier visuellement sur ce build. Windows n'a pas été recetté dans cette passe.
- Rouge/vert observé pour la recherche, le modèle de session, les permissions, le thème détaché et la génération du shell. Les adaptateurs natifs de géométrie ont été vérifiés par compilation et recette, sans test rouge préalable : écart TDD documenté.
- Sources, plan, traces de tests et résultats de revue : `work/collection-viewer-47/`. Cache Cargo isolé sous `~/Library/Caches/open-lol-companion/collection-viewer-47-target`, les archives du cache dans Documents ayant présenté des doublons. Aucun commit/push ; #47 reste ouvert.


## Finition de la galerie et des fiches — 3 octobre 2026

Le titre Collection, le pseudo et la source « Client League » ne prennent plus de bandeau. Sous les filtres, une seule ligne rassemble résultats filtrés (si différents du total), possédés/total et actualisation. Les indications utiles hors connexion et d’erreur restent présentes. Le champ annonce explicitement skin, champion, rareté et série en FR/EN.

La fiche rassemble nom et possession dans le splash, puis rareté/séries et souhait. L’aperçu SkinSpotlights est directement visible, avec bouton Lecture ouvrant la grande visionneuse existante. Le choix de cette passe est une **miniature cliquable**, pas une lecture dans le panneau étroit. La miniature YouTube ne démarre aucune vidéo ; attribution et accès externe restent visibles. Les skins sans référence gardent leur état explicite.

Collection et Champions partagent désormais `detailLayout.css` : deux colonnes permanentes dont la largeur et l’écart évoluent sur 240 ms, avec fondu/déplacement du détail. Son contenu garde sa largeur cible pendant le mouvement ; à la fermeture il devient inerte et est retiré après la sortie. Le composant Collection est réinitialisé immédiatement au changement de compte. Les préférences système et applicatives de mouvement réduit désactivent ces transitions. Le nombre de colonnes de cartes peut encore changer par palier : ce lot n’ajoute pas d’animation individuelle à chaque carte.

### Vérifications de la passe

- Tests de présentation : rouge constaté avant modification, puis vert ; suite complète `pnpm test` : **969 tests réussis**, deux tests Rust ignorés préexistants. `pnpm lint` et build Tauri macOS réussis.
- Recette native macOS en fenêtre 1280 × 800 : inventaire réel, compteur unique, recherche Ahri, fiche et aperçu entièrement visibles, ouverture/fermeture du grand lecteur puis retour à la fiche ; thèmes sombre et clair. Ouverture/fermeture des fiches Champions Ahri et Aatrox, focus restauré sur la carte.
- Deux tentatives de redimensionnement par l’automatisation n’ont pas changé la taille : **la fenêtre minimale 960 × 600 reste à vérifier visuellement**. Aucun résultat Windows ni mesure de fluidité/FPS n’est revendiqué. Les règles compactes et de mouvement réduit ont été relues.
- Revue stricte indépendante : aucun défaut P1/P2 identifié, réserves de recette conservées. Contrats Rust/shared et lecteur natif inchangés dans cette passe.
- Preuves et plan : `work/collection-polish-47/`. Build natif laissé ouvert pour revue. Aucun commit/push ; #47 reste ouvert pour les vérifications restantes et la couverture médias.


## Maintenance par lot du catalogue SkinSpotlights — 3 octobre 2026

L’application charge un catalogue JSON unique, indexé par identifiant Riot, et réutilise le même lecteur. Ajouter une association ou des chapitres n’ajoute aucun composant, iframe ou requête de recherche dans l’interface. Les médias restent sur YouTube et sont lus à la demande.

L’outil `scripts/update-skin-spotlights.py` remplace la liste de travail codée en dur des premiers essais. Il fonctionne hors application, sans paquet Python externe, avec au maximum quatre contrôles vidéo simultanés et une lecture DataDragon par champion pour tout le lot. Il actualise les références du catalogue et accepte une liste de nouveaux candidats :

```json
[{"champion":"Ahri","videoId":"IPU9_WRcsj4","minPublishedAt":"2023-02-05"}]
```

Cet exemple documente le format ; cette vidéo étant déjà dans le catalogue, l’utiliser comme ajout est volontairement refusé comme doublon. `champion` est la clé DataDragon. La date minimale est une décision de revue liée à la version visuelle du champion ; l’outil ne sait pas détecter une refonte dans les images. Pour les références existantes, leur date de publication sert de borne minimale.

```sh
# Revalider toutes les références et compléter les chapitres disponibles.
python3 -B scripts/update-skin-spotlights.py --patch 16.19.1 \
  --output work/spotlight-batch-47/candidate.json \
  --report work/spotlight-batch-47/report.json
# Ajouter --candidates chemin/du/lot.json pour de nouvelles références.
python3 -B -m unittest discover -s scripts -p 'test_update_skin_*.py'
```

Le rapprochement exige un nom Riot anglais unique, avec pour seule normalisation la casse. La chaîne officielle, l’identifiant vidéo, les dates, la durée et l’intégration déclarée sont contrôlés. Les collisions de skins et de vidéos sont refusées. Le générateur **ne découvre pas automatiquement toutes les vidéos de la chaîne** : les références candidates restent à sélectionner, puis le lot automatise la vérification et la création des associations.

Les chapitres reconnus explicitement sont Passif, Q/W/E/R, rappel, emotes, attaques de base, déplacement et mort. La fin d’un passage est le début du chapitre suivant, même si ce dernier est exclu. Les labels inconnus/mélangés et les types répétés vont au rapport ; aucune interprétation d’un « E Passive » en passif du champion. Une ligne de timestamp malformée fait échouer le contrôle, pour éviter d’étendre le passage précédent. Une curation déjà présente est préservée si la description SHA-256 et la durée n’ont pas changé ; autrement, une différence demande revue.

Le catalogue source n’est jamais écrasé par la commande. La sortie candidate est distincte et atomique ; **toute erreur empêche son écriture**, mais le rapport est produit. Une ancienne sortie candidate éventuelle n’est pas modifiée en cas d’échec : vérifier impérativement le code de sortie et le rapport du dernier lancement. Après revue du diff et du rapport, intégrer les données et exécuter les tests du catalogue TypeScript/Rust avant livraison.

### Source et limites

Les métadonnées viennent actuellement du JSON public présent dans les pages YouTube, sans exécution de script, cookies ni téléchargement vidéo. **Ce format HTML n’est pas contractuel** : un changement ou une page de consentement provoque un échec explicite, pas une association inventée. Pour une alimentation automatisée à grande échelle, la source durable à prévoir est l’[API officielle YouTube Videos](https://developers.google.com/youtube/v3/docs/videos) (description, chaîne, durée, intégration), avec les autorisations et quotas associés, hors du client distribué. Aucun secret ajouté dans cette passe. Les bornes de chapitre restent approximatives par rapport aux images et ne prouvent pas la lecture effective de chaque passage.

### Résultat du lot

- 23 références existantes recontrôlées ; **21 vidéos avec 178 passages**, contre six vidéos et 51 passages avant ce lot. Les six anciennes timelines sont conservées, 127 accès directs ajoutés.
- Dark Cosmic Lux et God-King Garen n’ont pas de chapitres publiés : lecture complète conservée.
- Une différence de casse HEARTSTEEL/Heartsteel a été signalée et résolue par correspondance unique, sans rapprochement flou ; le Homeguard d’Ahri fleur spirituelle reste une curation antérieure sur description/durée inchangées.
- Environ 11 Ko supplémentaires en JSON compact. Aucun nouveau code de lecteur ou d’interface, aucun téléchargement de vidéo.
- TDD : 18 tests de générateur (identité, chapitres, erreur réseau, doublons, lot idempotent, curation, export atomique), plus régression TypeScript du catalogue, rouge/vert observés. Avec les séries Riot : 24 tests Python réussis.
- Revue stricte indépendante : OK avec réserves de recette ; collecte HTML non contractuelle et absence de lecture systématique de chaque nouveau passage explicitement conservées.
- Validation du lot : `pnpm test` **970 réussis**, deux ignorés préexistants ; `pnpm lint` et build Tauri macOS au vert. Tests Python séparés : **24 réussis**.
- Recette native macOS : les nouveaux boutons et bornes d’Ahri de minuit sont affichés ; Emotes d’Ahri de l’assemblée (nouveaux passages) et d’Ahri arcade (témoin) lus avec images de jeu visibles. Fermeture et retour à la fiche confirmés. **Ahri de minuit a présenté un écran noir persistant malgré Réessayer** : cause non déterminée, lecture non validée. Ce résultat ne permet ni d’accuser le catalogue ni de valider toutes les vidéos. Windows non recetté dans cette passe.
- Plan, rapports, candidat et journaux : `work/spotlight-batch-47/`. Aucun commit/push ; #47 reste ouvert.


## Extension multi-champions — 3 octobre 2026

Le catalogue contient désormais **58 vidéos pour 30 champions**, dont **55 avec 441 passages**. Ce lot ajoute **35 références et 263 accès directs** sans modifier le lecteur ou l’interface. Les 23 références précédentes et leurs timelines sont conservées à l’identique. La taille JSON formatée est d’environ 101 Kio ; aucun média n’est embarqué.

La découverte a parcouru une page de recherche publique SkinSpotlights pour chacun de 47 champions, sans prétendre épuiser la chaîne : 426 références candidates distinctes. Le générateur a retenu 35 associations ; 275 n’ont pas satisfait les contrôles (notamment date, identité, intégration), et **116 sont restées non vérifiées suite aux réponses HTTP 429 de YouTube**. Ces dernières sont en attente, pas classées comme mauvaises vidéos. Aucune reprise massive n’a été effectuée après constat de la limitation.

La priorité est donnée aux vidéos publiées depuis janvier 2025. Des bornes conservatrices supplémentaires ont été utilisées suite aux notes Riot : Talon/Mordekaiser après les modifications VFX du printemps 2025, Kai’Sa après les changements audio 25.22, Pyke après 25.11 et Aurora après sa correction du E en janvier 2025. Sources et décisions dans `work/spotlight-expansion-47/visual-review.md`, notamment [25.08](https://www.leagueoflegends.com/en-us/news/game-updates/patch-25-08-notes/), [25.22](https://www.leagueoflegends.com/en-us/news/game-updates/patch-25-22-notes/) et [25.11](https://www.leagueoflegends.com/en-us/news/game-updates/patch-25-11-notes/). Cette recherche est ciblée, pas une certification des effets de chaque skin au dernier patch.

Exemples ajoutés : Petals of Spring Yasuo, Masked Justice Yone, Faerie Court Gwen, Firecracker Caitlyn, Risen Legend Orianna et plusieurs skins de Senna, Lucian, Jhin, Rakan, Xayah, Sett. Les variantes et titres ne sont jamais rapprochés approximativement. Radiant Serpent Sett conserve la vidéo entière : ses chapitres répétés entre formes ne rentrent pas dans le modèle d’un seul passage par type ; le E répété de Masked Justice Senna est également omis.

### Arrêt sur limitation de débit

Le générateur suspend désormais le reste du lot dès une réponse HTTP 429, sur Riot comme sur YouTube. Les quatre requêtes déjà en vol au maximum peuvent finir ; aucune reprise automatique ni attente agressive. Le rapport conserve les erreurs et l’export reste bloqué tant qu’il y en a. Deux régressions rouge/vert vérifient l’arrêt des requêtes et la conservation du catalogue source. Les références en attente seront à reprendre dans un lot ultérieur après disponibilité du fournisseur ; ne pas relancer en boucle.

### Traçabilité

- Découverte, métadonnées publiques utilisées, acceptations, exclusions, références en attente et diff regroupés dans `work/spotlight-expansion-47/`. Le cache ne contient aucun flux média ni secret.
- Revue indépendante du candidat : IDs Riot, chaîne, bornes de date, hash des descriptions, unicité, durées et chapitres contrôlés ; références précédentes identiques.
- Tests Python générateurs : **26 réussis**, dont 20 pour SkinSpotlights ; test TypeScript du catalogue étendu rouge/vert.
- Validation globale : `pnpm test` **971 réussis**, deux ignorés préexistants ; `pnpm lint` et build Tauri macOS au vert.
- Recette native macOS : Yasuo pétales de printemps sélectionné dans la collection, lecteur ouvert avec les neuf accès directs ; passage Emotes lu avec images de jeu visibles, flèche suivante vers Rappel et bornes 1:06–1:18 confirmées, puis fermeture et retour à la fiche. Capture : `work/spotlight-expansion-47/yasuo-native.png`.
- La lecture de chaque vidéo n’est pas garantie par les métadonnées ; l’anomalie d’écran noir sur Ahri de minuit du lot précédent reste ouverte. Aucun résultat Windows ne doit être déduit d’une recette Mac.
- Aucun commit/push ; #47 reste ouvert.


## Fiabilisation et livraison — 3 octobre 2026

Chaque changement réel d’URL ou action Réessayer crée une nouvelle vue média native. Un simple redimensionnement ou détachement conserve le lecteur et sa position de lecture. Cela sépare les sessions YouTube et les événements natifs des anciennes tentatives. Aucun préchargement global, téléchargement de média ni autoplay n’est ajouté.

Le chargement du **document** est suivi séparément : en cours, chargé, prolongé après 15 secondes, erreur native. Les événements tardifs après fermeture, relance ou A→B→A sont ignorés par génération. Un document chargé ne garantit pas la lecture du média : les erreurs internes YouTube ne sont pas encore remontées par l’API IFrame. L’aide de repli reste donc visible, même après chargement du document. Les erreurs/restrictions YouTube ne sont ni contournées ni masquées.

L’ouverture externe conserve la session si la commande OS échoue ; après succès elle ferme la vidéo intégrée et son éventuel shell détaché, pour éviter deux lectures simultanées. Le lien externe conserve le début du passage choisi. La liste, les filtres et la fiche du skin restent dans l’application.

Diagnostic macOS : l’ancien lecteur a affiché un écran noir à 0:00 pour Emotes d’Ahri de minuit, malgré rechargement ; la recréation du média directement sur ce passage a permis une lecture avec images de jeu à 0:30. C’est une récupération observée, pas une preuve d’absence de toute panne intermittente YouTube. Les tests automatiques couvrent les générations, les transitions et la présentation des messages FR/EN. La recette finale et la réserve Windows sont consignées dans [le rapport de livraison](recettes/2026-10-03-collection-videos.md).

La livraison rapproche les prérequis UI déjà développés localement des imports de sorts et recettes intégrés entre-temps sur `main`. La préférence Flash et l’activation existante des imports de sorts sont conservées. Les retouches Accueil/Profil/Amis encore locales ne font pas partie de ce lot. Les générateurs Python sont exécutés en CI Linux.

## Inventaire de tous les skins — 3 octobre 2026

Le passage complet utilise les 173 champions Data Dragon du patch 16.19.1, contrôlés contre la liste officielle `champion.json`. Chaque entrée hors apparence de base reçoit une ligne de couverture. Les chromas sont identifiés par le champ Riot `parentSkin`, séparés des skins principaux et signalés comme variantes ; ils ne gonflent pas le nombre de skins en attente de vidéo. Aucune association au parent n’est inventée. La fenêtre de sept ans porte sur la **publication vidéo**, pas sur la sortie du skin : minimum 2019-10-03 pour cette recette. Le skin reste consultable et souhaitable même sans référence vidéo. Une ancienne vidéo ne doit pas être importée uniquement pour remplir une fiche.

`scripts/catalog-skin-spotlights.py` réutilise les métadonnées publiques en cache, examine une première page de recherche de la chaîne par champion puis vérifie l’association dans l’ensemble des noms Riot anglais. L’option `--search-uncovered-skins` complète ce passage par une recherche ciblée des noms Riot sans résultat exact ou à revoir, sans rouvrir les candidats déjà classés trop anciens. Ces recherches restent partielles ; leurs statuts et sources sont exportés en CSV, y compris les blocages fournisseur. Un résultat trouvé par la recherche d’un autre champion ou skin peut être associé seulement si le nom exact est unique. Aucun rapprochement flou des éditions Prestige, chromas ou variantes. Une vidéo vérifiée par métadonnées ne garantit pas sa lecture dans toutes les Webviews.

La collecte est séquentielle, au plus une nouvelle requête par seconde, et cesse les requêtes dès un HTTP 403/429. Le cache reste exploitable ; une panne réseau ne transforme pas un refus de validation en vidéo « en attente ». Rapport/CSV et catalogue candidat sont séparés. Si l’inventaire Riot est incomplet ou vide, le rapport est écrit mais le candidat est refusé avec un code non nul. Aucun média n’est téléchargé et aucun cache de collecte n’est livré avec l’application.

Le catalogue atteint 895 vidéos, dont 812 avec 6 176 passages : 838 ajouts et retrait de God-King Garen (2018), sans modifier les 57 références conservées. Quatre descriptions contiennent des chapitres désordonnés : la vidéo complète validée reste disponible, mais tous les raccourcis sont retirés, sans réparation au jugé. Ce repli est une option explicite du vérificateur ; sa validation stricte par défaut reste inchangée. [Couverture complète et groupes à revoir](recettes/2026-10-03-skins-video-coverage.md).

### Bornes après mises à jour visuelles ou de compétences

La fenêtre générale est resserrée pour les versions dont une évolution importante a été repérée dans les notes Riot. Ces bornes sont conservatrices : elles peuvent exclure une présentation finale publiée juste avant la sortie. Le rapport conserve les cas pour une revue manuelle. Ce n’est pas un audit de toutes les modifications VFX mineures de sept ans.

| Champion | Publication minimum retenue | Source Riot |
| --- | --- | --- |
| Fiddlesticks | 2020-04-01 | [10.7](https://www.leagueoflegends.com/en-us/news/game-updates/patch-10-7-notes/) |
| Volibear | 2020-05-29 | [Mise à jour de Volibear](https://www.leagueoflegends.com/en-au/news/community/celebrate-volibear-s-update-with-a-free-skin/) |
| Dr Mundo | 2021-06-09 | [11.12](https://www.leagueoflegends.com/en-gb/news/game-updates/patch-11-12-notes/) |
| Udyr | 2022-08-24 | [12.16](https://www.leagueoflegends.com/en-au/news/game-updates/patch-12-16-notes/) |
| Aurelion Sol | 2023-02-10 | [13.3, report au 10 en SEA](https://www.leagueoflegends.com/en-au/news/game-updates/patch-13-3-notes/) |
| Jax | 2023-10-11 | [13.20](https://www.leagueoflegends.com/en-us/news/game-updates/patch-13-20-notes/) |
| Skarner | 2024-04-03 | [14.7](https://www.leagueoflegends.com/en-gb/news/game-updates/patch-14-7-notes/) |
| Lee Sin | 2024-05-01 | [14.9](https://www.leagueoflegends.com/en-us/news/game-updates/patch-14-9-notes/) |
| Teemo | 2024-10-09 | [14.20](https://www.leagueoflegends.com/en-us/news/game-updates/patch-14-20-notes/) |
| Viktor | 2024-12-11 | [14.24](https://www.leagueoflegends.com/en-us/news/game-updates/patch-14-24-notes/) |

Pour Fiddlesticks, Mundo, Udyr, Jax, Skarner et Viktor, le lendemain des notes sert de borne prudente lorsqu’elles n’explicitent pas le jour de sortie. Cette inférence peut exclure une vidéo à revoir. Les bornes précédentes Ahri/Caitlyn/MissFortune/Talon/Mordekaiser/Kai’Sa/Pyke/Aurora restent documentées dans les lots antérieurs et conservées dans l’outil.

### Relancer une maintenance

```sh
python3 scripts/catalog-skin-spotlights.py \
  --sources apps/desktop/public/game-data/catalog/sources.json \
  --cache work/spotlight-full-47/cache \
  --minimum-date 2019-10-03 --checked-at 2026-10-03 \
  --search-uncovered-skins \
  --output work/spotlight-full-47/candidate.json \
  --report work/spotlight-full-47/report.json \
  --coverage-csv work/spotlight-full-47/coverage.csv
```

Actualiser les dates pour chaque nouvelle recette. `--previous-cache` peut être répété pour reprendre les anciens JSON ; `--offline` limite la passe aux caches. Valider le code de sortie et relire le candidat avant copie dans `public/game-data/skin-spotlights.json`. Les recherches HTML ne sont pas contractuelles ni exhaustives, et une erreur fournisseur ne doit pas être relancée en boucle. Sur ce Mac, le Python de maintenance a été lancé avec `SSL_CERT_FILE=/etc/ssl/cert.pem` pour utiliser le magasin de certificats système ; aucune vérification TLS n’a été désactivée.

Utiliser un dossier de cache Riot distinct lors d’un changement de patch : un JSON Riot d’une autre version est refusé, pas remplacé silencieusement. Le cache de métadonnées YouTube ne remplace pas une recette de lecture récente.
