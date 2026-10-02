# Fixtures du connecteur LCU

## `rune-styles.json`

Extrait du [catalogue des styles de runes du client publié par CommunityDragon](https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perkstyles.json), consulté le 1er octobre 2026.

La fixture conserve les styles Précision (`8000`) et Sorcellerie (`8200`) du tableau `styles`, avec les champs `id`, `allowedSubStyles` et `slots`. Pour chaque ligne, seuls `type` et `perks` sont conservés. Les identifiants et leur ordre sont ceux de la source ; les noms, descriptions et chemins d'images inutiles à la validation sont omis. Le tableau extrait correspond aux champs consommés depuis `GET /lol-perks/v1/styles`, dont le contrat est confirmé dans le [schéma LCU](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json).

Ces données statiques ne contiennent aucun identifiant de joueur ni secret. Les réponses de pages utilisées dans les tests sont synthétiques et ne proviennent d'aucun compte réel.
