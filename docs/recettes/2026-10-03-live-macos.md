# Recette Live Client macOS — 3 octobre 2026 — #23

## Périmètre et version

[Ticket #23](https://github.com/bolitow/open-lol-companion/issues/23), section 6 du
cahier des charges : données locales en partie, cadence, événements et transitions.
La recette native du panneau #22 et les imports #63 restent distincts.

- macOS 27.0 (26A428), Mac mini Apple M4 Pro, 24 Go de RAM, arm64.
- League of Legends `16.19.8230722`, parties personnalisées CLASSIC, carte 11.
- Binaire desktop debug construit depuis `31c8d791b0ad9410b860c1690ac24e52d47c5d10` ;
  SHA-256 `1634ce295deceb21bd65bb0147f9a2bf0b2f4584997473af4258324fab46534b`.
- Les ajouts de cette recette extraient la boucle interne pour la tester avec
  une lecture contrôlée, sans modifier son comportement ni son API publique.
  Ces ajouts sont vérifiés par les tests ; le binaire de la recette réelle est
  celui identifié ci-dessus, pas un nouveau bundle contenant cette extraction.

## Méthode et données conservées

Matthieu joue manuellement. Un diagnostic Rust temporaire interroge uniquement
`https://127.0.0.1:2999/liveclientdata/allgamedata` : une requête à la fois, attente
d’une seconde après réponse, timeout de 2 s, limite de 4 Mio, sans proxy ni
redirection. L’exception de certificat du diagnostic est limitée à cette URL
loopback constante ; le transport applicatif n’est pas modifié.

Le diagnostic sélectionne le joueur actif sans ambiguïté puis remplace son
identité avant toute sortie disque. Aucun pseudo réel, PUUID, donnée adverse,
acteur d’événement, clé API ou mot de passe n’est conservé. Aucun accès mémoire,
aucune injection, aucun clic ou touche de jeu automatisé.

- [Série complète anonymisée](assets/2026-10-03-live-macos.csv) : 339 lectures
  entre 09:22:10 et 09:27:51, heure de Paris, dont 301 projections valides.
- 231 lectures valides pour Mel, 70 pour Yone ; 32 indisponibilités, 5 réponses
  HTTP non réussies et 1 réponse sans projection locale valide pendant les
  phases de démarrage/transition. Ces statuts sont ceux du diagnostic.
- Intervalle médian entre lectures valides consécutives : 1,008 s ; minimum
  1,003 s, maximum 1,810 s. Réponse valide la plus lente : 806 ms.
- Le diagnostic est un lecteur distinct du compagnon : ses horodatages ne
  mesurent pas la latence de rendu de l’application. Il a été arrêté après la capture.

## CS et actualisation réelle

| Heure de Paris | Temps du jeu reçu | Champion | CS reçus |
| --- | --- | --- | --- |
| 09:22:15,497 | 0,011 s | Mel | 0 |
| 09:24:13,221 | 117,698 s | Mel | 10 |
| 09:25:27,779 | 192,256 s | Mel | 20 |
| 09:26:42,191 | 0,835 s | Yone | 0 |

Aucune valeur intermédiaire de CS n’est reçue dans cette série : sur Mel, seuls
0, 10 et 20 apparaissent malgré les lectures à environ 1 Hz. Matthieu confirme
que le panneau suit ces paliers et que les autres données sont actualisées.
L’interface accessible confirme Mel niveau 3, puis Yone niveau 1, K/D/A 0/1/0,
0 CS et 0:52 dans la nouvelle partie.

Ce relevé établit les valeurs fournies par la source dans ces parties. Le nombre
exact de CS du HUD à chaque instant n’a pas été relevé : aucune valeur intermédiaire
du jeu ni mesure de retard HUD/API n’est inventée. Le lecteur conserve chaque
entier reçu, y compris hors multiples de dix ; les tests Rust et FR/EN couvrent
9, 10, 11, 19 et 20. Le signalement [Riot #416](https://github.com/RiotGames/developer-relations/issues/416)
est un retour développeur, pas une garantie contractuelle ni un correctif Riot.

## Transitions observées et confirmées

- **Relance du compagnon en pleine partie de Mel** : reconnexion réelle, champion,
  niveau, K/D/A et chronomètre affichés ensuite. La source Live continue à répondre.
- **Fin de partie puis nouvelle partie sans fermer le compagnon** : Matthieu
  confirme la disparition des anciennes données et le retour correct du nouveau
  champion et de son chronomètre. La capture passe de Mel à une source indisponible
  à 09:26:09, puis à Yone à 09:26:42 avec une horloge inférieure à une seconde.
- **Pas de réapparition des données de Mel** signalée par Matthieu ; l’inspection
  native montre Yone. Les lectures qui suivent le retour de Yone restent sur Yone.
- **Requête précisément interrompue pendant son vol** : pas d’instrumentation de
  cette course dans le jeu réel ; ce scénario est couvert séparément par les tests
  déterministes ci-dessous. La transition réelle entre parties ne prouve pas à
  elle seule que la sortie a coïncidé avec une requête du compagnon en vol.

Le message « Les statistiques ne sont pas encore connectées sur cet appareil »
signalé pendant la séance concernait le service communautaire, pas la source Live.
La relance effectuée lors de #22 avait omis sa configuration d’environnement.
L’adresse locale et un jeton de lecture temporaire ont été rétablis au lancement,
sans transmettre de clé Riot ni de secret serveur au desktop. Dans la partie de
Yone, l’interface affiche à nouveau les statistiques EUW1 / Solo-Duo / Mid,
avec la mention explicite de leur emploi en personnalisée. Cette restauration
de configuration ne déclare pas les imports #63 validés.

## Fixture et tests

- [Fixture réelle](../../crates/lcu-connector/tests/fixtures/live-client-allgamedata-macos-2026-10-03.json)
  issue de la réponse Mel à 20 CS, avec liste des champs retirés dans le README
  des fixtures. Son test vérifie les valeurs et l’absence d’identité dans la
  projection destinée à l’interface.
- Lecture en attente annulée à la sortie, réponse tardive rejetée, aucun nouveau
  polling hors partie.
- Une seule lecture en vol et attente après réponse ; suppression des données
  à l’indisponibilité, puis reprise avec une nouvelle valeur.
- Sortie/rentrée regroupées avant réveil du lecteur : nouvelle génération,
  ancien contexte supprimé, ancienne lecture annulée, nouveau champion et horloge.
- Recul d’horloge : suppression du contexte de la partie précédente.

Ces tests utilisent une horloge virtuelle et remplacent uniquement la lecture
externe ; ils exécutent la vraie boucle de suivi. Aucun test du watcher ne lit le
port réel 2999. Quatre mutations temporaires ont vérifié leur capacité à échouer :
suppression de l’annulation, du suivi des transitions regroupées, de l’effacement
du contexte lors du recul d’horloge et de l’effacement des données sur erreur.
Les quatre échecs attendus ont été observés, puis le code restauré passe les tests.

- `pnpm test` : succès, 255 tests TypeScript et 437 résultats Rust réussis,
  2 recettes catalogue explicitement ignorées. `OLC_TEST_DATABASE_URL` absente :
  les tests qui retournent sans base n’ont pas exercé PostgreSQL ; aucune recette
  backend n’est revendiquée ici.
- `pnpm lint` : succès, typage TypeScript, `cargo fmt --check` et Clippy à zéro erreur.
- `git diff --check` : succès. Revue indépendante : la portée des preuves et les
  critères encore requis sont conservés explicitement.

## Statut et limites

**Recette fonctionnelle macOS confirmée par Matthieu sur cette configuration.**
Les CS reçus, la fixture et la transition entre deux champions sont documentés.
Le ticket reste ouvert pour la recette Windows attribuée à Louison et les
compléments de preuve requis : comparaison chiffrée HUD/API synchronisée et
interruption réseau en pleine partie indépendante de sa fin, ainsi que la course
entre une requête en vol et une sortie/rentrée rapide observée en conditions réelles.
Les essais déterministes ne remplacent pas ces observations réelles.
