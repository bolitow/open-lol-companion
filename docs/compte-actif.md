# Compte League actif dans l’accueil — #65

Le desktop suit automatiquement le Riot ID du client League et sa région. Consulter un autre joueur ne remplace ni ce compte ni le contexte de draft. Tant que League est connecté, le choix manuel du compte d’accueil est désactivé avec une explication. Hors connexion, le favori manuel reste disponible.

À la fermeture du client, l’accueil garde le dernier compte et les données déjà chargées en mémoire, avec un état déconnecté. Après redémarrage de l’app, seule l’identité est restaurée ; les statistiques et l’historique sont rechargés par le service de profils configuré, sans cache de parties sur disque. Une reconnexion actualise le compte même s’il est inchangé. Les données précédentes restent consultables pendant cette actualisation et en cas d’échec ; Réessayer ne les efface pas. Une pagination ne peut pas concurrencer le rechargement du profil. Une indisponibilité de l’identité locale garde le dernier accueil, sans le présenter comme le compte actif.

## Contrat et lecture

`LcuSession.account: LcuAccount | null` est identique en Rust et TypeScript : `platform`, `game_name`, `tag_line`. Aucun PUUID, identifiant de session ou mot de passe n’est projeté par ce nouveau flux. La seule persistance est `olc.app.home-player`, déjà utilisée par #64, avec ces trois champs. Ce stockage local n’est pas une preuve de possession et aucune association de comptes n’est publiée.

Lectures uniquement depuis Rust :

- `GET /lol-summoner/v1/current-summoner` : `gameName`, `tagLine`, contrôle `unnamed`.
- `GET /riotclient/region-locale` : `region` ; ni la langue ni le tag personnel ne déterminent la plateforme.
- Relecture du compte après la région ; un Riot ID changé entre les lectures invalide le résultat.

Sources : [schéma LCU extrait du client](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json), définitions `LolSummonerSummoner` et `LolL10nRegionLocale`, consulté le 2 octobre 2026. La LCU ne garantit pas de transaction atomique ; un changement reste réconcilié par l’événement suivant ou le rattrapage. Une région inconnue ou PBE, un compte sans Riot ID ou une erreur produisent `null`, sans fallback inventé.

Abonnement WebSocket avant les lectures. Une création/modification du compte ou de la région déclenche une relecture ; une suppression invalide le compte actif. Rattrapage toutes les 30 secondes, chaque lecture complète bornée à 3 secondes. Après la lecture initiale, la relecture du compte s’exécute en parallèle de la réception WebSocket : une réponse lente ne retarde pas les phases. Une seule relecture reste en vol ; un nouvel événement de compte la remplace, une suppression ou une fermeture WebSocket l’annule. Les identités inchangées ne produisent pas d’événement supplémentaire ni de requête de statistiques. Les phases de draft ne rechargent pas le profil. La connexion système reste le mécanisme Windows/macOS existant.

L’identité reste visible même si le service de profils est absent, limité ou en erreur ; un message FR/EN explique alors pourquoi les statistiques sont indisponibles. Les anciennes réponses d’un compte supprimé du cache ne peuvent pas remplacer le nouvel accueil. Le cache garde au maximum le compte d’accueil et le joueur consulté.

## Recette

Tests automatisés : rattrapage d’une identité absente sans événement, phase reçue pendant une réponse HTTP suspendue, suppression/déconnexion sans identité tardive, projection minimale et validation, région indépendante du tag, compte changé pendant les lectures HTTP, 404, compte initial puis changement/suppression WebSocket, rejet après déconnexion, A→B avec profil consulté indépendant, rafraîchissement à la reconnexion, absence de doublons de requêtes, réponse tardive et service non configuré.

Recette visuelle isolée : états actif, compte indisponible, déconnecté et service indisponible en FR/EN ; petites fenêtres sans défilement global. Les identités de recette sont synthétiques et ne sont pas embarquées dans l’application.

À tester ultérieurement avec League réel sur Windows (Louison) et macOS : démarrage avant/après connexion, A→B dans la même région puis une autre région, fermeture/réouverture, login incomplet, service de profils réel, maintien de la consultation d’un autre joueur. Ne pas considérer les tests simulés ou la compilation CI comme cette recette. Aucun lancement de League nécessaire pendant le développement de ce lot.

Hors périmètre : authentification du profil app, vérification de propriété, comptes liés publiquement, suivi d’amis, import automatique #63, modification de la draft.
