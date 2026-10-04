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
