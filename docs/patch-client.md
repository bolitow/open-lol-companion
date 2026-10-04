# Patch du client LoL — #93 (cœur Rust)

Le desktop peut lire la version du jeu installée dans le client League au lieu de la déduire du catalogue embarqué. La lecture Rust et l’affichage dans les réglages sont disponibles. Le choix du patch des requêtes, le recoupement avec le manifeste et le repli restent dans #93.

## Contrat

Commande Tauri `client_patch`, réservée à la fenêtre principale, sans argument. Lecture à la demande : rien n'est ajouté à `LcuSession` ni au suivi WebSocket.

- Succès : `ClientPatch { gameVersion, patch }`, identique en Rust (`lcu_connector::ClientPatch`) et TypeScript (`@olc/shared`). `gameVersion` est la chaîne brute du client, par exemple `16.19.715.1234` ; `patch` en garde `majeur.mineur`, ici `16.19`.
- Erreur : `ClientPatchError`, `unavailable` (client fermé, refus, corps non JSON, délai de 3 secondes dépassé) ou `invalid_response` (JSON qui n'est pas une chaîne, version vide, par exemple pendant une mise à jour, ou forme inattendue).

Le cœur Rust ne déduit aucune version Data Dragon (`16.19.1`) ni libellé public. Les réglages (#186) convertissent seulement le numéro technique pour l’affichage : à partir de 15, le majeur public vaut majeur + 10 (`16.19` → `26.19`). Les valeurs plus anciennes conservent leur numéro. Une entrée invalide ne devient jamais un patch fictif. Les requêtes et catalogues gardent leur version technique.

## Lecture

`GET /lol-patch/v1/game-version`, réponse 200 de type chaîne. Source : [schéma LCU extrait du client](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json), opération `GetLolPatchV1GameVersion`, consulté le 4 octobre 2026. Le schéma ne décrit pas le format de la chaîne : les deux premiers segments doivent être des entiers, la suite est conservée telle quelle, bornée à 256 caractères, sans espace ni caractère de contrôle. Recette macOS du 4 octobre 2026 : le client renvoie aussi un suffixe de build, par exemple `16.19.8230722+branch.releases-16-19.code.public.content.release.anticheat.vanguard` (82 caractères). Ce suffixe est conservé, sans être interprété.

La connexion réutilise la découverte du client existante (lockfile puis processus), déjà prise en charge sous Windows et macOS. Le mot de passe du client ne quitte pas le cœur Rust.


## Réglages — #186

La carte « Version du client LoL » est retrouvable par recherche (patch/version),
avec erreurs FR/EN distinctes : client introuvable ou version illisible. Elle lit
la version lors de son montage et sur « Actualiser » uniquement, sans polling.
Une seule lecture peut être en cours ; un échec retire la version précédemment
lue pour ne pas la présenter comme actuelle. Le navigateur affiche une indication
desktop, sans tenter de découvrir League. La paire majeure/mineure de la version brute et son patch à deux segments doivent donner le même libellé public, y compris avec un suffixe de build.

La première recette macOS avait renvoyé `invalid_response` : la limite de 64 caractères rejetait le suffixe légitime. La correction est couverte par le format observé, les bornes, les suffixes corrompus et la cohérence Rust/interface. Aucun patch de remplacement n’est inventé.

Après correction, recette native macOS du 4 octobre 2026 : la carte des réglages affiche **26.19** depuis le client ouvert, et conserve cette valeur après actualisation manuelle. Windows reste à vérifier.
