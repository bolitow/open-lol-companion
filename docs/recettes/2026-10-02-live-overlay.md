# Recette locale du 2 octobre 2026 — #22 / #23 / #63

## Environnement

- Branche `codex/live-client-overlay`, issue de `main` 9468560 ; PR #69 déjà fusionnée.
- Machine de développement macOS 27.0 (26A428), Tauri 2.12, tauri-nspanel 2.1.0.
- Windows non disponible localement : recette attribuée à Louison dans les tickets.
- Aucun secret ni identifiant d'authentification dans cette recette.

## Collecte relancée

La base PostgreSQL persistante `olc18` du conteneur `olc-ticket18-rehearsal`, volume
`olc-ticket18-rehearsal-data`, contient toujours **9 701 parties et 9 701 timelines**
après redémarrage ; **905** correspondent à EUW1 / classée Solo / 16.17–16.19.
La première agrégation retient 882 de ces parties, après exclusion de 23 remakes.
Ce nombre est un état local initial, pas le nombre total de matchs Riot.

Le census temporaire précédent a disparu au redémarrage. Il est remplacé par le
**run 18** du collecteur Rust, checkpoints/IDs/jobs enregistrés dans PostgreSQL :

```sh
cargo run -p olc-collector -- run --platform EUW1 --queue 420 \
  --patches 16.17,16.18,16.19 --tiers MASTER,GRANDMASTER,CHALLENGER \
  --window-days 56 --seeds-per-division 50000 --max-matches-per-seed 10000 \
  --target 1000000 --call-budget 200000 --concurrency 4 --collect-ranks
```

L'exécution est détachée, ses logs et paramètres non secrets sont conservés dans
`~/Library/Application Support/OpenLoLCompanion/collector/euw-apex-2026-10-02/`.
Un contrôle intermédiaire du run indique 2 400 appels, 68 190 matchs en attente
et 26 matchs déjà traités depuis le cache. Ces compteurs continuent d’évoluer.
Le lot découvre d'abord les identifiants, puis télécharge les détails manquants.
Les parties déjà présentes sont réutilisées ; un ID découvert n'est pas encore
une partie téléchargée ni une preuve d'appartenance au patch. Les seeds viennent
du classement actuel : cela ne reconstitue pas exhaustivement tous les joueurs
qui avaient ce rang sur chaque patch historique.

L'agrégateur tourne séparément, publication initiale puis horaire :

```sh
cargo run -p olc-collector -- aggregate --min-games 1 \
  --patches 16.17,16.18,16.19 --platforms EUW1 --queues 420 --watch
```

Le seuil 1 est réservé au test local demandé. Les effectifs restent visibles ;
une fréquence/taux observé sur peu de parties ne devient pas une estimation fiable.
La population publiée regroupe les parties stockées répondant aux filtres et n'est
pas exclusivement celle des nouveaux seeds Master+. Les rangs collectés sont des
observations actuelles, pas une reconstitution du rang au moment du match.

Après un nouvel arrêt de la machine, redémarrer Docker et ce conteneur, puis
reprendre le même run (ne pas créer un autre `run`) :

```sh
docker start olc-ticket18-rehearsal
cargo run -p olc-collector -- resume 18 --concurrency 4 --call-budget 200000
```

Relancer aussi la commande d'agrégation ci-dessus. Une clé Riot expirée suspend la
collecte ; la renouveler dans l'environnement serveur puis reprendre le lot.
Il n'y a pas de lancement automatique à l'ouverture de session.

## Vérifications automatiques

- Tests Rust Live : projection du joueur local, ambiguïtés, réponse incomplète,
  HTTP/redirection/taille/timeout, données retirées après coupure, reset d'horloge,
  maintien du champion malgré regroupement des événements, priorité du poste réel.
- Tests frontend : instantanés périmés, annulation, contexte champion/poste/file,
  catalogue tardif, mode consultatif, effectifs et variantes séparés, aperçu passif.
- Tests moteur : placement aux différentes résolutions/scalings, nom du jeu/PID,
  activation/focus/fin de partie, réglages bornés, langue sans changement des réglages.
- `pnpm test`, `pnpm lint`, `cargo test -p olc-desktop --lib` et bundle macOS debug :
  succès local. Les 16 tests du panneau comprennent les mesures de hauteur
  sérialisées, leur reprise après échec et l’opacité native/CSS.

## Essais réellement réalisés

- App macOS démarrée et reconnexion au client LoL en salon observée.
- Réglages : activation enregistrée, aperçu visible annoncé par le moteur,
  conservation du focus sur le formulaire, arrêt automatique de l'aperçu observé.
- Liquid Glass AppKit initialisé sous macOS 27 ; démarrage et commandes d’aperçu
  sans plantage. La WebView est détachée avant la conversion NSWindow→NSPanel,
  puis rattachée, pour préserver les observateurs WebKit. Les captures de la
  fenêtre principale seules ne valident pas les pixels du panneau ni sa
  composition au-dessus de LoL.
- API locale de builds Bard/Support, patch 16.19, EUW1, file 420 : HTTP 200,
  88 variantes publiées, seuil local 1 (première page 50).
- Au contrôle initial, le port Live 2999 n’était pas disponible. Une nouvelle
  personnalisée a ensuite confirmé le trajet réel TLS/Rust/interface : Jhin,
  niveau 1, K/D/A 0/0/0, CS 0, chronomètre 0:34, carte 11 CLASSIC.
- Diagnostic du panneau absent : tous les indicateurs étaient actifs, mais
  CoreGraphics retournait la fenêtre LoL au niveau 1000, dimensions 2560×1440.
  Le filtre limité au niveau 0 la rejetait ; le niveau fixe 3 du panneau aurait
  aussi été insuffisant. Test reproduit rouge puis vert : acceptation 0/1000,
  niveau du panneau 3/1001 et cache incluant ce niveau. Correctif chargé dans
  le compagnon pendant la partie ; affichage confirmé depuis par l’utilisateur.
- La revue indépendante de #63 a relevé deux P2 dans la garde/cache des imports
  (réponse HTTP ancienne après événement WebSocket, retour A→B→A sans build B).
  Ces anomalies distinctes du rendu du panneau ont depuis été corrigées et
  couvertes par les tests de garde et de génération des imports.

## À consigner avec une vraie partie sur chaque OS

| Scénario | macOS / Matthieu | Windows / Louison |
| --- | --- | --- |
| Prépick, runes et objets importés, deux sorts inchangés | À faire | À faire |
| Champion réel, poste et build conservés à l'entrée en partie | À faire | À faire |
| Données Live/chronomètre effectivement actualisés | À faire | À faire |
| Clics/clavier traversants, aucun vol de focus | À faire | À faire |
| Alt-Tab/Cmd-Tab, minimisation, retour au jeu | À faire | À faire |
| Fenêtré/sans bordure, Spaces plein écran Mac | À faire | À faire |
| Exclusif désactivé explicitement dans les réglages | À faire | À faire |
| Changement d'écran, résolution, Retina/125/150 % | À faire | À faire |
| Deux parties successives, reconnexion, fermeture app | À faire | À faire |
| FPS/CPU, transparence réduite, absence de contenu tronqué | À faire | À faire |

Les tickets restent ouverts jusqu'à ces recettes. L'éditeur interactif complet,
les alertes/suggestions de #27 et la validation publique Riot ne sont pas attestés
par les tests unitaires ni par le succès de la compilation.

## Confirmation et compléments

L'utilisateur a confirmé que le panneau apparaît dans sa personnalisée après le
correctif de niveau macOS. Cette preuve ne remplace pas les recettes de focus,
clics, multi-écrans, FPS/CPU ou Windows.

Les défauts d'import relevés par revue sont corrigés et testés, y compris le poste
manuel des personnalisées. Le compte actif et les amis passent maintenant par la
LCU : voir `compte-actif.md` et `amis-client.md`. L'essai de réservation de quota a
été retiré à la demande de l'utilisateur ; aucun changement des limites de
collecte n'est livré.
