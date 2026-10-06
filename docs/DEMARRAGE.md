# Démarrer le développement

Ce guide vous amène de zéro à l'app qui tourne sur votre machine, sous Windows ou macOS.

## 1. Prérequis

| Outil | Version | Installation |
| --- | --- | --- |
| Node.js | 20 ou plus (22 conseillé) | [nodejs.org](https://nodejs.org) |
| pnpm | 10 | `corepack enable` (fourni avec Node) |
| Rust | stable | [rustup.rs](https://rustup.rs) |
| League of Legends | à jour | Pour tester la connexion au client (facultatif au début) |

**Windows** : installez aussi les [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (charge de travail « Développement Desktop en C++ »). WebView2 est déjà présent sur Windows 10 et 11.

**macOS** : `xcode-select --install`.

Détail complet : [prérequis Tauri 2](https://v2.tauri.app/start/prerequisites/).

## 2. Installer et lancer

```bash
git clone https://github.com/bolitow/open-lol-companion.git
cd open-lol-companion
pnpm install
pnpm dev          # lance l'app desktop (Tauri) avec rechargement à chaud
```

Le premier lancement compile les dépendances Rust (quelques minutes), les suivants sont rapides.

Autres commandes :

| Commande | Effet |
| --- | --- |
| `pnpm dev:ui` | Interface seule dans le navigateur (http://localhost:1420), sans Rust |
| `pnpm test` | Tests TypeScript + tests Rust du connecteur LCU, du client de builds, du collecteur, de l'API et du desktop Tauri |
| `pnpm typecheck` | Vérification des types |
| `pnpm lint` | Typage + `cargo fmt --check` + `cargo clippy` (doit être à 0 avant une PR) |
| `pnpm format` | Formate le code Rust |
| `pnpm build:desktop` | Installeur de production pour votre OS |

## 3. Vérifier la connexion au client LoL

Lancez le client League of Legends puis l'app : l'écran d'accueil passe au vert (« Client League of Legends détecté ») en moins de 3 secondes.

Sans le jeu, vous pouvez simuler un client en pointant l'app vers un faux lockfile :

```bash
echo "LeagueClient:1234:50000:motdepasse:https" > /tmp/lockfile
OLC_LOL_LOCKFILE=/tmp/lockfile pnpm dev
```

(Sous PowerShell : `$env:OLC_LOL_LOCKFILE="C:\temp\lockfile"; pnpm dev`.)

## 3 bis. Collecteur Riot (backend, facultatif)

Le collecteur `services/collector` récupère des parties via l'API Riot et les stocke dans PostgreSQL. Il ne dépend ni de l'app ni du client LoL.

1. PostgreSQL : `docker compose -f services/collector/docker-compose.yml up -d` (ou une installation locale).
2. Copiez `services/collector/.env.example` en `.env` à la racine, renseignez `RIOT_API_KEY` (clé de développement du Developer Portal, valable 24 h) et `DATABASE_URL`.
3. `cargo run -p olc-collector --release -- sync-static` pour synchroniser les deux patches récents en FR/EN.
4. `cargo run -p olc-collector --release -- run --target 50 --collect-ranks` pour un petit essai, puis `report <n°>`.

Tests PostgreSQL : définissez `OLC_TEST_DATABASE_URL` (ex. `postgres://postgres:postgres@localhost:5432/postgres`) ; sans elle, `pnpm test` les ignore. Détails : [`services/collector/README.md`](../services/collector/README.md).

Pour les statistiques (#18), seule `DATABASE_URL` est nécessaire :
`cargo run -p olc-collector --release -- aggregate --json` utilise les deux patches
du cache. `--patches 16.19,16.18` fixe une sélection ; `--all-stored` prend tout le
stockage. Les agrégats séparent plateforme/file/rôle/rang observé et exposent taux,
effectifs, builds et timelines. Les données brutes de joueurs restent privées.

Pour les fiches détaillées du jeu (#61), après `sync-static` :
`cargo run -p olc-collector --release -- catalog --patch-count 2 --json`.
Cette commande enrichit les données par CommunityDragon et publie des fiches
versionnées avec provenance et couverture, sans clé Riot. Reconstruction depuis
les archives : `catalog --rebuild <publication_id> --json` (sans réseau).
Options, filtres et limites : [référentiel du jeu](catalogue-jeu.md).

Pour les vidéos de skins (#47), `scripts/catalog-skin-spotlights.py` prépare
un catalogue candidat et un rapport de couverture depuis les métadonnées
publiques, sans télécharger les vidéos. Paramètres, cache, arrêt sur refus
fournisseur et revue avant copie : [maintenance SkinSpotlights](collection-skins.md#relancer-une-maintenance).

Pour une campagne multirégion bornée :
`cargo run -p olc-collector --release -- campaign --hours 24` ; reprendre avec
`campaign-resume <id>`. La campagne collecte Solo/Duo et Flex par défaut ; les autres files
(ARAM, Swiftplay, Arena…) s'ajoutent avec `--queues`, par exemple `--queues 420,440,450`. Dans un autre terminal, `aggregate --sync-static --watch`
vérifie les statiques et recalcule chaque heure (par lots modifiés ; `--full` force
le recalcul complet). La campagne respecte les quotas, une échéance persistée et des
tranches de 15 minutes par plateforme. Une clé de développement peut expirer avant
la fin. Aucun superviseur ni service permanent n'est installé. Voir
[le contrat et les options](../services/collector/README.md).

Rétention des données personnelles (#99) : `cargo run -p olc-collector --release -- purge --watch`
supprime les parties brutes (et le cache des parties exclues) après 90 jours et retire PUUID, Riot ID, icône et niveau de compte après 30 jours
(valeurs proposées, réglables par `OLC_RETENTION_*`). Détails dans le
[README du collecteur](../services/collector/README.md#rétention-des-données-personnelles-99).

## 3 ter. API interne (backend, facultatif)

Compléter le `.env` existant avec `services/api/.env.example`, notamment un secret
JWT aléatoire d'au moins 32 octets. Utiliser la même `DATABASE_URL` que le collecteur.
`cargo run -p olc-api -- serve` écoute sur `127.0.0.1:3030` ;
`cargo run -p olc-api -- token --subject development` émet un jeton de lecture
depuis le serveur. Profils : clé Riot backend requise ; statiques/agrégats sans clé.
HTTPS/WSS via proxy en production. Les migrations s'appliquent au démarrage ;
redémarrer le collecteur avec cette version pour partager les quotas.

Routes, variables, cache, WebSocket et PowerShell : [contrat API](../services/api/README.md).

### Relier la préparation desktop aux builds (#13)

Le cœur Rust a besoin d’une URL (origine du serveur, sans chemin) et d’un jeton de
lecture émis par le serveur. Deux sources, dans cet ordre (#98) :

1. `OLC_API_URL` et `OLC_API_TOKEN`, **toutes deux** présentes dans l’environnement du
   processus qui lance `pnpm dev` (raccordement développeur, prioritaire) ;
2. sinon **Réglages → Accès à l’API** : l’URL et le jeton saisis y sont validés par le
   cœur Rust puis gardés dans le trousseau du système (Keychain sous macOS,
   Gestionnaire d’identifiants sous Windows ; service
   `io.github.bolitow.openlolcompanion`, compte `api-access`). Le changement
   s’applique sans redémarrage (builds, profils, canal des publications) ; « Retirer
   le jeton » supprime l’entrée. Le jeton n’est ni journalisé, ni renvoyé à
   l’interface, ni réaffiché après saisie ; seule l’URL est relue.

Le front ne garde aucun jeton ; ne pas utiliser un préfixe `VITE_`, ne pas embarquer
de jeton dans le bundle et ne pas transmettre le secret JWT du serveur ni une clé
Riot au desktop. Les variables ne sont pas chargées automatiquement depuis le `.env`
du backend. Sous macOS, une version recompilée ou non signée peut déclencher une
demande d’accès au trousseau au démarrage : la fenêtre reste utilisable, les builds
et profils attendent la réponse.

- Développement local : `OLC_API_URL=http://127.0.0.1:3030` ; définir le jeton
  temporaire dans le terminal de lancement, puis `pnpm dev`. Sous PowerShell,
  utiliser les variables `$env:OLC_API_URL` et `$env:OLC_API_TOKEN`.
- Serveur distant : HTTPS obligatoire. HTTP n’est accepté que pour une adresse
  IP loopback littérale ; redirections, identifiants dans l’URL, chemins et
  paramètres sont refusés.
- Publier des agrégats du patch du catalogue desktop avec le collecteur avant
  d’attendre des résultats. Le champion/prépick et le poste locaux sont suivis ;
  région, file et rang sont des filtres statistiques explicites, pas une
  identification automatique du compte. Le patch est celui du catalogue local.
- Sans configuration, jeton valide ou agrégats correspondants, l’interface
  affiche la cause ou l’état vide. Le catalogue local et les runes équipées
  restent accessibles. L’aperçu navigateur ne dispose pas de ce transport natif.

La saisie dans les réglages permet d’utiliser une app installée lancée depuis le
Finder ou le menu Démarrer, mais la distribution des jetons reste manuelle : aucun
endpoint public d’émission ni rafraîchissement n’existe encore, et un jeton
`olc-api token` expire au plus tard après 24 h (#98). Les
catégories sont indépendantes, triées par fréquence, et ne constituent ni un
build conjoint gagnant ni une recommandation matchup/pro. Voir
[le suivi de validation](integration-front.md).

**Annonce des publications (#125).** Avec les mêmes `OLC_API_URL` et `OLC_API_TOKEN`,
le cœur Rust ouvre aussi `/v1/ws` (`wss` pour HTTPS, `ws` pour le loopback), envoie le
jeton dans le premier message (jamais dans l’URL) et reçoit les dates de publication
des statistiques et des données statiques. Une coupure déclenche une reconnexion avec
attente doublée de 1 s à 60 s ; un jeton refusé ou expiré (fermeture `1008`, ou 401/403 d’un proxy
d’authentification) arrête les tentatives
(état `unauthorized` : enregistrer un nouveau jeton dans les réglages relance
l’écoute sans redémarrage, ou relancer l’app avec de nouvelles variables). L’état est lu par la
commande `publication_state` et émis avec l’événement `publication-state`.

La recette de transport TLS se lance avec `cargo test -p olc-build-client wss_` :
véritable serveur WSS sur un port loopback éphémère, authentification dans la
première frame et deux publications reçues. L'autorité synthétique n'est approuvée
que par le client du test positif ; le test négatif garde les autorités de
production et refuse le certificat avant toute authentification. Aucun trousseau
système ni dépendance OpenSSL à l'exécution. Ce contrôle ne remplace pas la
[recette application–API sur les deux OS](recettes/2026-10-06-publications-125.md).

## 4. Où coder quoi

Les commandes Tauri `import_runes`, `import_spells` et `import_items` sont décrites
dans le [contrat des imports client](imports-client.md), avec les types, les erreurs
FR/EN et la recette Windows/macOS à réaliser. Leur déclenchement dans
l'écran build appartient à #13.

| Dossier | Contenu | Langage |
| --- | --- | --- |
| `crates/lcu-connector` | Détection du client, identifiants, phases de jeu et imports | Rust |
| `apps/desktop/src-tauri` | Cœur de l'app : commandes appelées par l'interface, overlays, capture | Rust |
| `apps/desktop/src` | Interface de l'app | React + TypeScript |
| `packages/shared` | Types et utilitaires partagés (phases, Data Dragon) | TypeScript |
| `services/collector` | Collecte Riot multirégion, Data Dragon et agrégats PostgreSQL | Rust |
| `services/api` | REST/JWT, WebSocket, profils et cache statique | Rust |
| `apps/web` | Site Next.js : tierlist, page champion, profil, recherche Ctrl+K ([README](../apps/web/README.md)) | TypeScript |

Règle d'or : ce qui touche au système (fichiers, processus, réseau local, secrets) vit en Rust ; l'interface appelle des commandes Tauri (`invoke("…")`) et ne voit jamais de mot de passe.

## 5. Première contribution

0. Lisez [`AGENTS.md`](../AGENTS.md) : workflow, Definition of Done et règles bloquantes.
1. Prenez un ticket du [sprint en cours](sprint-1.md) ou étiqueté `good first issue`.
2. Branche `feat/…` ou `fix/…`, puis pull request vers `main`.
3. La CI tourne sur la PR (pas au push ni après la fusion) : Linux à chaque fois, Windows et macOS quand l'app, le connecteur ou `@olc/shared` changent (et pas en brouillon). Elle doit être verte.
   Un workflow planifié, [`patch-watch.yml`](../.github/workflows/patch-watch.yml), relance chaque jour la suite de tests sur Linux quand Data Dragon publie un nouveau patch LoL (aucun secret, aucun appel Riot authentifié). Il consigne le résultat dans une issue « Patch LoL X.Y : contrôle automatique » : ouverte si les tests échouent, fermée aussitôt s'ils passent. Pour retester un patch déjà tracé : Actions, « Contrôle à chaque patch LoL », *Run workflow*, option « force ». Les recettes LCU réelles sur Windows et macOS restent manuelles.

Et avant tout : relisez la section 2 du [cahier des charges](cahier-des-charges.md) sur la conformité Riot.

## 6. Essayer le prototype visuel de l’accueil

Le [prototype interactif isolé](prototype-accueil.md) se lance sans League of Legends :

```bash
pnpm --filter @olc/desktop dev --host 127.0.0.1 --port 1421
```

Ouvrir http://127.0.0.1:1421/prototype.html. Toutes ses données sont fictives ; l’écran LCU habituel reste inchangé.


### Démonstration partageable

Le prototype de conception est consultable sur [ChatGPT Sites](https://open-lol-companion-prototype.peon45.chatgpt.site), avec un [accès direct aux maquettes](https://open-lol-companion-prototype.peon45.chatgpt.site/#layouts). Données fictives, sans connexion au client LoL. Lancer « Démo draft » depuis l’accueil pour le parcours automatique. Ce site est une publication manuelle du prototype ; voir [le suivi de conception](prototype-accueil.md) pour ses limites et sa mise à jour.

## Front réel et référence visuelle

`pnpm dev` lance désormais le socle réel : accueil, draft, partie, bilan et réglages. Les événements du watcher pilotent la navigation ; les réglages ouverts ne sont pas interrompus. FR/EN, thème et mouvements sont mémorisés. Voir [le périmètre et le plan](integration-front.md).

`pnpm dev:ui` sert la même interface dans un navigateur, sans connexion native. Les panneaux métier sont explicitement indisponibles tant que leurs données ne sont pas raccordées. `/prototype.html` reste une référence séparée à données fictives, pas le front de production.

### Éditer et importer les runes (#13 / #14)

Dans **Draft**, les panneaux **Communauté** (API configurée) et **Runes équipées &
Catalogue** permettent de modifier les deux arbres et les fragments. **Modifier**
active l’édition ; **Réinitialiser** reprend la source courante ; **Importer dans
LoL** est toujours une action explicite. L’agrandissement conserve les choix.
Les statistiques de la variante ne sont plus affichées sur une page modifiée.

Le front utilise `import_draft_runes({ request: { championId, runes } })` ; Rust
vérifie le catalogue du client, puis sa session et son champion avant écriture.
Ce parcours concerne uniquement les drafts Faille 400/420/440, CLASSIC, carte 11.
Le filtre statistique de file ne remplace pas ce contrôle. Le navigateur permet
de consulter et d’éditer, mais n’écrit jamais dans LoL.

Un import accepté attend la confirmation du client avant d’être présenté comme
équipé. Après une coupure ou un délai dépassé, vérifier la page dans LoL avant
de relancer. Détails et limites : [contrat des imports](imports-client.md).

## Profils joueurs dans le desktop (#64)

La recherche globale accepte `Nom#TAG` avec région. La page **Joueurs** affiche le
profil et l’historique public paginé via les commandes Tauri `player_profile` et
`player_matches`, vers les routes existantes `/v1/profiles/{platform}/{name}/{tag}`
et `/matches?start=…&count=…`. Les mêmes variables **Rust uniquement**
`OLC_API_URL` et `OLC_API_TOKEN` que les builds configurent ce service. L’API doit
elle-même disposer de son service de profils Riot. Aucun nouveau secret n’est requis
par React. Une installation non configurée affiche un état explicite ; l’aperçu
navigateur ne remplace pas le transport Tauri.

**Utiliser pour l’accueil** mémorise uniquement région/nom/tag dans
`olc.app.home-player`, sans PUUID, jeton ou historique. **Retirer de l’accueil**
efface ce choix. Le compte reste affiché après fermeture de League ; consultation
d’un autre profil et navigation vers un champion ne le remplacent pas. Le favori
ne prouve pas la propriété du compte : synchronisation LCU et liaison multi-comptes
restent hors de ce lot. Les profils restent consultables client fermé si le service
est configuré ; aucune partie de League n’est lancée.

Historique : pages de 10, curseur conservé après erreur, dédoublonnage entre pages,
progression possible sur une page vide. Parties privées exclues par l’API. Les
rangs sont ceux du profil acquis à la date affichée, pas ceux au moment des matchs ;
aucune estimation de MMR. Les commandes HTTP sont limitées à 512 Kio, 60 s par
requête / 65 s au total, quatre appels simultanés partagés avec les builds.

Le [suivi du compte League actif](compte-actif.md) alimente automatiquement l’accueil. Le service de profils reste nécessaire pour ses statistiques, mais pas pour détecter son Riot ID local.

Les [réglages recherchables](reglages.md) regroupent thème, langue, animations et Flash D/F. Les préférences antérieures sont reprises ; les fonctions système du ticket #11 restent séparées.
La CI exécute aussi les tests desktop TypeScript et du client de builds Rust sur Linux/Windows/macOS. Les tests de l’exemple d’export du catalogue sont déclarés dans `services/collector/Cargo.toml` (`[[example]] test = true`) : ils tournent avec `cargo test -p olc-collector`, donc avec `pnpm test` et sur les trois OS de la CI.

### Réglages système (#11)

Le desktop utilise le tray Tauri2 et le plugin autostart2.7. Le lancement au démarrage n’est jamais activé par l’installation des dépendances ni par le lancement de développement : il faut utiliser le contrôle dédié. Les commandes `desktop_settings`, `set_desktop_setting` et `set_desktop_locale` restent internes au desktop. Le paramètre `--autostart` masque la fenêtre seulement si son tray est disponible.

Les tests de `olc-desktop-support` sont inclus dans `pnpm test` et dans la CI Windows/macOS/Linux. Ils ne modifient pas le démarrage du poste. Pour les vérifications manuelles de fermeture, réouverture et ouverture de session, suivre [la recette des réglages](reglages.md#recette-native-à-exécuter-sur-chaque-os).

La commande native `export_diagnostics` utilise le plugin dialog2.8 et produit un ZIP local (crate zip2.4, entrées non compressées et bornées). Aucun endpoint d’envoi ni variable d’environnement supplémentaire. Les tests utilisent uniquement des fichiers synthétiques ; le journal est limité à la session courante. Voir [contenu et limites de l’archive](reglages.md#export-local-des-diagnostics).
## Tester les imports au prépick (#63, partie client)

Dans **Réglages**, le panneau **Imports au prépick** permet
l'activation séparée des runes et objets. Les deux options sont initialement
désactivées. En développement, le minimum initial est **1 partie par variante** ;
en production, **100**. Ce minimum est réglable de 1 à 1 000 et mémorisé sur
l'appareil. Sélectionner un champion dans LoL suffit, avant même le verrouillage.
En personnalisée Faille, choisir aussi le **Poste en personnalisée** (par exemple
Support pour Bard) : les variantes proviennent alors des statistiques Solo/Duo.
Une seule page **Open LoL Companion** est réutilisée entre tous les champions ;
les pages personnelles et les sorts d'invocateur sont conservés.
Démarrage, méthode, limites et recette :
[imports automatiques dans le client](imports-automatiques.md).


### Messages d’accès et de version (#186)

Un refus HTTP 401/403 des builds ou profils, ou un refus du canal de publications,
affiche « Jeton refusé ou expiré » dans **Réglages → Accès à l’API**. Le statut ne
contient que l’indicateur `authorizationRejected`, l’origine, l’URL et un code
d’erreur de trousseau : aucun jeton ni corps de réponse. Un remplacement réussi
réinitialise le refus ; une réponse tardive de l’ancien client ne peut pas invalider
le nouveau. Une notification pendant la lecture des réglages provoque une relecture
unique, pas un polling. En développement, si les variables d’environnement sont
prioritaires, le message demande de les remplacer puis de relancer l’application.

Les profils distinguent le code API `riot_busy` (« Riot est occupé »), le quota
`rate_limited` et l’indisponibilité. Un 503 n’est considéré `riot_busy` que si son
corps borné à 4096 octets expose exactement ce code public.
Le patch du client est consultable séparément dans les réglages, au format public
(`26.19`) : voir [patch-client.md](patch-client.md).

### Contexte de préparation (#92)

La région du compte actif et la file de la draft alimentent la préparation et la fiche champion. En personnalisée, les statistiques restent celles de Solo/Duo ; le poste choisi dans les imports est partagé avec l’aperçu. Aucun poste Mid n’est inventé lorsque le poste manque : choisissez-le dans le filtre.

Un clic allié consulte son poste connu (cellule conservée pour les champions dupliqués en personnalisée). Un clic ennemi verrouillé choisit un matchup local sans remplacer votre champion ; l’absence de données de matchup est indiquée, les chiffres restent génériques. Aucun poste ni intention adverse n’est déduit.

Les filtres explicites restent pendant la navigation et les snapshots de la même draft. Une nouvelle draft reprend région/file réelles et votre champion. L’import automatique et l’aperçu partagent la requête ; consulter un autre champion suspend les nouveaux imports automatiques. Changer les filtres ne rejoue pas un import déjà envoyé : son statut conserve la population effectivement utilisée. Les imports manuels restent disponibles. Le défaut de rang fait l’objet de #180 ; le patch de requête reste #93.

### Rangs et population des builds (#105 / #180)

Les deux menus partagent Échantillon collecté (`ALL`), huit paliers Fer+ à Master+, puis les dix paliers simples. Le défaut lit uniquement le profil LCU du compte actif : Solo/Duo d’abord, Flex ensuite ; palier simple, Master+ pour Grand maître/Challenger, Émeraude+ sans palier fiable. La fraîcheur retenue localement est de 30 minutes ; l’expiration ne déclenche aucune requête de profil. Un choix explicite reste prioritaire pendant la session. En Normal et dans les autres files non classées, le rang effectif est ALL et le menu est désactivé ; revenir en classé restaure le choix explicite ou le défaut local.

Les imports attendent la fin de lecture du profil local pour ne pas envoyer un défaut transitoire. Un import déjà envoyé n’est pas rejoué lorsque le rang détecté ou les filtres changent. La vue en partie de l’application reprend le rang choisi ; la fenêtre d’overlay sans profil local utilise le repli Émeraude+ (aucune recherche de profil adverse).

La source des builds expose la répartition des participations classées du périmètre patch/région/file, tous rôles confondus, en effectifs et parts. Le badge « Échantillon surtout Master+ » suit exclusivement `high_elo_biased` fourni par le serveur. Aucun seuil de biais n’est calculé dans le desktop. Les anciens rapports sans répartition et les libellés de population futurs restent lisibles. La tierlist n’est pas encore présente dans l’application desktop ; le site existant n’est pas modifié par ce lot.

### Observations complémentaires des builds (#113)

Le transport desktop conserve le résumé du champion, les points de compétence, les événements d’objets et les compteurs de variantes omises du groupe demandé. Ces informations sont communes aux pages : elles ne sont conservées qu’une fois et un changement entre pages invalide la réponse. La limite de réponse reste 8 Mio par page, 1 000 variantes ; les événements respectent le plafond déclaré par le serveur (actuellement 2 000), avec une borne défensive client de 10 000 lignes. Aucun téléchargement de données brutes de partie.

Dans la préparation et la fiche champion : **Source & méthode** affiche le résumé et les variantes non publiées par catégorie ; les volets compétences et objets donnent accès aux observations chronologiques. Les points désignent l’ordre d’investissement, pas le niveau du champion. Le temps est une moyenne publiée, pas une recommandation. Les événements sont comptés par objet, type et minute, par pages de 24 lignes : ce ne sont ni des parties distinctes ni une médiane d’achèvement. Les événements rares (ventes notamment) peuvent être absents après plafonnement ; le plafond et les lignes non servies sont explicités.

Les anciens rapports restent lisibles. Un ancien total global de variantes omises sans ventilation par catégorie n’est jamais attribué au champion. Les statistiques détaillées de performance, matchups, splits et classement relèvent de #185. Le recalcul chiffré sur la base de collecte reste côté Matthieu ; les tests de pagination du client utilisent des réponses locales représentatives sur Mac, la recette native Windows reste à faire.

### Patch des statistiques (#93)

La préparation, les fiches champion et la vue en partie affichent le patch effectivement consulté, résolu depuis le client et le manifeste API. Un patch antérieur est signalé ; les imports communautaires sont suspendus tant que client, catalogue et données ne sont pas compatibles. La lecture est mutualisée et renouvelée sur sélection/publication, sans polling. Détails, replis et limites : [patch-client.md](patch-client.md).

Pour distribuer les mises à jour du catalogue desktop depuis votre API, voir [catalogue-desktop.md](catalogue-desktop.md). Sans configuration, le catalogue embarqué reste utilisable.
### Rechercher les vidéos encore manquantes

Pour rechercher aussi une présentation récente des anciens skins sans référence, utiliser
`--global-search-uncovered-skins` avec `scripts/catalog-skin-spotlights.py`.
Les résultats sont filtrés sur la chaîne officielle et restent partiels. Le batch
`scripts/update-skin-spotlights.py` accepte `--allow-title-variants` pour revalider
les titres annotés et anciens noms Riot sourcés ; sans cette option, il reste strict.
Voir [les règles et commandes de récupération](collection-skins.md#récupération-des-références-manquantes--3-octobre-2026).
