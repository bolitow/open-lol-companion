# Recette application–API et transport WSS — #125

## macOS, 6 octobre 2026

Application native Tauri issue de la recette #94 (base `c56291a` avec correctif
d'historique), SHA256 du binaire effectivement lancé :
`03c668ae5a04e0af2f02764d29537c8473144f852ba31223d611bece573aa472`.
Les fichiers de publication, d'accès API et de rafraîchissement du manifeste sont
inchangés entre cette base et `054bfec`. L'API a été recompilée sur `054bfec`.
Ces observations ne sont pas attribuées à un nouveau bundle de main.

Le processus natif reçoit un accès de lecture temporaire par les variables
`OLC_API_URL`/`OLC_API_TOKEN`. API locale Rust réelle, PostgreSQL 15.15 dans une
base de recette dédiée, aucune donnée Riot ni aucun accès production. Un proxy
loopback relève uniquement les heures et routes ; les profils sont masqués.
L'inspecteur WebKit du desktop appelle la vraie commande `publication_state`.

| Action | Observation |
| --- | --- |
| Démarrage avec accès valide | `connected`, révision 1, aucune version publiée |
| Insertion d'une date dans `champion_stats_snapshot` | `connected`, révision 2, `stats_version = 2026-10-06 08:30:00+00` |
| Publication reçue, sans action sur le front | Deux lectures `/v1/static/manifest` supplémentaires, depuis les consommateurs principal/overlay |
| Arrêt de l'API | `reconnecting`, révision 2 conservée |
| Service indisponible | Tentatives après environ 1, 2, 4, 8 et 16 secondes |
| Relance avec la même publication | `connected`, révision 2, même date ; aucun nouveau rafraîchissement pour cette version identique |
| Nouveau processus avec accès synthétique invalide | `unauthorized`, révision 0, message « Jeton refusé ou expiré » dans les réglages |
| Après refus | Une seule ouverture WebSocket ; aucune nouvelle tentative pendant plus de 90 secondes |

Limites : cette recette utilise WS loopback ; elle ne valide pas une terminaison
TLS de production. La relecture automatique du manifeste est observée ; le rendu
d'un build en draft et un import LCU ne sont pas revendiqués. Les données de
recette sont synthétiques et ne prouvent aucune qualité statistique.

Nettoyage : processus API/proxy/desktop de recette arrêtés, base temporaire
supprimée, aucun jeton enregistré dans le trousseau. Application relancée sans
configuration API de recette. Les préférences initiales et le compte conservé
sont préservés. Journaux locaux sous `work/ticket-verification-20261006/`.

## Windows

Recette native du 6 octobre sur
`acffccb793013b177073bce75f6ba2d552928dcd`, binaire SHA256
`D07F9E529AA967D1D687B6D60ADDBD61071429EDEF52F9E919F303A2AD3810C9`.
Les chemins de publication, d'authentification et d'invalidation sont inchangés
entre ce commit et `054bfec` ; l'affichage du patch public a évolué sans incidence
sur ce canal. PostgreSQL de recette et API isolés, authentification
synthétique en environnement du processus, session League préservée.

- Changement de `published_at` : trois relectures REST automatiques, HTTP 200.
- Coupure : tentatives WebSocket à environ 1, 2, 4, 8, 16 puis 32 secondes.
- Relance de l'API, puis nouvelle date : nouvelles relectures REST automatiques.
- Nouveau processus avec accès invalide : une seule ouverture WebSocket pendant
  94 secondes, sans nouvelle tentative.
- Ressources de recette nettoyées, aucune configuration persistante d'accès.

### Complément natif en développement : lecture exacte des états

La protection anti-collage DevTools empêchait la lecture directe de
`publication_state`. Elle a été conservée. Une page de recette temporaire charge
le vrai front `/src/main.tsx` et affiche uniquement les données publiques de la
commande et de l'événement `publication-state`, dans la vraie WebView Tauri.
Le fichier est hors production et a été retiré après recette.

Sources `acffccb`, binaire Tauri **dev** SHA256
`798CC8589247127A246D7F5A0C3632002853DD0AB36C9973322955EA263200D9`,
distinct du binaire embarqué précédent. Configuration standard du `devUrl`
vers `http://127.0.0.1:1420/recipe125.html`, sans changement des capabilities,
de CSP, du trousseau ou des règles TLS. Page SHA256
`4CC9537C3FB9EBB40A0BE655C6BC4BA76F71049A77312A7A5B6BA5F5DFC734BD`,
configuration SHA256
`79239D549A7C00AD5E9648D4E6C316CA555EAAFBADA0358E16D3B0CA7EFBC417`.

| Action | État réellement lu dans la WebView native |
| --- | --- |
| Connexion initiale | `connected`, révision 1 |
| Nouvelle publication | `connected`, révision 2, relectures REST automatiques |
| Coupure | `reconnecting`, révision 2 |
| Relance sans changer la publication | `connected`, révision 2 |
| Nouveau processus avec accès invalide | `unauthorized`, révision 0, version des statistiques absente |
| Après refus | Une seule ouverture WebSocket, aucune nouvelle tentative pendant 63 secondes |

Preuves locales : `outputs/windows-pr197-acffccb/qa125dev/native-states.json`,
`requests.jsonl`, `artifact-hashes.json`, configuration et compte rendu. Processus,
page, profil et PostgreSQL de recette nettoyés ; checkout propre ; binaire initial
restauré avec son empreinte, session League préservée. Aucun WSS natif Windows
revendiqué : son transport est couvert séparément par les tests Rust ci-dessous.

## Transport WSS automatisé

`cargo test -p olc-build-client wss_` exerce le client réel et un serveur Rustls
sur un port loopback éphémère. Le certificat et la clé de serveur sont des
fixtures synthétiques publiques, sans compte, service externe ou secret réel.

Le client positif approuve uniquement l'autorité de recette dans sa propre
configuration de test, sans changer le système ni la configuration distribuée.
L'URI `/v1/ws` et l'absence de jeton dans l'URI/en-tête sont contrôlées ; le serveur
lit l'authentification dans la première frame et émet deux publications. Le
client négatif garde les autorités publiques de production et le handshake
échoue avant l'authentification. Aucun vérificateur permissif n'est utilisé.

Les tests vérifient TLS et WebSocket ensemble ; ils ne remplacent pas une recette
du proxy HTTPS d'un déploiement. Les commandes complètes et la CI sont consignées
dans la PR de cette passe.
