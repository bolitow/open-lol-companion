# Patch du client LoL — #93 (cœur Rust)

Le desktop peut lire la version du jeu installée dans le client League au lieu de la déduire du catalogue embarqué. Cette partie livre seulement la lecture côté Rust et son contrat ; l'interface qui choisit le patch des requêtes, le repli et l'affichage restent à faire.

## Contrat

Commande Tauri `client_patch`, réservée à la fenêtre principale, sans argument. Lecture à la demande : rien n'est ajouté à `LcuSession` ni au suivi WebSocket.

- Succès : `ClientPatch { gameVersion, patch }`, identique en Rust (`lcu_connector::ClientPatch`) et TypeScript (`@olc/shared`). `gameVersion` est la chaîne brute du client, par exemple `16.19.715.1234` ; `patch` en garde `majeur.mineur`, ici `16.19`.
- Erreur : `ClientPatchError`, `unavailable` (client fermé, refus, corps non JSON, délai de 3 secondes dépassé) ou `invalid_response` (JSON qui n'est pas une chaîne, version vide, par exemple pendant une mise à jour, ou forme inattendue).

Aucune version Data Dragon (`16.19.1`) ni libellé public (`26.19`) n'est déduit : ils viennent du manifeste Data Dragon ou de l'API, à recouper par l'interface.

## Lecture

`GET /lol-patch/v1/game-version`, réponse 200 de type chaîne. Source : [schéma LCU extrait du client](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json), opération `GetLolPatchV1GameVersion`, consulté le 4 octobre 2026. Le schéma ne décrit pas le format de la chaîne : les deux premiers segments doivent être des entiers, la suite est conservée telle quelle, bornée à 64 caractères, sans espace ni caractère de contrôle. Le format n'a pas encore été relu sur un client réel.

La connexion réutilise la découverte du client existante (lockfile puis processus), déjà prise en charge sous Windows et macOS. Le mot de passe du client ne quitte pas le cœur Rust.
