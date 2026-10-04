# Checklist de conformité Riot

Références (textes primaires, relus le 04/10/2026, mise à jour du 29/05/2025 pour les politiques générales) :
[Developer API Policy](https://developer.riotgames.com/docs/lol#developer-api-policy) (« Registration », « Monetization », « Security », « [Game Integrity](https://developer.riotgames.com/docs/lol#developer-api-policy_game-integrity) » : source de la règle « information not present in the game client » et de l'interdiction d'identifier les joueurs « deliberately hidden by the game »),
[politiques générales du Developer Portal](https://developer.riotgames.com/policies/general) (« Core Policies », « Monetization », « Game Integrity »),
[Game Policy de League of Legends](https://developer.riotgames.com/docs/lol#game-policy) (« Approved » et « Unapproved Use Cases ») et section 2 du [cahier des charges](../docs/cahier-des-charges.md).
L'ancien lien du centre d'aide vers la Developer API Policy n'est pas la référence : la version publiée sur docs/lol fait foi. Une lecture de seconde main (presse, résumé automatique, outil concurrent) ne tranche jamais : recouper sur ces pages. À vérifier pour toute fonctionnalité qui lit ou affiche des données de jeu.

- [ ] L'information affichée est visible par le joueur dans le client ou le jeu à ce moment-là (tableau des scores, minimap, écran de sélection…). La Game Policy exclut toute information de la partie en cours « previously unknown to the player ».
- [ ] Aucune information sur l'adversaire qui n'est pas visible (cooldowns d'ultimes, sorts, position hors vision, or exact non affiché…).
- [ ] Aucune action automatique dans le jeu (pas de clic, pas de touche simulée). Seuls les imports dans le **client** (runes, sorts, items) sont permis, et à l'initiative du joueur ou de son réglage. Aucun pick, ban ni verrouillage envoyé au client (pas de bouton « Lock »), aucun échange de position envoyé au client.
- [ ] Aucune modification du jeu ni injection dans son processus : fenêtres externes uniquement.
- [ ] Aucune publicité, nulle part.
- [ ] Pas d'imitation de l'interface de Riot.
- [ ] Données statiques (icônes, noms) issues de Data Dragon / CommunityDragon, avec la mention légale Riot.
- [ ] Fonctionnalité listée dans la demande de clé de production si elle utilise l'API Riot, et usage de la LCU déclaré sur le Developer Portal.
- [ ] Aucun taux de victoire, tier ni note d'augment (Arena ou Mayhem) ni d'objet Arena (voir « Fonctions refusées »).
- [ ] Aucun MMR, ELO ni équivalent estimé (voir « Fonctions refusées »).
- [ ] Aucune désanonymisation : en sélection des champions, aucune identité n'est lue, stockée ni envoyée (voir « Anonymat de la sélection »).

## Fonctions refusées

Elles ne se développent pas, même sur demande d'un ticket ou du cahier des charges, tant qu'une décision écrite de Riot ne les autorise pas. Décision du ticket #79 ; les rapprochements avec les textes sont dans le §2 du cahier.

| Fonction refusée | Pourquoi |
| --- | --- |
| Cooldowns ou timers de sorts d'invocateur et d'ultimes adverses | Developer API Policy > Game Integrity ([docs/lol#developer-api-policy_game-integrity](https://developer.riotgames.com/docs/lol#developer-api-policy_game-integrity)) : « Products must not use or incorporate information not present in the game client that would give players a competitive edge (e.g., automatically or manually allowing tracking enemy ultimate cooldowns) ». Information de partie inconnue du joueur (Game Policy). La date du 13/03/2025 (annonce de Riot Developer Relations relayée par la presse) n'est qu'un contexte de seconde main |
| Timers de camps de jungle non vus (camps adverses ou non observés) | Information de partie inconnue du joueur. Le module Jungle de #4 se limite aux camps tués par le joueur, déduits de son propre or et de son CS jungle |
| Or exact de l'adversaire non affiché par le client | Information de partie inconnue du joueur |
| Conseils tactiques en temps réel qui dictent une décision | « Apps that dictate player decisions » (Game Policy) ; « Game Integrity » : mettre en évidence et proposer plusieurs choix, jamais imposer |
| Coaching vocal tactique par IA | Même motif ; zone grise, refusé sans validation de Riot |
| Taux de victoire d'augments (Arena, Mayhem) et d'objets Arena, y compris tiers, notes et classements dérivés, sur site, app et overlay | Game Policy : règle pour « all websites, applications and overlays » ; « augment » n'est pas qualifié par le mode, Mayhem inclus. Ne rien collecter ni calculer à ce sujet |
| MMR, ELO, note « de niveau caché », courbe LP déduite d'un MMR | « Game Integrity » : pas d'alternative au classement officiel, MMR ou ELO inclus. Comparer au rang affiché et au rôle ; le delta de LP réel annoncé par le client peut s'afficher tel quel |
| Bouton Lock, ou tout pick, ban ou échange de poste envoyé au client | Action dans le client hors de la liste blanche des imports |
| Premades et badges de joueurs en sélection des champions | Désanonymisation. Autorisé : son propre groupe, ou la phase de chargement et la partie (identités publiques), après validation de Riot (#30) |

Cas non tranchés par les textes (suspendus, à soumettre à Riot via #2 et #30) : popularité seule d'un augment (sans victoire), overlay en sélection d'augment, premades au chargement, comparaison à l'adversaire direct (CS, vision, or visible au tableau des scores).

## Anonymat de la sélection

- Base : Developer API Policy > Game Integrity : « Products cannot identify or analyze players who are deliberately hidden by the game ».
- Aucune identité (puuid, gameName, tagLine, pseudo d'invocateur) ni intention ou poste non publics des autres joueurs ne sort de la lecture de la sélection : ni vers l'interface (IPC), ni vers le service, ni dans un journal.
- Aucune requête de rang, de maîtrise ou d'historique n'est déclenchée par la liste des participants d'une sélection (pas de multi-recherche pré-remplie, pas d'appel LCU par puuid, pas de croisement d'historiques pour deviner des groupes).
- Les fonctions de profil (recherche de joueur, maîtrise, « joué récemment avec ») ne partent que d'une saisie manuelle du joueur ou du compte actif.
- Le test de la projection de draft doit vérifier l'absence de `puuid`, `gameName`, `tagLine` en plus de `summoner` et `chat` (renforcement à faire dans le code, hors du lot documentaire de #79).

Cas limites (OCR des augments ou des reliques, timers de reliques, différence d'or, comparaison à l'adversaire direct) : ouvrir une discussion dans le ticket et obtenir une validation explicite avant de fusionner.
