# Changelog

Toutes les évolutions notables du projet. Format : [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versionnage [sémantique](https://semver.org/lang/fr/). Règles : [`rules/changelog.md`](rules/changelog.md).

## [Non publié]

### Ajouté

- Revue de conformité Riot de tous les overlays et projections (panneau en partie, projection Live Client, sélection des champions, bilan, prototype) : rapport daté ligne à ligne, cas à soumettre à Riot et fonctions refusées complétés dans `rules/conformite-riot.md`, test de non-fuite des identités de la projection de sélection et filtre défensif des bans adverses (seuls les bans annoncés par une action terminée d'une cellule adverse sont projetés). Décisions du 4 octobre : objets, sorts, niveau et score de vision adverses refusés ; différence d'or par équipe seulement à soumettre à Riot, affichage prévu ; % de victoire estimé de la draft affiché dès maintenant, désactivable, conservé dans la liste soumise à Riot (#30, suites d'interface : #187).

- Projection Live Client élargie par liste blanche : or courant, vision, mort/réapparition, niveaux de compétences, côté et poste du joueur local, totaux K/D/A et CS des équipes comme au tableau des scores, événements publics (objectifs, kills) sans nom de joueur ; aucune donnée adverse cachée. La dernière lecture est conservée en mémoire jusqu'au bilan d'après-partie puis purgée (#102).

- Collecteur : `olc-collector close-unserved-ranks` ferme les demandes de rang encore en attente pour des parties non classées (créées avant le filtrage par file), en simulation par défaut et avec `--apply` pour agir ; `campaign --queues` choisit les files d'une campagne (une exécution par plateforme et par file, budget et cible réparties) : Solo (420) et Flex (440) par défaut, les autres files (ARAM, Swiftplay, Arena…) sur demande en les listant, rangs coupés hors 0/420/440 (#90).
- Desktop : **Réglages → Accès à l’API** permet de saisir l’adresse du service et un jeton d’accès, validés par le cœur Rust puis gardés dans le trousseau du système (Keychain sous macOS, Gestionnaire d’identifiants sous Windows). Builds communautaires, profils joueurs et canal des publications basculent sans redémarrage ; le jeton n’est ni journalisé, ni renvoyé à l’interface, ni réaffiché, et peut être retiré. Les variables `OLC_API_URL` et `OLC_API_TOKEN` restent prioritaires pour le développement. Aucun endpoint public d’émission ni rafraîchissement de jeton : modèle de distribution à valider (#98).
- RGPD : rétention configurable des données personnelles du collecteur. `olc-collector purge` (ponctuel ou `--watch` horaire) supprime les parties brutes et le cache négatif `excluded_matches` après 90 jours et, après 30 jours, retire PUUID, Riot ID, icône et niveau de compte (`summonerLevel`) des participants des parties et timelines, supprime observations de rang, joueurs de départ et travaux des exécutions inactives ; valeurs par défaut proposées, à valider. L'API ajoute `POST /v1/privacy/export` et `POST /v1/privacy/erase` (PUUID dans le corps, réservés aux sujets de `OLC_API_PRIVACY_OPERATORS`, nouveau code `forbidden`), types alignés dans `@olc/shared`. CGU, politique de confidentialité et hébergement UE restent à traiter (#99).
- Desktop : **Réglages → Accès à l’API** permet de saisir l’adresse du service et un jeton d’accès, validés par le cœur Rust puis gardés dans le trousseau du système (Keychain sous macOS, Gestionnaire d’identifiants sous Windows). Builds communautaires, profils joueurs et canal des publications basculent sans redémarrage ; le jeton n’est ni journalisé, ni renvoyé à l’interface, ni réaffiché, et peut être retiré. Les variables `OLC_API_URL` et `OLC_API_TOKEN` restent prioritaires pour le développement. Aucun endpoint public d’émission ni rafraîchissement de jeton : modèle de distribution à valider (#98).
- RGPD : rétention configurable des données personnelles du collecteur. `olc-collector purge` (ponctuel ou `--watch` horaire) supprime les parties brutes après 90 jours et, après 30 jours, retire PUUID, Riot ID, icône et niveau de compte (`summonerLevel`) des participants des parties et timelines, supprime observations de rang, joueurs de départ et travaux des exécutions inactives ; valeurs par défaut proposées, à valider. L'API ajoute `POST /v1/privacy/export` et `POST /v1/privacy/erase` (PUUID dans le corps, réservés aux sujets de `OLC_API_PRIVACY_OPERATORS`, nouveau code `forbidden`), types alignés dans `@olc/shared`. CGU, politique de confidentialité et hébergement UE restent à traiter (#99).
- CI : un workflow planifié (`patch-watch.yml`) compare chaque jour la version courante de Data Dragon aux patchs déjà contrôlés et, pour un nouveau patch LoL, lance les tests sur Linux puis consigne le résultat dans une issue « Patch LoL X.Y : contrôle automatique » ; aucun secret ni appel Riot authentifié, relance manuelle possible avec l'option « force ». Les contrôles du catalogue exporté et du format de match-v5 sur le nouveau patch restent à faire (#121).
- Profils de l'API : chaque rang Solo/Flex expose désormais victoires, défaites et drapeaux league-v4 (série, vétéran, nouveau dans le palier, inactif), optionnels et jamais déduits ; types Rust (API, client) et `@olc/shared` alignés par un JSON de référence testé des trois côtés ; README de l'API corrigé sur le peak, non fourni pour un tiers. La partie client LoL et interface reste à faire (#96).
- Annonce automatique des publications : le cœur Rust du desktop se connecte au canal WebSocket `/v1/ws` de l'API (jeton dans le premier message, jamais dans l'URL), se reconnecte avec un repli de 1 s à 60 s et s'arrête si le jeton est refusé ou expiré (fermeture `1008` du serveur, ou réponse 401/403 d'un proxy d'authentification au handshake). L'état (`revision`, connexion, dernière publication) est lisible par la commande `publication_state` et émis avec l'événement `publication-state`, typé dans `@olc/shared` ; l'interface n'est pas encore branchée dessus (#125).
- Quota Riot partagé : l'API des profils dispose d'une réserve de 20 % de chaque fenêtre (4 appels/s, 20 par 2 min) que le collecteur ne consomme plus, ce qui réduit l'attente d'une recherche de profil pendant une rafale de collecte sans la supprimer : la réserve est consommable, ne protège ni d'un 429 Riot ni d'un outil tiers sur la même clé, et n'existe pas pour une fenêtre d'un seul appel ; le plafond global de la clé reste intact. Nouveau code d'erreur `riot_busy` (503) quand aucun créneau n'est obtenu en 20 s, distinct de `unavailable` et du refus immédiat `rate_limited` (emplacements Riot, HTTP ou WebSocket pleins, ou 429 Riot propagé) ; la priorité est un paramètre obligatoire du transport coordonné, et les tests d'intégration du quota partagé utilisent des fenêtres de 60 s et vérifient par comptage d'appels, sans comparer de durées mesurées : les seules attentes chronométrées (50 à 200 ms) sont des vérifications « ne passe pas », qu'une CI lente ne peut pas faire échouer (#122).

- Desktop : commande `client_patch` qui lit la version du jeu installée dans le client LoL (`/lol-patch/v1/game-version`) et en tire le patch `majeur.mineur`, avec son type partagé ; l'interface ne l'utilise pas encore (#93, partie cœur Rust).
- Catalogue : l'infobulle des compétences et des sorts d'invocateur garde le type de dégâts (physique, magique, brut) sous forme de segments typés `tooltip_segments`, lu dans les balises de Data Dragon et non dans la formulation, donc valable dans toutes les langues ; le texte brut `tooltip` ne change pas, la regex du desktop reste à supprimer (#107).
- Catalogue : augments Arena et Mayhem en données statiques (noms FR/EN, description, rareté, icône), exposés par l'API du référentiel et l'export desktop ; sans statistique, popularité ni tier, catalogue publié non régénéré (#118).
- Catalogue : les surcharges de valeurs par mode des objets (ARAM, Arena, URF…) sont interprétées en effets `cdragon_parameters:{mode}` avec provenance, sans toucher aux valeurs de base ; types partagés correspondants (#116).
- Collecteur : commande `campaign-queues` (ARAM, Swiftplay et Arena par défaut) avec une cible et un budget d'appels par plateforme et par file, et `campaign-report` pour suivre les parties retenues face à ces cibles ; les files inconnues sont refusées au lancement (#97).
- Rapport d’intégration des réglages et des recettes, avec contrôle en lecture seule de la collecte EUW, des agrégats et de leur publication API (#11, #18, #63).

- Bilan de la recette multirégion : 8 601 nouvelles parties, quinze plateformes, arrêt sur refus de clé et limites de couverture/statistiques documentés (#18).
- Recette fonctionnelle macOS du panneau acceptée par Matthieu sur sa configuration : affichage, interactions et raccourci confirmés, capture et versions consignées ; mesures chiffrées et autres configurations restent à compléter (#22).
- Recette Live macOS avec valeurs CS réellement reçues, passage Mel → Yone et fixture anonymisée ; tests des lectures interrompues, coupure/reprise et remise à zéro entre parties (#23).
- Import des sorts d’invocateur les plus joués dès le prépick, sur option séparée,
  avec préférence Flash D/F partagée avec l’import manuel et confirmation des
  deux emplacements dans le client (#63).
- Recette macOS des imports : runes confirmées par le joueur, set d’Ahri relu
  dans le client et suivi de la conservation des pages personnelles (#63).

- Amis du compte LoL connecté dans l’accueil, présence, accès au profil si identité complète, états FR/EN et rafraîchissement local indépendant du quota Riot (#71).
- Profil, rangs et historique du compte actif lus dans le client LoL, source affichée, personnalisées incluses, pagination sans mélange avec le service public et actualisation en fin de partie (#64, #65).

- Données locales en partie (champion, niveau, K/D/A, CS et temps), contexte de draft conservé et builds consultatifs avec effectifs ; lecture bornée et annulation des réponses périmées (#23, #63).
- Panneau natif transparent sans prise de focus, masqué hors jeu actif, réglages/aperçu/raccourci, placement relatif ; NSPanel et Liquid Glass Apple sur macOS 26+, vibrance native en repli, moteur Windows conditionnel (#22). Recettes LoL réelles macOS et Windows à compléter avant clôture.

- Export local de diagnostics : aperçu FR/EN, journal technique de session borné et résumé facultatif de logs League choisis par le joueur ; ZIP sans identités, secrets ni lignes brutes, annulation et erreurs explicites (#11).

- Réglages système natifs : fermeture dans la barre système, menu FR/EN et lancement à l’ouverture de session sur choix explicite ; état relu depuis l’OS, annulation et repli visible sans tray (#11).

- Réglages desktop : recherche locale FR/EN par synonymes, modification depuis les résultats, annulation et sauvegarde des préférences ; Flash D/F partagé avec la draft et les imports automatiques activés (#11, #63).
- Couverture CI des tests desktop, client de builds et exporteur de catalogue ; rattrapage du compte actif sans bloquer les événements de phase et annulation à la déconnexion (#65, #8).

- Imports runes/objets au prépick du champion, activables séparément : variante valide la plus jouée au poste réel, seuil réglable (1 en dev, 100 en production), effectifs et taux observés ; garde de draft/file/poste juste avant écriture, confirmation et anti-doublons. Personnalisées Faille prises en charge avec poste choisi et source Solo/Duo explicite. Une seule page de runes réutilisée pour tous les champions, sans attendre le verrouillage ni l'identifiant de partie Riot. Sans overlay (#63, partie client) ; sorts ajoutés sur option séparée le 3 octobre.

- Accueil desktop synchronisé au compte League actif : changement automatique, identité conservée hors connexion, consultation des autres joueurs indépendante et état explicite quand le compte ou les statistiques sont indisponibles (#65).

- Commande `import_runes` : validation selon le catalogue du client, secondaires de lignes distinctes et remplacement de la page réservée à l'app (#14).
- Commande `import_spells` : import en sélection des champions avec Flash sur D/F, sans modifier le skin ni ajouter Flash à un build qui ne le contient pas (#15).
- Commande `import_items` : set prioritaire par champion/carte, conversion des objets Larme vers leurs formes achetables et conservation des sets personnels ; contrats partagés et erreurs FR/EN pour les trois imports (#16).
- Synchronisation des imports #14–16 avec `main`, en conservant les gardes du desktop et les recettes des deux OS.

- Joueurs desktop : recherche Riot ID/région, profil et rangs officiels, historique paginé, accès au champion joué et retour conservé ; compte favori mémorisé pour l’accueil, transport Rust et erreurs FR/EN (#64).

- Champions desktop : bibliothèque illustrée FR/EN, recherche globale au clavier, filtre par classe, fiche compétences/builds/catalogue et contexte conservé au retour ; consultation indépendante de la draft et sans import (#13).

- Draft : cartes consultables au clavier/clic, passif au survol, retour nommé vers le pick local et statistiques dans les infobulles (#12, #13).

- Navigation : champion consulté et filtres de préparation conservés au retour, sans réimport ; suivi du pick local repris à une nouvelle draft (#8, #13).

- Draft et imports manuels de runes/sorts étendus aux personnalisées Faille identifiées, avec équipes incomplètes et bots ; autres cartes/modes encore refusés (#12–15).

- Préparation : import manuel de la variante d’objets affichée, ordre et composants conservés, moteur #16 réutilisé sans toucher les sets personnels (#13, #16).

- Préparation : choix et import manuel des deux sorts d’invocateur, position de Flash D/F mémorisée, confirmation depuis League et garde du contexte de draft ; commandes accessibles aussi sans statistiques (#13, #15).

- Compagnon animé dans l’accueil : salut, repos et transformation au clic ; feu WebGL lié à sa silhouette, arrêt hors écran et respect des mouvements réduits. Première validation en rouge sur thème sombre (#1). Accueil limité à une fois par session, gestes accélérés avec anticipation, repos plus discret et feu ascendant sans contour uniforme.

- Préparation : édition des arbres de runes et fragments, réinitialisation et import manuel dans LoL ; contexte de draft et champion revalidés, confirmation de la page équipée, pages personnelles préservées (#13, #14).

- Préparation : builds communautaires reliés à l’API via Rust, filtres champion/poste/région/file/rang, variantes de runes et d’achats, compétences et sorts officiels ; effectifs, seuils et provenance visibles, sans données fictives embarquées ni import automatique (#13).

- Draft : runes équipées suivies depuis LoL, arbres complets consultables et vue agrandie ; catalogue local FR/EN d’objets, composants et infobulles, sans import ni build fictif (#13).

- Référentiel de jeu FR/EN : objets et statistiques enrichies par CommunityDragon, champions/compétences, runes/fragments et catalogues ; sources archivées, couverture explicite, reconstruction hors ligne et API de recherche/diff par patch (#61, sous-ticket de #18).
- API interne Rust/Axum : tierlist et builds filtrés, profils par Riot ID actuel et historique paginé, accès JWT, notifications WebSocket et statiques FR/EN revalidables par CDN (#19).
- Quotas Riot PostgreSQL partagés entre l'API et le collecteur, y compris après annulation d'un appel ou réponse 429 (#19).

- Agrégats par patch, région, file, rôle et rang observé : winrate, pickrate, bans, tiers avec seuils, builds, objets, sorts, runes et chronologie des compétences ; publication atomique ponctuelle ou horaire (#18).
- Synchronisation Data Dragon FR/EN pour les patches récents : champions standard et Classic, compétences et catalogues, cache versionné réutilisable conservé en cas d'échec (#18).
- Collecte sur les 15 plateformes Riot et toutes les files accessibles, rangs Iron à Challenger, observations Solo/Flex des participants et campagne reprenable jusqu'à 24 heures (#18).
- Draft réelle en lecture seule : picks/prépicks, bans, chrono et côtés bleu/rouge issus du client ; cartes officielles locales et projection Rust sans identité ni intentions adverses (#12).

- App réelle : socle visuel sombre/clair, navigation et réglages FR/EN persistants ; suivi des phases via le watcher Rust et un snapshot Tauri versionné, sans faux profils ni builds. Les vues métier restent en attente de leurs données (#8, #21 ; préparation #12/#13).

- Prototype : parcours draft simulé (bans, picks, aperçu/prépick, verrouillage, chargement et arrivée en partie), transitions réduisibles, liens directs et galerie conservant 19 wireframes Figma ; publication partageable sur ChatGPT Sites (#4).

- Prototype d’accueil interactif séparé (`/prototype.html`) : thèmes sombre/clair, ouverture animée, recherche clavier, historique et scénarios fictifs de session, réglages FR/EN mémorisés ; tests du modèle et guide de prise en main (#4).

- Règles de développement communes aux contributeurs et aux assistants de code : `AGENTS.md` et dossier `rules/` (workflow, Definition of Done, conformité Riot, revue, documentation).
- Commandes `pnpm lint` (typage, `cargo fmt`, `cargo clippy`) et `pnpm format`, vérifiées en CI.
- Connecteur LCU : client HTTPS, WebSocket WAMP et reconnexion automatique au client League of Legends, avec les événements `connected`, `disconnected` et `phaseChanged` pour l'app (#7).
- Collecteur Riot API (`services/collector`, prototype) : joueurs de départ league-v4, parties Ranked Solo/Duo EUW et timelines match-v5 stockées dans PostgreSQL, gestionnaire de quotas Riot, arrêt et reprise sans doublon, bilan de collecte (#17).

### Modifié

- Cahier des charges et règles de conformité Riot recoupés sur les textes d'origine (Developer API Policy, Game Policy et politiques générales du Developer Portal) : suppression du bouton Lock, des premades en sélection, des tiers et taux d'augments et d'objets Arena ; refus explicite des cooldowns adverses, camps non vus, or adverse exact, conseils tactiques en temps réel et MMR estimé ; cas non tranchés suspendus jusqu'à décision écrite de Riot (#79).

- Collecteur : les détails de parties sont réservés avant les demandes de rang, et une partie téléchargée puis exclue du périmètre est mémorisée (faits seulement, table `excluded_matches`) pour ne pas être retéléchargée par une autre exécution, puis purgée par `purge` après la durée de rétention des parties brutes (90 jours par défaut) ; budget d'appels Riot mieux employé (#90).
- Export du catalogue desktop : conserve aussi les objets propres à l'ARAM (carte 12) et à l'Arena (carte 30), avec le décompte par carte dans le manifeste ; la publication versionnée n'est pas encore régénérée (#116).
- Export du catalogue desktop : les tests de l'exemple `export_desktop_catalog` sont déclarés testables (`[[example]] test = true`) et tournent donc avec `pnpm test` et sur les trois OS de la CI ; l'étape dédiée Linux disparaît (#116).
- Export du catalogue desktop : quand le repli `all_items` se déclenche (carte illisible), l'export journalise la raison et, par carte, le nombre d'objets à disponibilité illisible, aussi consigné dans le manifeste (champ optionnel `unreadable_by_map`, reflété dans `@olc/shared`) ; le seuil de déclenchement ne change pas (#116).
- Catalogue : la lecture des paramètres `mDataValues` d'un objet et de ses surcharges de mode partage une seule fonction, sans changement de comportement (#116).
- Agrégats : une partie dont la file n'est pas identifiée (710, 3130, identifiants inconnus) est exclue sous la raison `unknown_queue` au lieu d'être agrégée sans contrôle de format ; ses données brutes restent en base (#97).
- Réglages : panneau « Imports au prépick » déplacé dans les paramètres, avec son état d'activation ; le moteur reste actif pendant la navigation (#63).

- Documents de planification `docs/superpowers/` exclus du suivi Git (#18).
- Front réel : panneaux adaptés à la hauteur de fenêtre dès 960×600, sans défilement global ; icônes Lucide à la place des tracés manuels, réglages compacts et textes FR/EN.

- Prototype draft : cartes de champions illustrées, branches principale/secondaire de runes avec toutes leurs options et sélection accentuée, noms accessibles au clavier, suppression du faux statut prépick après ban (#4).

- Prototype draft/build : équipes compactes, champion et matchup illustrés, runes et chemin d’objets nommés, plan de jeu et scaling conservés après verrouillage ; direction graphique validée par Louison et Matthieu (#4).

- Prototype : sphères lumineuses traversant trois profondeurs, reflets de surface synchronisés et transitions de combustion entre pages/phases ; ambiance suspendue pendant ces transitions et omise sans panneau cible (#4).

- Prototype d’accueil : hiérarchie compacte, matières translucides et courbe d’objectif ; combustion WebGL turbulente multicolore, rythme ralenti à 2,75 secondes, seconde vague entre les panneaux, reflet de surface et bords altérés fixes sur la carte de session ; ambiance persistante à trois profondeurs (braises arrière, crépitements sur les cartes, étincelles avant), budget raster partagé et pause dans un onglet masqué ; ouverture interrompue à la première interaction, repli statique sans WebGL (#4).

- CI allégée : elle ne tourne plus qu'à l'ouverture et à la mise à jour des PR (plus au push ni après fusion) ; builds Windows et macOS seulement si l'app, le connecteur ou `@olc/shared` changent, et pas en brouillon ; caches d'une PR supprimés à sa fermeture ; plus d'artefact gitleaks.
- `CLAUDE.md` renvoie désormais vers `AGENTS.md`.
- `pnpm test` et la CI lancent aussi les tests du collecteur (PostgreSQL 17 en CI Linux, compilation et tests sous Windows et macOS quand le collecteur change) (#17).
- Code Rust formaté avec `cargo fmt`.

### Corrigé

- Intégration des réglages système avec les recettes macOS : permissions des commandes système limitées à la fenêtre principale, préférence Flash unique pour les trois écrans, panneau overlay accessible dans la recherche, amis/Live/raccourci et fermeture native conservés (#11, #22, #23, #63).

- Compteur CS en partie : précision de la source LoL indiquée en FR/EN ; tests garantissant que chaque valeur reçue est transmise sans arrondi ni seuil de dix (#23).

- Imports : garde de sélection courante, invalidation des reçus lors des échanges et changements de poste manuel, rejet des réponses tardives ; synchronisation du poste avant import (#63).
- Alerte de raccourci d’overlay conservée après sauvegarde ou changement de langue (#22).

- Overlay macOS : reconnaissance de la fenêtre LoL sans bordure au niveau natif 1000 et panneau placé au-dessus ; le cache suit aussi les changements de niveau. Affichage/masquage Windows explicitement non activants ; tests du moteur ajoutés à la commande globale et aux jobs desktop (#22).
- Test de sauvegardes concurrentes adapté aux refus de remplacement observés sous Windows, tout en exigeant des lectures intègres et le nettoyage des temporaires (#11).

- API builds/tierlist : sélection indexée des morceaux par population avant lecture du JSON, avec migration des instantanés déjà publiés et réponses inchangées (#19).
- Listes natives Windows : fond et texte des options accordés au thème pour éviter les libellés clairs sur fond blanc (#13).

- Agrégats : publication atomique des grandes listes en morceaux bornés, pour dépasser la limite d’un objet JSONB unique tout en conservant le schéma JSON public (#18).

- Collecteur : une ouverture de transaction PostgreSQL annulée ne peut plus rendre au pool une connexion encore dans une transaction ; les connexions saines restent réutilisées (#18).
- Collecteur : un travail refusé par Riot (401/403) ne peut plus être réservé une seconde fois pendant la suspension ; ce refus prime sur les limites de durée ou de budget (#18).
- Collecteur : une exécution dont le dernier appel consomme exactement le budget est terminée normalement, au lieu d'exiger une reprise inutile (#17).

## [0.1.0] — 2026-09-29

### Ajouté

- Monorepo pnpm + Cargo, CI Linux / Windows / macOS avec recherche de secrets (#5).
- Squelette de l'app desktop Tauri 2 + React qui indique si le client League of Legends est détecté.
- Crate `lcu-connector` : lecture du lockfile sur Windows et macOS, secours par les arguments du processus, authentification, phases de jeu (#7).
- Paquet `@olc/shared` : types partagés et utilitaires Data Dragon.
- Cahier des charges, guide de démarrage et plan du sprint 1.
