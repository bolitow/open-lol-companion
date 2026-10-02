# Intégration du front réel — #8, #21, préparation #12 / #13

Le 1er octobre 2026, Louison autorise le passage de la maquette à l'application. La direction validée dans #4 est la référence ; les corrections visuelles se feront désormais sur le front réel. Le prototype publié et Figma restent consultables.

## Premier lot et plan avant code

1. Relier le watcher Rust existant à Tauri : un état courant versionné, une commande de lecture initiale et un événement. Le client est connecté seulement après une connexion réussie, pas à la seule présence du lockfile. Aucune donnée d'authentification ne franchit cette frontière.
2. Construire la navigation React et tester les transitions, doublons, événements obsolètes, déconnexion, retour et maintien des réglages ouverts. Réutiliser `screenForPhase` de `@olc/shared`.
3. Intégrer l'en-tête minimal (menu à gauche, réglages à droite), l'accueil et les états draft/partie/bilan. Préférences locales FR/EN, sombre natif/clair, réduction des animations. Pas de faux profil, recommandations, historique ni succès d'import.
4. Vérifier tests, lint, compilation, revue stricte et parcours navigateur. Tester le binaire macOS si possible ; la recette Windows avec LoL reste à Louison, celle macOS à Matthieu.

Architecture : `src/app` contient état, traductions, adaptateur Tauri et écrans ; `src/ui` contient l'identité commune. Le watcher du connecteur reste la source de vérité. Les vues de jeu attendent les données métier de #12/#13 : une phase connue ne suffit pas à inventer une composition. Le navigateur affiche explicitement l'absence de connexion native.

## Suite fonctionnelle, conservée dans le périmètre des tickets

- #12 : cartes illustrées, distinction prépick/pick, mise à jour animée de la seule carte concernée, côtés bleu/rouge corrects ; aucune sélection automatique.
- #13 / #61 : arbres de runes complets, objets/composants/utilitaires, données localisées fiables. Infobulles personnalisées pour passifs et statistiques au survol et au focus, fermeture Échap, placement dans le viewport ; pas de valeurs inventées.
- Reprendre les effets de combustion et particules validés dans les composants communs après mesure dans la WebView native ; respecter les mouvements réduits et suspendre hors visibilité. Ne pas confondre ce travail avec les overlays de jeu.
- La barre de scénario appartient exclusivement au prototype et ne sera pas importée dans l'app.

Les comptes ne sont pas encore fournis par le connecteur : aucun compte n'est fabriqué ou effacé. Leur mémorisation et le changement automatique attendent l'intégration dédiée.

## Preuves du premier lot — 1er octobre 2026

- Tests des nouvelles transitions et du cycle d'abonnement observés en échec avant implémentation, puis verts ; tests Rust du snapshot et de sa sérialisation également rouges puis verts.
- `pnpm test` : 74 tests réussis (48 desktop, 5 shared, 21 Rust). `pnpm lint` et build Tauri macOS debug réussis.
- Navigateur : FR/EN, thème clair/sombre et préférence d'animation conservés au rechargement ; recherche vers Draft, retour Réglages → Draft ; disposition 1366×768 et 390×844 sans débordement horizontal observé ; aucune erreur/alerte console relevée.
- Bundle Tauri macOS lancé : lecture du snapshot natif confirmée par « En attente du client » (et non « Aperçu navigateur »), recherche et clic sur Draft exercés dans WebKit.
- Revue indépendante : deux défauts corrigés (clic recherche WebKit et textes de session injustifiés), contre-revue OK. Auto-revue : aucun secret exposé, pas de polling React, pas de nouveau chemin système spécifique à un OS ; contrats Rust/TS alignés.
- Non vérifié : session réelle Lobby → Draft → Partie, reconnexion au client réel, recette Windows, budget CPU/GPU en partie. Ces limites empêchent de clôturer les tickets fonctionnels.
- Le connecteur #7 est réutilisé sans modifier TLS, endpoints ni découverte système. Aucune dépendance npm ajoutée ; `tokio` déjà présent dans le workspace est désormais déclaré par le crate desktop pour le canal.
- Build de contrôle : `target/debug/bundle/macos/Open LoL Companion.app`. Le prototype publié n'est pas remplacé ; aucune livraison GitHub effectuée. Les changements distants postérieurs au checkout ont été repérés par fetch, sans mélanger automatiquement leurs sources au travail local non committé.

## Deuxième lot — draft réelle et fenêtre sans défilement global (#12)

Plan avant code : ajouter un contrat de draft public normalisé en Rust/TypeScript, abonner le watcher à la session de sélection, lire l'état initial si déjà en draft, puis afficher les cartes et bans dans un composant dédié. Tests rouges puis verts du parsing, des états prépick/pick, des côtés, de la suppression et de la déconnexion. Aucun identifiant joueur ni intention adverse non verrouillée ne traverse la frontière Rust.

Source de schéma : [Swagger extrait du client 26.16](https://github.com/KebsCS/lcu-and-riotclient-api/blob/main/lcu/swagger.json) et code Riot extrait, https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-champ-select/global/default/rcp-fe-lol-champ-select.js (16.19, 29 septembre 2026). `player.team` : 1 bleu, 2 rouge ; joueur local par `localPlayerCellId`, jamais par index de tableau. Les champs retenus sont explicitement listés dans le parseur. Modes à banc/rerolls ou compositions hors deux équipes de cinq : état non pris en charge, sans déduire des camps imaginaires.

Contrainte ajoutée par Louison : la fenêtre native (minimum 960×600) n'a pas de barre de défilement globale. Les zones partagent l'espace disponible ; seul le contenu d'une liste/panneau qui s'allonge peut défiler intérieurement. Vérification de l'accueil, draft et réglages à 960×600 et 1366×768, dont réglages anglais et messages d'indisponibilité. Un navigateur plus étroit conserve un repli accessible.


### Preuves du deuxième lot — 1er octobre 2026

- `pnpm test` : 85 tests réussis (51 desktop, 5 shared, 29 Rust), dont lecture de session 5v5, intentions adverses masquées, prépick/verrouillage, côtés, bans, chrono, suppression et déconnexion. `pnpm lint` et build Tauri macOS debug réussis.
- Recette macOS dans le bundle Tauri avec **faux serveur LCU local et données synthétiques** : connexion alors qu'une draft est déjà active, camps inversés selon le côté, prépick Ahri puis verrouillage, transition automatique vers Partie, suppression de session, déconnexion puis reconnexion. Aucun de ces essais ne constitue une recette sur le client Riot réel.
- Recette navigateur : accueil/réglages et draft vide, ainsi que le composant DraftBoard rempli dans un banc temporaire, à 960×600 et 1366×768. Aucun débordement de document ; toutes les zones et les commandes des réglages restent visibles. Les petits panneaux de préparation sont compactés sous 680 px de haut. FR/EN et clair/sombre vérifiés. Banc retiré après contrôle.
- À la demande de Louison : icônes [Lucide React](https://lucide.dev/guide/react/getting-started) 1.49.0, licence ISC, imports explicites ; fin des pictogrammes SVG tracés manuellement dans le composant commun. Aucun test de logique ajouté pour ce remplacement purement visuel.
- 173 cartes officielles locales et noms FR/EN Data Dragon 16.19.1 ; pas de CDN à l'exécution. L'actualisation de ces données relève de #61. Champion absent du catalogue : état explicite sans image inventée.
- Revue indépendante : **OK**, aucun défaut concret restant identifié. Auto-revue : DTO minimal, positions/intention adverses masquées, pas de commande de sélection, pas d'identifiant joueur ni secret exposé ; changements limités au front commun et au suivi de draft.
- Incident de validation local : des métadonnées/bibliothèques compilées Rust dupliquées avec suffixe ` 2.rmeta` / ` 2.rlib` empêchaient l'édition de liens. Déplacées hors du cache et conservées dans le dossier de recette ; les commandes standard repassent sans modification des sources ni de l'environnement permanent.
- Restent hors de ce lot : glisser-déposer des postes (#12), recommandations, runes/objets et infobulles (#13/#61), mesures de performances en partie, recette LoL réelle Windows/macOS. #12 reste ouvert. Pas de commit/push ni de modification du site publié.

Le faux serveur, son lockfile et le binaire de recette ont été arrêtés/retirés ; le serveur Vite local préexistant sur 1421 reste disponible. Le Mac s’est verrouillé en fin de recette : la toute dernière build (icônes finales) n’a pas été rouverte en natif, mais son front a été contrôlé dans le navigateur.

## Troisième lot — runes équipées et catalogue d'objets (#13)

Plan avant code :

1. Réutiliser le normaliseur #61, désormais fusionné, pour produire un catalogue local FR/EN versionné de runes, fragments et objets avec illustrations. Aucun second normaliseur et aucun serveur requis à l'exécution.
2. Lire la page de runes courante dans le cœur Rust et suivre ses événements ; ne projeter que les arbres, choix et états utiles, sans nom de page ni identifiant de compte. Tester les données invalides, les fragments répétés, suppression, reconnexion et sérialisation.
3. Remplacer les panneaux vides par les arbres complets avec choix équipés mis en avant, un catalogue d'objets recherchable et leurs composants. Infobulles personnalisées au survol/focus. Aucun import de runes, achat ni recommandation inventée ; le catalogue est explicitement distinct d'un build conseillé.
4. Garder les panneaux utilisables à 960×600, tester FR/EN et clair/sombre, exécuter tests/lint/build et revue indépendante. Les builds statistiques, contexte matchup, sorts et imports restent les sous-lots suivants de #13/#14–16.

Synchronisation : `origin/main` a été avancé par fast-forward jusqu'à `6d68432` (catalogue #61), puis le travail local a été réappliqué. Les entrées de changelog et exports partagés ont été conservés des deux côtés ; aucune livraison Git n'a été créée.

### Preuves du troisième lot — 1er octobre 2026

- Contrat `RunePage` Rust/TypeScript : arbres, neuf identifiants ordonnés (fragments répétés conservés), validité, page temporaire et modifications signalées par LoL. Source : [Swagger extrait du client](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/refs/heads/main/lcu/swagger.json), modules Riot Collections/Champion Select et socket distribués via CommunityDragon. Lecture `GET /lol-perks/v1/currentpage`, abonnement WAMP global `OnJsonApiEvent`, distribution par URI. Aucune écriture LCU. Les noms/IDs de page et de compte ne sont pas projetés.
- Catalogue statique #61 réutilisé dans un export développeur, pas dans les dépendances runtime du desktop : 424 fiches par langue et 405 icônes locales. Manifestes, sources et empreintes contrôlés. Voir `apps/desktop/public/game-data/README.md` pour la régénération et les limites du filtre carte 11.
- Arbres complets, ordre source et choix équipés différenciés, bouton de retour après exploration, vue agrandie. Objets recherchables sans accents, composants navigables, statistiques avec unités et descriptions en texte seul. Les arbres inconnus restent indisponibles et les variables non résolues ne sont pas affichées. Fiches modales accessibles au clavier, Échap et retour du focus ; infobulle unique au survol/focus.
- TDD observé sur la projection Rust, les mappings et états de catalogue, l'ordre source des runes, les arbres inconnus et les unités. `pnpm test` vert : 66 tests desktop, 7 shared ; les suites Rust signalent 287 succès et 2 tests ignorés. Attention : les tests d'intégration PostgreSQL qui se retirent sans `OLC_TEST_DATABASE_URL` ne constituent pas une recette de base de données ; aucune base dédiée n'a été configurée pour ce lot. Exporteur : 4 tests ciblés verts. `pnpm lint` sans erreur, build Vite et bundle Tauri macOS debug réussis.
- Recette navigateur : 960×600 et 1366×768, FR/EN, sombre/clair, recherche « trinite » et « ward », objet → composant → fermeture, dialogue agrandi → fiche → fermeture séparée. Dimensions de document égales au viewport ; les contenus longs défilent dans leurs panneaux. Aucune erreur/alerte console relevée. Le banc à données synthétiques a été retiré des sources après contrôle.
- Recette dans le bundle macOS : chargement du catalogue embarqué sans serveur, recherche et fiche d'objet ; avec faux serveur LCU local, lecture de la page équipée au démarrage, mise à jour Domination → Précision, suppression, déconnexion, reconnexion et retour des neuf choix. Ce sont des données synthétiques, pas une validation sur le client Riot réel.
- Revue stricte indépendante : **OK** après correction des arbres de repli trompeurs, descriptions à variables et ordre des runes ; retour de focus confirmé dans le navigateur. Auto-revue : pas de réseau externe au rendu, imports Lucide explicites, aucun secret ni action automatisée de jeu, pas de polling React.
- Incidents locaux résolus sans perte : trois copies TS identiques au snapshot antérieur, 274 copies d'assets suffixées « 2 » non référencées, et 39 bibliothèques Rust numérotées ont été déplacées hors compilation et conservées avec manifestes dans le dossier de recette. Aucun nettoyage général du Mac. Le stash `49dd74344f07ec0eb0aedca67c57262a703b4ff2` reste disponible comme sauvegarde du front avant synchronisation.
- #13 reste ouvert : branchement aux statistiques de builds, choix matchup/pro, édition/imports #14–16, mise à jour automatique du catalogue et recette LoL réelle Windows (Louison)/macOS (Matthieu) restent hors de ce sous-lot. Pas de commit, push ni republication du prototype.

Fichiers de recette et sauvegardes : `/Users/louisonraymond/Documents/Codex/2026-09-30/sal/work/preparation/`. Capture du composant rempli, explicitement synthétique : `outputs/front-runes-objets-recette.png` dans ce même dossier de tâche `sal`. Le serveur de développement préexistant sur 1421 est conservé.

Le faux serveur et l'instance de recette ont été arrêtés, leur lockfile retiré. L'application a été relancée sans variable de recette et reste ouverte sur Draft en attente du vrai client ; le catalogue demeure consultable.

## Quatrième lot — builds statistiques dans la préparation (#13)

Plan avant code (1er octobre 2026) :

1. Ajouter un client Rust de l’API #19 et une commande Tauri de lecture : configuration serveur/jeton exclusivement Rust, HTTPS (HTTP autorisé uniquement sur loopback), erreurs publiques bornées, délais et tailles bornés, pagination complète cohérente. Contrat partagé TS explicite ; aucune donnée LCU transmise.
2. Déterminer le champion depuis le prépick/pick local et le rôle depuis sa position, permettre une consultation manuelle clairement indiquée. Filtres statistiques explicites patch/région/file/rôle/rang, changement de contexte sans anciennes données. Garder les runes équipées distinctes des runes statistiques.
3. Afficher les variantes observées de runes, achats (composants/consommables conservés), inventaire final, sorts et ordre des points investis. Fréquence, effectif, victoire uniquement si publiée, provenance et date consultables. Ne pas fusionner ces catégories indépendantes en un build conjoint, ni qualifier une fréquence de meilleur choix. Ajouter les assets officiels nécessaires via le normaliseur existant.
4. TDD sur transport, périmètre/pagination, seuils et transformations ; recette locale avec le vrai serveur et une base synthétique dédiée si aucun endpoint réel n’est disponible. Contrôle visuel FR/EN, sombre/clair, 960×600 et 1366×768 ; tests, lint, build et revue indépendante.

Hors de ce lot : filtres matchup/pro absents de #19, édition/imports #14–16, déploiement/authentification publique, collecte réelle et validation League Windows/macOS. Absence actuelle de configuration API sur ce Mac constatée ; aucune statistique fictive ne sera embarquée dans l’app.


### Preuves du quatrième lot — 1er octobre 2026

- Client `olc-build-client` : origine HTTPS ou loopback, TLS public distinct du
  LCU, jeton exclusivement Rust, redirections refusées, erreurs normalisées,
  pagination complète avec périmètre/publication identiques, plafond de réponse,
  4 appels simultanés et délai global de 30 secondes incluant l’attente. Les
  réponses obsolètes ne peuvent pas réapparaître après un changement de contexte.
- TDD : transport/authentification, absence de secret dans les erreurs,
  incohérence de pages, file d’attente de cinq appels, modèle de runes/compétences
  et seuils, changement de contexte, chargement des fiches champion et distinction
  « aucun objet observé » / « catégorie absente » observés rouges puis verts.
- Actifs officiels Data Dragon 16.19.1 : 458 fiches de base par langue, 173
  ensembles champion + cinq compétences chargés à la demande, 1 477 PNG ;
  1 496 fiches par langue au total. Manifestes et sources conservés. Les
  compétences ne gonflent pas le catalogue chargé à l’ouverture (environ 85 Ko
  par champion contre environ 5,6 Mo pour le catalogue de base). Aucun CDN runtime.
- Recette native : **vrai bundle macOS → commande Tauri → vrai serveur API #19 →
  PostgreSQL**, avec 12 variantes et une draft **entièrement synthétiques** dans
  une base dédiée. Ahri → Jinx sans données → Ahri, fiches objet → composant,
  coupure du serveur → erreur explicite, catalogue/runes équipées toujours
  disponibles. Ce contrôle ne valide ni le vrai client Riot, ni la qualité des
  statistiques, ni la connexion utilisateur publique.
- Recette navigateur du même front : 960×600 et 1366×768 sans défilement global,
  FR/EN et sombre/clair, variantes, effectifs sous le seuil, filtres sans données,
  retour au champion suivi, provenance, arbres agrandis, dialogues et icônes.
  À petite hauteur les panneaux défilent intérieurement ; les arbres peuvent
  s’ouvrir en grand. Les filtres sont compactés et le panneau d’achats conserve
  une zone de lecture, plutôt qu’une bande d’icônes tronquées.
- Auto-revue : catégories indépendantes, taux API en pourcentage, échantillons
  faibles masqués, valeurs absentes non inventées, achats répétés/consommables
  préservés, ordre de compétences présenté comme points investis et non niveaux,
  paire de sorts sans affectation D/F. Aucune écriture LCU ni automatisation.
- Revue stricte indépendante : **OK** après correction de deux défauts (séquence
  vide confondue avec absence de données et cinquième appel rejeté plutôt que mis
  en attente). Dernier ajustement de hauteur contrôlé visuellement.
- Limites conservées : endpoint/accès de production non configurés, données
  réelles et client Riot Windows/macOS non testés, performances en partie non
  mesurées. Matchup/pro, imports #14–16 et clôture de #13 restent hors de ce lot.
  Le prototype publié reste inchangé ; aucun commit/push.

- Validation finale : `pnpm test` **389 tests réussis** (88 desktop, 7 shared,
  294 Rust), 2 tests live optionnels ignorés ; les tests PostgreSQL ont réellement
  tourné avec PostgreSQL local 15.15. Les 10 tests de l’exporteur sont également
  verts. `pnpm lint`, `git diff --check` et build Tauri macOS debug réussis
  (bundle 62,68 Mio). La contre-revue finale CSS/doc est **OK**.
- Le bundle final a été relancé **sans configuration de recette** : absence du
  vrai client et de l’API correctement signalée ; catalogue local consultable.
  Les serveurs/faux client, le jeton et la base dédiés à la recette ont été
  supprimés/arrêtés. Vite préexistant sur 1421 et l’application normale restent
  disponibles. Captures et journaux sont conservés dans le dossier de tâche.


## Cinquième lot — édition et import manuel des runes (#13 / #14)

Plan avant code (1er octobre 2026), parcours accepté avec Louison :

1. Réutiliser localement la PR #57 de Matthieu au SHA
   `91fd2ce9d401007a1baed41443a567f4f2bdb31b`, sans fusion distante ni remplacement
   des fichiers contenant le front courant. Conserver le contrat d’import et ses
   validations du catalogue LCU ; garder la provenance de cette dépendance.
2. Éditer les arbres complets dans le panneau : choix par ligne, deux secondaires
   de lignes différentes, fragments, changement d’arbre, réinitialisation.
   Partir de la variante consultée ou de la page équipée ; les modifications
   locales restent distinctes des statistiques et de la page réellement équipée.
3. Import manuel explicite, un appel à la fois, erreurs FR/EN et aucune relance
   automatique. Vérifier le contexte courant côté Rust après acquisition du
   verrou ; restreindre ce premier parcours aux drafts Faille 400/420/440, mode
   CLASSIC vérifié dans la vraie session (pas dans le filtre statistique). Bloquer
   un autre champion consulté. Un HTTP accepté et une page équipée concordante
   sont deux états différents.
4. TDD des règles d’édition, protections contre appels obsolètes et validations
   de contexte ; recette du vrai bundle avec faux client isolé uniquement,
   fenêtres 960×600 et 1366×768, FR/EN, clair/sombre. Tests, lint, build, revue.

Hors de ce lot : imports automatiques, sorts/items #15–16, autres modes,
configuration API de production, fusion/publication Git et actions sur le vrai
client. Les recettes Rust partielles de la PR ne remplacent pas celles du nouveau
parcours UI sur les deux OS.

### Réalisation et vérifications du cinquième lot

- Les arbres complets deviennent éditables, y compris dans la vue agrandie :
  quatre choix principaux, deux secondaires de lignes différentes et trois
  fragments. Un changement d’arbre retire les choix incompatibles. La
  réinitialisation reprend la source ; un changement de variante ou de contexte
  repart de celle-ci. Pas de sauvegarde persistante des brouillons dans ce lot.
- Les runes observées, personnalisées et équipées sont distinguées. Les métriques
  d’une variante disparaissent dès qu’elle est modifiée. L’import explicite
  utilise la commande protégée `import_draft_runes` ; pas d’import automatique,
  ni d’écriture vers le client réel pendant la recette. Les pages personnelles
  sont préservées selon le contrat [imports-client](imports-client.md).
- TDD : échecs observés avant implémentation pour les règles d’édition, le
  contrôleur d’import et le succès d’une draft valide côté Rust. Les tests
  couvrent aussi réponses périmées, double clic, phases/champion/mode invalides.
- `pnpm test` avec PostgreSQL de recette : **109 tests desktop, 9 shared et
  318 Rust réussis**, deux tests réels optionnels ignorés. `pnpm lint` sans
  erreur. Bundle Tauri debug macOS reconstruit.
- Recette native avec faux client isolé : POST puis PUT sur le même ID, aucune
  altération de la page personnelle, contrôles désactivés pendant l’import,
  acceptation HTTP distincte de la confirmation via watcher, refus de mode
  réel malgré Solo/Duo dans les filtres, rejet HTTP, déconnexion/reconnexion.
- Recette navigateur avec bridge synthétique, vrais composants/assets : FR/EN,
  sombre/clair, 1366×768 et 960×600, modifications/reset, remplacement des
  secondaires, invalidation après changement d’arbre, changement de variante,
  ouverture/Échap/retour du focus. Pas de barre de défilement du document.
  En petite hauteur, le contenu du panneau défile localement et les arbres
  s’ouvrent en grand ; les commandes ne compriment plus les runes à une bande.
- Revue indépendante du code : **OK**, 48 tests front ciblés, 20 tests Rust
  d’import et typage relancés par le relecteur. Auto-revue : contrat #14 préservé,
  source PR #57 documentée, aucun secret ni réseau système dans React.

Limites : vérification native **synthétique**, pas une recette League réelle.
Le nouveau parcours UI doit être testé par Louison sur Windows et sur macOS
avec League. Catalogue local et client peuvent diverger selon leur patch : le
client fait autorité et refuse l’import incompatible. Les autres modes,
imports automatiques/sorts/objets, données matchup/pro et configuration API de
production restent hors de ce lot. #13 et #14 restent ouverts.

Preuves : dossier de tâche `sal/work/rune-editor` (baseline, échecs TDD, tests,
lint, build, faux client et scénario), captures dans `sal/outputs`. Aucun commit,
push ou changement du prototype publié. Les serveurs/fichiers temporaires de
recette sont retirés du projet après vérification ; Vite préexistant sur 1421
reste disponible.

Une régression trouvée par la revue finale a été corrigée avec test rouge puis
vert : explorer un arbre sans page, puis créer sa page et terminer l’édition
ne réaffiche plus l’ancien arbre exploré. Le parcours est aussi rejoué dans
l’interface. Verdict indépendant final : **OK** après cette correction.


## Sixième lot — sorts d’invocateur (#13 / #15)

Plan avant code conservé dans `sal/work/spell-editor/PLAN.md` : reprendre #58,
projeter les sorts locaux, éditeur D/F manuel, garde de draft, confirmation,
TDD et recette native. Hors lot : objets #16, imports automatiques, compagnon,
fusion ou publication Git. PR source : `baa355496d16bb0ac502dc20044893e8618db4a1`.

- Paire communautaire ou équipée modifiable ; emplacements D/F, icônes officielles,
  fiches existantes, neuf sorts CLASSIC et Flash mémorisé après choix explicite.
  Les statistiques disparaissent lorsqu’une paire différente est choisie.
- Mode équipé utilisable sans API de statistiques. Les trois colonnes gardent
  runes, objets et sorts accessibles ; les sorts passent avant le détail des
  compétences, repliable. Contenus longs à défilement local.
- Contrôleur d’import commun aux runes/sorts, avec instances indépendantes ;
  double clic bloqué et réponse d’un ancien contexte ignorée. Garde Rust
  partagée, moteur de Matthieu conservé. Confirmation par `localSpells`.
- TDD modèle D/F, paire sans Flash, permutation sans doublon, données invalides,
  projection locale, garde champion/mode/phase. La revue a trouvé puis fait
  corriger un choix implicite de Flash quand seul l’autre sort était modifié
  (test rouge puis vert).
- Contrôles navigateur : 1280×720 et 960×620, clair/sombre, paire modifiée,
  métriques masquées, préférence conservée au rechargement, import accepté
  sans fausse confirmation. Aucun débordement global à 960×620.
- Bundle natif macOS contrôlé avec faux client isolé : envoi PATCH, confirmation
  différée puis watcher, refus de mode. Voir [imports-client](imports-client.md).
- Revue indépendante : **OK avec réserves de recette réelle** ; 12 tests TS
  ciblés, 14 Rust sorts/gardes, diff check et provenance PR58 vérifiés.

État à la fin du sixième lot : la recette réelle UI macOS/Windows restait ouverte et le transfert vers Test Windows était à organiser. La passe nocturne ci-dessous actualise cet état. Aucun commit/push/publication.
Les temporaires de recette sont conservés hors dépôt dans `sal/work/spell-editor`
(pour reproduction) ; le HTML de recette est retiré du projet et le faux client
arrêté. Vite préexistant sur 1421 reste disponible.

Recette Windows à exécuter sur ce snapshot : vérifier D/F, paire sans Flash, refus de doublons, import et confirmation visuelle, retour dans l’app, changement de champion, sortie/reconnexion de draft et conservation des sorts/skin initiaux après restauration. Le compte et les pages personnelles ne doivent pas être modifiés hors de ces actions explicites.

Validation globale finale : `pnpm test` (122 desktop, 10 shared, 333 Rust ; deux tests réels optionnels ignorés), `pnpm lint` et build Tauri macOS. Une erreur de typage temporaire du compagnon parallèle a été signalée dans son chat avec autorisation de Louison et corrigée par ce lot avant revalidation globale. Contrôle FR/EN effectué à 960×620.

## Navigation de préparation — passe nocturne (#8, #13)

Le champion consulté, le poste, la région, la file, le rang et l’onglet restent en mémoire pendant les allers-retours entre pages. Une nouvelle sélection après le lobby reprend le champion local ; une coupure/reconnexion seule conserve la consultation. Aucun import n’est déclenché par la navigation. Les dialogues et modifications locales non importées des éditeurs restent éphémères lorsqu’on quitte la page ; la configuration réellement équipée est relue depuis League.

## Finition de draft — passe nocturne (#12, #13)

Les cartes de champions connus sont consultables au clic ou au clavier. Le contour suit le champion affiché dans la préparation ; « Vous » et l’état de verrouillage désignent le joueur local. Consulter un autre champion ne change aucun choix dans League. Le retour au prépick/pick porte désormais le nom du champion réel. Les infobulles partagent le style des objets/runes, chargent le passif officiel au survol/focus et présentent jusqu’à trois statistiques lisibles quand présentes. Les emplacements vides restent non interactifs.

## Recette de la passe nocturne — 1er octobre 2026

Plan et périmètre : [plan-front-nuit](plan-front-nuit.md). Résultats, preuves et limites : [recette dédiée](recettes/2026-10-01-front-nuit.md). Les statistiques restent celles des populations Solo/Flex/Normal Draft choisies ; accepter une partie personnalisée ne crée pas une population statistique personnalisée. Les listes natives Windows reçoivent des couleurs explicites d’options, cohérentes avec le thème.


## Champions desktop — 2 octobre 2026 (#13)

La navigation expose une bibliothèque locale de 173 champions (patch 16.19.1). La
recherche globale cherche les pages et les champions (noms FR/EN, accents et
ponctuation tolérés, nom exact prioritaire), avec Ctrl/Cmd+K, flèches, Entrée et Échap.
Les cartes ouvrent une fiche latérale, absente avant sélection, avec trois onglets :
compétences, builds communautaires et catalogue libre. Les flèches/Home/End naviguent
entre les onglets. Les fenêtres 960×600 et 1280×800 utilisent des défilements internes.

`AppState.champions` conserve recherche, classe, tri, champion, onglet, filtres des
builds et position de la grille pendant la session. Cet état est séparé de
`preparation` : consulter un champion ne change pas le pick suivi en draft. Un retour
après navigation ou passage automatique en draft retrouve la consultation. Les
variantes de builds et explorations temporaires de catalogue ne sont pas persistées.

L’index compact `public/game-data/champion-directory.json` est dérivé des fiches
normalisées locales ; après publication du catalogue, exécuter
`node apps/desktop/scripts/build-champion-directory.mjs` depuis la racine. Ce script
refuse les versions hétérogènes ou les classes inconnues. Les classes officielles ne
sont pas des postes : le filtre de grille est une **classe**, le filtre des builds
est un **poste**. Aucune liste de postes n’est inventée.

Les compétences, runes, objets et images restent consultables avec League fermé.
Les builds passent par le contrat Rust `community_builds` existant et requièrent une
API configurée ; le navigateur affiche explicitement cette limite. Aucun build de
remplacement fictif. Le mode `readOnly` des panneaux masque les imports et ne monte
pas le réglage Flash. Chaque catégorie conserve ses variantes, effectifs et taux
propres ; aucune promesse de build global optimal.

Maîtrise, matchs récents, recommandations personnalisées, profils de joueurs,
builds pro et matchups attendent leurs services dédiés. Le site #20 reste à Matthieu.
Recette et limites : [2026-10-02-champions-desktop.md](recettes/2026-10-02-champions-desktop.md).

## Joueurs et accueil — #64 (2 octobre 2026)

Le front réel Tauri dispose de la recherche Riot ID/région, des rangs Solo/Flex et
de l’historique public fourni par #19. L’état au niveau de l’app garde au maximum
le profil de l’accueil et le joueur consulté. Les réponses tardives sont ignorées.
Les listes défilent dans leurs panneaux ; ouvrir un champion puis revenir conserve
les parties chargées et la position. Le choix explicite de l’accueil survit aux
navigations et déconnexions LCU, puis est relu au lancement suivant.

Ce choix est un favori local, pas une connexion Riot vérifiée. Authentification,
multi-comptes, détection du compte LCU, amis et conseil personnalisé restent à faire.
Pas de modification du site #20. Recette :
[profils desktop](recettes/2026-10-02-joueurs-desktop.md).
