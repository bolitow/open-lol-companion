# Catalogue desktop téléchargeable — #93

Le desktop conserve son catalogue embarqué pour démarrer sans serveur. Une API configurée peut fournir un instantané complet du patch lu dans League. Les catalogues FR/EN, index, fiches et images basculent ensemble ; les imports continuent de vérifier la compatibilité avec le patch client et le rapport statistique.

## Production et distribution

Depuis un export schéma 2 réalisé par le normaliseur existant :

```sh
cargo run -p olc-collector --example export_desktop_catalog -- --version 16.19.1 --output /chemin/export/catalog
cargo run -p olc-collector --example publish_desktop_catalog -- --input /chemin/export/catalog --output /chemin/artefacts
```

Remplacer la version d’exemple par la release choisie. Le second outil vérifie les empreintes de l’export, construit les index FR/EN et télécharge les artworks du skin de base depuis CommunityDragon au patch exact (quatre transports maximum, bornes et TLS vérifiés). Un artwork manquant interrompt la publication ; aucune icône étirée ne le remplace. La provenance figure dans `artwork-sources.json`, celle de la normalisation dans `catalog/sources.json`. La publication normalisée de l’API et l’export CLI sont deux produits distincts : l’identité de cet export couvre tous ses fichiers, sans prétendre être un `publication_id` PostgreSQL.

Configurer `OLC_DESKTOP_CATALOG_DIR=/chemin/artefacts` dans le processus API. Cette configuration est optionnelle ; sans elle les routes retournent une indisponibilité. Aucune clé Riot n’est nécessaire à la distribution. Les routes sont publiques comme les routes de données statiques existantes :

- `/v1/desktop-catalog/{version}/manifest` : manifeste schéma 1, ETag égal au `snapshot_id` entre guillemets ; revalidation 304.
- `/v1/desktop-catalog/snapshots/{snapshot_id}/{path}` : fichier appartenant à cet instantané, empreinte revérifiée, cache HTTP immuable.

`heads/{version}` ne change qu’après écriture et validation du paquet complet. Une republication de la même release change d’identité si un fichier change. Les anciens instantanés serveur restent disponibles : leur purge relève de l’exploitation et doit respecter les clients en reprise et la rétention des patchs. Aucun déploiement n’est effectué par ce lot.

## Reprise ciblée des segments #107

Le 6 octobre 2026, l’embarqué 16.19.1 a été repris pour les seuls `tooltip_segments`. Les 1 496 records
par langue, les 316 objets de la carte 11 et leurs recettes, les cartes et les 1 477 icônes restent
identiques après retrait du champ ajouté. Les augments et la sélection élargie #116/#118 ne sont pas
intégrés par cette reprise. Le normaliseur 3 décrit les champs reprojetés depuis les sources originales ;
les autres champs gardent le résultat du normaliseur 1. `manifest.json` et `sources.json` déclarent
ce périmètre dans `reprojection` (`fields`, `base_normalizer_version`, `other_records_preserved`).

L’outil développeur sans PostgreSQL, clé Riot ou téléchargement consomme 348 **vrais** documents
Data Dragon : 173 fiches champion détaillées et `summoner.json` par langue. Les JSON raw doivent
être nommés `<source_id>.json` dans un cache local, acquis depuis les URL épinglées de `sources.json`
avec TLS validé et des bornes. Une source normalisée n’est jamais réutilisée comme raw. L’outil vérifie
le contexte public exact, recalcule chaque ID `make_source`, reprojette avec `project_sources`, puis
copie uniquement le champ si le tooltip et sa provenance sont exactement ceux de l’origine. Une
source divergente, absente ou un nombre de tooltips inattendu interrompt la génération.

Les deux dossiers de sortie doivent être absents ou vides, avec leur parent déjà présent, distincts
des origines et non imbriqués entre eux. Leurs parents sont résolus avant comparaison, y compris
les alias `..` et symlinks d’ancêtres. Par prudence, les sorties différant seulement par la casse
sont aussi refusées sur tous les OS ; leurs chemins doivent être Unicode. Les chemins/symlinks
et lectures sont bornés (16 Mio par JSON, 2 Mio par PNG, 256 Mio
pour l’export, 1 000 sources maximum). Depuis des caches déjà acquis :

```sh
cargo run -p olc-collector --example reproject_tooltip_segments -- \
  --input /chemin/export-original \
  --raw-dir /chemin/raw-public \
  --output /chemin/work/catalog-candidat \
  --snapshot-input /chemin/snapshot-original \
  --snapshot-output /chemin/work/snapshot-candidat
```

`--expected-tooltips` vaut 726 par langue par défaut. Après validation et mesure du candidat,
actualiser explicitement l’embarqué tout en gardant l’export précédent. Le snapshot est dérivé dans
un **nouveau** cache avec les primitives existantes ; tous les fichiers hors périmètre réutilisent leurs
octets/empreintes. Le nouveau `heads/16.19.1` est écrit en dernier, après validation du snapshot et
écriture complète de l’export. Un échec I/O laisse éventuellement des fichiers candidats pour
diagnostic, sans publier ce pointeur ; il faut reprendre dans deux nouvelles sorties vides. Les
anciennes données et leur pointeur ne sont jamais modifiés. Aucun artwork
ou cosmétique n’est réacquis, et cet outil ne publie ni l’API PostgreSQL ni un déploiement externe.

Recette 16.19.1 : 726 tooltips par langue (692 compétences et 34 sorts), concaténation exacte,
provenance inchangée, 2 005 fichiers du snapshot dont seuls 350 JSON de catalogue changent.
L’export passe de 56 089 960 à 57 085 679 octets (+1,78 %), dont **995 454 octets** dus au seul champ
segments (FR 522 594, EN 472 860) ; les métadonnées et la sérialisation expliquent le reste. Le gzip
indicatif, calculé par document, ajoute 54 921 octets. La tête d’origine est `a257ff0b…a40e97`, le
candidat scratch `835f22f7…2d70544`. Ce candidat n’est pas déployé/configuré dans l’application.

Le type est consommé par occurrence dans les formules et textes correspondants ; descriptions
courtes distinctes et champs invalides restent neutres. La migration de l’interprète BIN, les autres
balises et la recette visuelle/native Windows restent ouverts. Les tests de l’outil font partie de
`pnpm test`, grâce à sa déclaration `[[example]] test = true`.

## Installation native

Le cœur Rust sélectionne la plus haute release du patch majeur.mineur effectivement lu du client. Il n’interprète pas le suffixe de build LCU comme une release Data Dragon, n’installe pas de patch futur et ne suit pas arbitrairement le realm EUW en l’absence du client.

Le stockage est sous le répertoire local de données Tauri, dans `desktop-catalog-v1`. Les fichiers sont adressés par SHA-256 ; un manifeste préparé sert de journal de reprise. Seuls les fichiers absents ou corrompus sont redemandés. Un fichier partiellement reçu est recommencé, sans reprise HTTP par plage. Les bornes sont de 2 Mio pour le manifeste, 16 Mio par JSON, 8 Mio par image, 10 000 fichiers et 256 Mio par instantané ; elles s’appliquent aussi aux flux sans Content-Length. Chaque requête expire après 30 secondes.

Un verrou interprocessus protège la synchronisation. Les lectures consultent des fichiers immuables et ne sont pas bloquées pendant les téléchargements. Le pointeur actif est remplacé avec un fichier temporaire sur le même volume, après vérification des octets et du contrat métier. L’initialisation est attendue par toutes les fenêtres ; le remplacement de la configuration API et le commit du catalogue partagent un verrou.

L’actif, le précédent et le téléchargement en cours sont conservés. Les instantanés encore épinglés par une application restent aussi disponibles : actuellement, les épingles sont gardées jusqu’à la fermeture du processus. Une app ouverte pendant plusieurs patchs conserve donc plus de deux instantanés ; le nettoyage redevient effectif à la synchronisation suivante après fermeture des lecteurs. Cela privilégie la continuité des fiches et des images sur une suppression prématurée. Les objets non référencés sont supprimés uniquement sous verrou ; un nettoyage impossible n’empêche pas de consulter la dernière version saine.

Au démarrage, un paquet endommagé fait revenir au précédent sain puis à l’embarqué, avec indication de récupération. Le front ne reçoit aucun chemin disque ni secret : `catalog_state`, `catalog_sync`, `catalog_read`, événement `catalog-state`. Les images passent par le protocole local `catalog`, limité aux images déclarées et vérifiées de l’instantané. Les index et détails d’une lecture gardent le même snapshot ; une bascule invalide les anciennes réponses et remonte les lecteurs sans perdre la navigation.

Revalidation : démarrage/session, publication serveur, retour au premier plan après cinq minutes, puis toutes les trente minutes seulement quand la fenêtre est visible. Les demandes sont mutualisées. Le mode navigateur conserve les ressources embarquées et ne simule pas la persistance native.

## Limites et recette

`cosmetics.json` contient les identifiants d’icônes de profil, les séries FR/EN et les chemins des miniatures/splashs de skins. Ces métadonnées versionnées sont vérifiées et activées avec le reste du catalogue. Les images cosmétiques ne gonflent pas le paquet obligatoire : le protocole local `cosmetic` les résout par snapshot/type/identifiant déclaré et Rust les télécharge à la demande depuis les CDN publics au patch explicite, sans redirection ni URL libre venant de l’interface.

Le cache indépendant `cosmetic-images-v1` est limité à 128 Mio et 4 096 entrées, avec 8 Mio maximum par image. MIME, signature et empreinte locale sont vérifiés ; les écritures sont atomiques et les entrées les moins récemment utilisées sont évincées. Le transport est sérialisé pour dédupliquer les demandes et limiter la charge. L’empreinte locale détecte une corruption du cache, mais n’est pas une empreinte des octets d’image annoncée par le serveur : la provenance repose sur HTTPS et l’URL du CDN versionné. Une correction upstream à URL identique attend l’éviction du cache pour être relue.

Un ancien paquet sans `cosmetics.json` reste accepté : les séries embarquées et les images historiques servent de repli. Les URL de skins issues du client sont alors épinglées au patch LCU ; si ce patch n’est pas lisible, l’image est absente plutôt que suivie via `latest`. Les vidéos restent dans leur catalogue distinct. Les effets présents dans les fiches gardent leur provenance et leur statut ; aucune valeur manquante n’est inventée. Le ticket #93 reste ouvert pour les manifestes régionaux et les validations restantes.

Recette Windows manuelle encore à effectuer. Tests de verrou/écriture et chemins partagés couvrent les deux plateformes au niveau du code ; cela ne remplace pas une exécution Windows. Les logs de recette locale Mac et les fixtures de cette passe sont conservés dans `work/catalog-93/`, sans secret.

Les primitives utilisées sont documentées par [fs2 FileExt](https://docs.rs/fs2/0.4.3/fs2/trait.FileExt.html) et [tempfile NamedTempFile::persist](https://docs.rs/tempfile/latest/tempfile/struct.NamedTempFile.html#method.persist). Le fichier est synchronisé avant remplacement ; le répertoire est également synchronisé sur Unix. La résistance aux coupures matérielles dépend du système de fichiers.

## Intégration des branches

Le raccord des consommateurs Collection/avatars dépend des PR #77, #174, #175 et du catalogue #191 (lui-même basé sur #190). La branche d’intégration conserve les deux ensembles ; aucune de ces PR n’est fusionnée automatiquement sur main. Les données de possession/souhaits continuent de venir du client et du stockage par compte, sans collecte cosmétique privée côté serveur.

Recette d’intégration : [macOS et limites de validation](recette-catalogue-cosmetique-93.md).
