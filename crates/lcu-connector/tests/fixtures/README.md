# Fixtures du connecteur LCU

## `rune-styles.json`

Extrait du [catalogue des styles de runes du client publié par CommunityDragon](https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perkstyles.json), consulté le 1er octobre 2026.

La fixture conserve les styles Précision (`8000`) et Sorcellerie (`8200`) du tableau `styles`, avec les champs `id`, `allowedSubStyles` et `slots`. Pour chaque ligne, seuls `type` et `perks` sont conservés. Les identifiants et leur ordre sont ceux de la source ; les noms, descriptions et chemins d'images inutiles à la validation sont omis. Le tableau extrait correspond aux champs consommés depuis `GET /lol-perks/v1/styles`, dont le contrat est confirmé dans le [schéma LCU](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json).

Ces données statiques ne contiennent aucun identifiant de joueur ni secret. Les réponses de pages utilisées dans les tests sont synthétiques et ne proviennent d'aucun compte réel.

## `live-client-allgamedata-macos-2026-10-03.json`

Extrait réel de `https://127.0.0.1:2999/liveclientdata/allgamedata`, capturé le
3 octobre 2026 à 09:25:27, heure de Paris, pendant la personnalisée de Matthieu
avec Mel. Client LoL `16.19.8230722`, macOS 27.0 sur Apple M4 Pro. Contrat :
[documentation Riot](https://developer.riotgames.com/docs/lol#game-client-api_live-client-data-api).

L’extraction Rust remplace l’identité locale par `fixture-local` dans les deux
champs `riotId` nécessaires à la correspondance, avant toute écriture. Elle ne
conserve que ce joueur, son champion, niveau, K/D/A, CS, les `itemID`, les trois
champs de `gameData` consommés et les événements publics retenus, sans acteurs.
Tous les autres joueurs, identités et champs sont supprimés ; aucun dump brut
n’est conservé. Les nombres et identifiants de jeu conservés ne sont pas simulés.

Cette fixture teste le schéma et la projection, pas la cadence. La série de
valeurs reçues et les limites de la mesure figurent dans la
[recette macOS #23](../../../../docs/recettes/2026-10-03-live-macos.md).

## Sélection
`champ-select-public.json` est un extrait anonymisé de la première capture publique de https://gist.github.com/xadamxk/8cb5d21d24bb78d63c5241e97087bb23. Capture ancienne (horodatage décembre 2019), consultée le 1er octobre 2026, pas un relevé du client actuel. Seuls actions, camps, champion/intent, position, bans et timer sont conservés ; identifiants, chat, skins et sorts exclus. Le schéma courant a été confronté au dump Riot 16.19 / CommunityDragon. Les autres variantes des tests sont synthétiques, dérivées de ce schéma. La recette du client réel reste indispensable.
