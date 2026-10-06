# Compte League actif dans l’accueil — #65

Le desktop suit automatiquement le Riot ID du client League et sa région. Consulter un autre joueur ne remplace ni ce compte ni le contexte de draft. Tant que League est connecté, le choix manuel du compte d’accueil est désactivé avec une explication. Hors connexion, le favori manuel reste disponible.

À la fermeture du client, l’accueil garde le dernier compte et les données déjà chargées en mémoire, avec un état déconnecté. Après redémarrage de l’app, seule l’identité est restaurée. Pour le compte actif, le profil et l’historique sont lus dans le client LoL ; les autres comptes utilisent le service public configuré. Aucun cache de parties sur disque. Une reconnexion actualise le compte même s’il est inchangé. Les données précédentes restent consultables pendant cette actualisation et en cas d’échec ; Réessayer ne les efface pas. Une pagination ne peut pas concurrencer le rechargement du profil. Une indisponibilité de l’identité locale garde le dernier accueil, sans le présenter comme le compte actif.

## Changement d’avatar (#127)

L’identité repose sur la plateforme et le Riot ID ; `profile_icon_id` est uniquement
un attribut d’affichage. Son changement est diffusé à l’interface sans augmenter la
génération du lecteur de profils, invalider une lecture en vol ou effacer les choix
de préparation. Les lectures locales conservent leur contrôle privé du PUUID ;
un véritable changement de compte ou une reconnexion invalide toujours les anciennes
réponses. Les images utilisent le catalogue versionné du patch (#93).

## Contrat et lecture

`LcuSession.account: LcuAccount | null` est identique en Rust et TypeScript : `platform`, `game_name`, `tag_line`. Aucun PUUID, identifiant de session ou mot de passe n’est projeté par ce nouveau flux. La seule persistance est `olc.app.home-player`, déjà utilisée par #64, avec ces trois champs. Ce stockage local n’est pas une preuve de possession et aucune association de comptes n’est publiée.

Lectures uniquement depuis Rust :

- `GET /lol-summoner/v1/current-summoner` : `gameName`, `tagLine`, contrôle `unnamed`.
- `GET /riotclient/region-locale` : `region` ; ni la langue ni le tag personnel ne déterminent la plateforme.
- Relecture du compte après la région ; un Riot ID changé entre les lectures invalide le résultat.

Sources : [schéma LCU extrait du client](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json), définitions `LolSummonerSummoner` et `LolL10nRegionLocale`, consulté le 2 octobre 2026. La LCU ne garantit pas de transaction atomique ; un changement reste réconcilié par l’événement suivant ou le rattrapage. Une région inconnue ou PBE, un compte sans Riot ID ou une erreur produisent `null`, sans fallback inventé.

Abonnement WebSocket avant les lectures. Une création/modification du compte ou de la région déclenche une relecture ; une suppression invalide le compte actif. Rattrapage toutes les 30 secondes, chaque lecture complète bornée à 3 secondes. Après la lecture initiale, la relecture du compte s’exécute en parallèle de la réception WebSocket : une réponse lente ne retarde pas les phases. Une seule relecture reste en vol ; un nouvel événement de compte la remplace, une suppression ou une fermeture WebSocket l’annule. Les identités inchangées ne produisent pas d’événement supplémentaire ni de requête de statistiques. Les phases de draft ne rechargent pas le profil. La connexion système reste le mécanisme Windows/macOS existant.

Le profil du compte actif ne dépend plus du service public ni du quota du collecteur. Une indisponibilité locale est indiquée sans bascule automatique vers Riot API. Les autres profils restent dépendants du service public. Les anciennes réponses d’un compte supprimé du cache ne peuvent pas remplacer le nouvel accueil. Le cache garde au maximum le compte d’accueil et le joueur consulté.

## Recette

Tests automatisés : rattrapage d’une identité absente sans événement, phase reçue pendant une réponse HTTP suspendue, suppression/déconnexion sans identité tardive, projection minimale et validation, région indépendante du tag, compte changé pendant les lectures HTTP, 404, compte initial puis changement/suppression WebSocket, rejet après déconnexion, A→B avec profil consulté indépendant, rafraîchissement à la reconnexion, absence de doublons de requêtes, réponse tardive et service non configuré.

Recette visuelle isolée : états actif, compte indisponible, déconnecté et service indisponible en FR/EN ; petites fenêtres sans défilement global. Les identités de recette sont synthétiques et ne sont pas embarquées dans l’application.

À tester ultérieurement avec League réel sur Windows (Louison) et macOS : démarrage avant/après connexion, A→B dans la même région puis une autre région, fermeture/réouverture, login incomplet, service de profils réel, maintien de la consultation d’un autre joueur. Ne pas considérer les tests simulés ou la compilation CI comme cette recette. Aucun lancement de League nécessaire pendant le développement de ce lot.

Hors périmètre : authentification du profil app, vérification de propriété, comptes liés publiquement, suivi d’amis, import automatique #63, modification de la draft.


### Avatar et connexion dans la navigation (#4, #65)

Le bandeau de connexion et le footer permanent sont remplacés par l’avatar du compte, en haut à droite. Un clic ouvre son Riot ID, la région, l’état du client et les actions profil/session ou réessayer. La pastille verte indique uniquement la connexion au client League ; jaune indique attente/recherche et rouge une erreur de suivi. L’aperçu navigateur est gris. Le statut est aussi lisible par les lecteurs d’écran et dans le panneau. Échap, un clic extérieur ou la sortie du focus ferment le panneau.

`LcuAccount.profile_icon_id` projette uniquement `profileIconId` du `current-summoner` déjà lu par Rust (entier public positif ou nul). Aucun appel local supplémentaire et aucun secret ne transitent dans React. Le dernier identifiant d’icône est mémorisé avec l’identité locale existante, remplacé au changement de compte et effacé avec « oublier ». Il reste distinct des paramètres `PlayerRequest` envoyés au service de profils.

L’image vient du CDN HTTPS public [Data Dragon](https://developer.riotgames.com/docs/lol#data-dragon_other), version 16.19.1 du catalogue actuel. Pas de téléchargement ni de cache fichier spécifique : le cache HTTP du WebView s’applique. Une icône récente absente de cette version, une erreur réseau ou un identifiant manquant donnent une silhouette neutre, jamais l’avatar d’un autre compte. Le service de profils peut fournir l’icône en repli pour le même compte. Les mentions Riot restent visibles dans le menu principal.
## Profil, historique local et amis — complément du 2 octobre 2026

Le profil actif utilise désormais `GET /lol-ranked/v1/current-ranked-stats` pour les rangs et `GET /lol-match-history/v1/products/lol/current-summoner/matches?begIndex=…&endIndex=…` pour les parties. Le PUUID est utilisé seulement dans Rust pour retrouver la participation locale, par jointure `participantId`, et vérifier le compte avant/après. Les réponses tardives d'une ancienne connexion sont rejetées même après A→B→A. La lecture complète est bornée à dix secondes. Le DTO desktop ne transmet plus de PUUID, quelle que soit la source.

Le profil affiche sa source (`lcu` ou `api`). L'historique local peut inclure les personnalisées et avertit qu'il dépend des données fournies par le client. `gameCount` n'est jamais présenté comme un total saison. Une pagination reste attachée à sa source et refuse les indices incohérents ; fermer League ne mélange pas les pages locales et publiques. Les données en mémoire restent visibles pendant l'actualisation ; le passage à `EndOfGame` actualise une fois le compte actif, et l'accueil propose aussi Actualiser.

Recette réelle macOS : profil local avec niveau et deux rangs ; dix parties locales en 289 ms, aucune partie omise. Le client annonçait une page suivante, mais la demande suivante renvoyait encore les indices 0–9 : le lecteur l’a refusée sans ajouter de doublons. Une sonde initiale n'avait fourni qu'une personnalisée : la disponibilité dépend aussi du cache/service du client. Aucune garantie de cinquante parties ou d'exhaustivité de saison. Client fermé après redémarrage de l'app, le dernier compte peut être consulté via le service public ; cette voie garde ses contraintes de configuration/quota.

Amis : voir [contrat #71](amis-client.md). Les données client restent séparées du collecteur Riot et des agrégats de builds.

Recette finale du bundle macOS : profil actif, niveau, rangs Solo/Flex et source client LoL visibles, dix parties (dont les personnalisées récentes) dans l'accueil. Les amis sont chargés simultanément, sans appel à l'API publique pour ces lectures. La pagination au-delà de cette première page reste dépendante du service/cache LCU ; ne pas annoncer cinquante parties validées.


### Finition Accueil / Joueurs — 3 octobre 2026 (#4)

Le bloc de préparation de l’accueil est raccourci pour donner davantage de hauteur aux dernières parties. La fiche partagée Accueil/Joueurs affiche l’avatar public (initiales en repli si l’image manque), un Riot ID compact et les deux rangs. Une valeur de PL absente reste « — », jamais zéro par défaut.

L’historique distingue le chargement du profil, son indisponibilité et une page effectivement vide. Les matchs déjà reçus restent visibles pendant un échec d’actualisation. Le repère V/D et sa barre à deux segments représentent uniquement les parties affichées, sans extrapolation de niveau ou de saison, et restent bornés après pagination. Les données gardent leur source client/API et leur horodatage.

Contrôle de présentation avec fixtures explicitement signalées : 1000 × 650 en sombre, 1280 × 800 en clair ; défilement interne, aucun dropdown natif. Parcours profil → champion Ahri → retour : même Riot ID, défilement conservé à 300 px. Cette recette visuelle ne prouve pas une lecture LCU réelle. Le module Amis et les transports existants sont conservés ; aucune nouvelle recommandation implémentée.

Bundle Tauri macOS reconstruit et relancé : avatar du compte conservé, état déconnecté et historique indisponible vérifiés sur Accueil puis Joueurs, avec retour fonctionnel. League fermé et service public non configuré pendant ce contrôle : rangs et parties remplis vérifiés uniquement avec les fixtures. `pnpm test` : 854 tests réussis, deux tests Rust ignorés ; `pnpm lint` et revue stricte OK. Test manuel Windows non exécuté.

### Ajustements de l’accueil après retour visuel (#4)

Les amis occupent une ligne : avatar Data Dragon issu de `icon_id`, nom et pastille de présence sur l’avatar. Le Riot ID, la région et le statut restent dans le titre au survol et le nom accessible ; les identités incomplètes restent sans action. Une icône absente ou en erreur revient aux initiales. La liste continue de défiler à l’intérieur de sa carte.

Les dix emblèmes officiels Riot sont embarqués dans `apps/desktop/public/game-data/ranks` (provenance documentée dans ce dossier), affichés sur l’accueil et le profil. Aucun emblème de rang n’est attribué aux données absentes ou non classées.

Le texte « Compte actif dans League » est retiré de l’accueil. Les états déconnecté/compte indisponible restent explicites. Actualiser et Voir le profil sont regroupés près de l’identité avec libellés accessibles ; Retirer de l’accueil n’apparaît que hors connexion. Les actions n’occupent plus un pied de carte susceptible de déborder sous les rangs.

Recette de cette passe : 857 tests réussis (deux tests Rust ignorés), lint et build macOS OK ; revue stricte OK après ajout du rôle accessible aux amis sans profil. En 1000 × 650, sombre et clair, la carte et les actions restent contenues, les lignes d’amis mesurent 46 px. Sur le bundle natif connecté à League, avatars réels des amis et deux emblèmes Or visibles. Une réponse d’historique refusée comme incohérente a été observée après reconnexion, puis après Réessayer/Actualiser ; ce parcours de données n’a pas été modifié dans cette passe et nécessite une investigation distincte. Aucun test Windows exécuté.


## Pagination locale résiliente — #94

Si League renvoie une page d’indices cohérents commençant à zéro alors qu’une page
suivante était demandée, le lecteur termine l’historique local sans réajouter les
parties : l’interface conserve les lignes déjà chargées et affiche « Fin des parties
fournies par le client ». Tout autre décalage ou intervalle incohérent reste une erreur.
La plateforme, la jointure vers le participant local, les identifiants de partie et
leur unicité sont contrôlés même sur cette page répétée.

Une partie dont les statistiques sont illisibles (par exemple victoire absente,
date invalide ou champion zéro) est omise et comptée dans `omitted_matches`.
L’avertissement local FR/EN existant l’indique. La pagination avance selon le nombre
d’entrées brutes, y compris si elles sont toutes omises ; elle ne dépend pas du
nombre de cartes affichables. Une identité ambiguë, une autre plateforme ou un
doublon restent fatals, même si les statistiques de cette partie sont invalides.
Aucun nouveau endpoint ni repli vers l’API publique n’est ajouté.

Les scénarios sont couverts par le serveur LCU simulé et un test du parcours
store → historique rendu en FR/EN. La recette sur client réel reste distincte des
tests automatisés ; l’alternative par endpoint PUUID n’est pas utilisée ni déclarée
vérifiée par cette correction.


Recette réelle macOS du 4 octobre 2026 sur la base `9b4aea6` avec ce correctif :
lecture via le connecteur Rust, 10 parties au curseur 0 (`next_start=10`), puis
0 partie au curseur 10 (`next_start=null`), sans erreur ni doublon. Aucun Riot ID,
PUUID ou secret enregistré dans le journal de recette. Les anomalies de partie
restent validées par simulation, pas par altération des données du client. Le
rendu FR/EN est couvert par le test de composant ; le bundle ouvert n’a pas été
remplacé pendant cette recette. Windows réel reste à vérifier.

### Affichage des rangs apex (#106)

Master, Grandmaster et Challenger sont affichés sans division, quelle que soit la
division technique reçue. Leurs points de ligue restent affichés ; les autres paliers
gardent leur division. Les contrats API, le collecteur et la validation LCU ne sont
pas modifiés : la valeur réellement renvoyée par un compte apex dans le client reste
à observer avant d’élargir son parsing.
