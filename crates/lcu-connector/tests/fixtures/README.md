# Fixtures du connecteur LCU

## `rune-styles.json`

Extrait du [catalogue des styles de runes du client publié par CommunityDragon](https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perkstyles.json), consulté le 1er octobre 2026.

La fixture conserve les styles Précision (`8000`) et Sorcellerie (`8200`) du tableau `styles`, avec les champs `id`, `allowedSubStyles` et `slots`. Pour chaque ligne, seuls `type` et `perks` sont conservés. Les identifiants et leur ordre sont ceux de la source ; les noms, descriptions et chemins d'images inutiles à la validation sont omis. Le tableau extrait correspond aux champs consommés depuis `GET /lol-perks/v1/styles`, dont le contrat est confirmé dans le [schéma LCU](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json).

Ces données statiques ne contiennent aucun identifiant de joueur ni secret. Les réponses de pages utilisées dans les tests sont synthétiques et ne proviennent d'aucun compte réel.

## Sélection
`champ-select-public.json` est un extrait anonymisé de la première capture publique de https://gist.github.com/xadamxk/8cb5d21d24bb78d63c5241e97087bb23. Capture ancienne (horodatage décembre 2019), consultée le 1er octobre 2026, pas un relevé du client actuel. Seuls actions, camps, champion/intent, position, bans et timer sont conservés ; identifiants, chat, skins et sorts exclus. Le schéma courant a été confronté au dump Riot 16.19 / CommunityDragon. Les autres variantes des tests sont synthétiques, dérivées de ce schéma. La recette du client réel reste indispensable.
