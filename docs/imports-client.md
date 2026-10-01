# Imports dans le client LoL — #14, #15

Les commandes Rust sont disponibles pour le branchement de l'écran build (#13).
Elles ne se déclenchent pas au démarrage : l'interface doit les appeler à la suite
d'une action du joueur ou de son réglage d'import automatique, séparément pour
chaque type. Aucun choix de build ni action dans la partie n'est automatisé ici.

## Contrat Tauri

| Commande | Requête de `@olc/shared` | Effet |
| --- | --- | --- |
| `import_runes` | `ImportRunesRequest` | Valide puis crée ou remplace la page de l'app |
| `import_spells` | `ImportSpellsRequest` | Place les sorts choisis sur D/F pendant la sélection |

Chaque commande reçoit `{ request: ... }`, résout avec `null` (`ImportResult`)
et rejette avec un code `ImportError`. `importErrorMessage(error, "fr" | "en")`
fournit le message traduit, y compris pour une erreur IPC inconnue. Aucun corps
d'erreur LCU, chemin de découverte ou identifiant d'authentification n'est renvoyé.

```ts
import { invoke } from "@tauri-apps/api/core";
import { importErrorMessage, type ImportSpellsRequest, type ImportResult } from "@olc/shared";

const request: ImportSpellsRequest = { spellIds: [4, 14], flashSlot: "F" };
try {
  await invoke<ImportResult>("import_spells", { request });
} catch (error) {
  // À afficher dans l'état d'erreur traduit de l'écran build (#13).
  const message = importErrorMessage(error, "fr");
}
```

Les types de requêtes sont dans `packages/shared/src/imports.ts` et leurs miroirs
Rust dans `crates/lcu-connector/src/imports/`. Le nom du champion est fourni par l'appelant
dans la langue de l'interface.

## Runes

Exemple :

```json
{
  "championName": "Jinx",
  "primaryStyleId": 8000,
  "subStyleId": 8200,
  "selectedPerkIds": [8005, 9111, 9104, 8014, 8233, 8236, 5005, 5008, 5001]
}
```

Le catalogue du client connecté (`GET /lol-perks/v1/styles`) fait autorité :
quatre runes primaires dans l'ordre des lignes, deux secondaires de lignes
distinctes, puis trois fragments dans l'ordre. Les doublons de fragments sont
autorisés lorsque leurs lignes le permettent. Un catalogue incompatible bloque
l'import avant toute écriture ; aucun catalogue figé n'est embarqué en production.

Le préfixe exact `Open LoL Companion : ` est réservé à l'app. La seule page portant
ce préfixe est remplacée par `PUT /lol-perks/v1/pages/{id}`, nommée pour le nouveau
champion et sélectionnée avec `current: true`. Sans page réservée, une page est
créée par `POST /lol-perks/v1/pages`. Plusieurs correspondances ou une page
réservée non éditable bloquent l'import. Aucune page n'est supprimée pour libérer
une place : si la capacité est atteinte, le refus du client est remonté.

La propriété de la page repose sur ce préfixe, pas sur un identifiant enregistré
sur disque : ne pas l'utiliser pour une page à conserver comme page personnelle.
Renommer la page de l'app hors de ce préfixe la conserve et conduit à une nouvelle
création au prochain import.

## Sorts

`spellIds` contient exactement deux identifiants positifs différents. `flashSlot`
est explicitement `"D"` ou `"F"`. Si Flash (4) fait partie de la paire, il est placé
sur la touche choisie ; sinon l'ordre fourni reste inchangé. Flash n'est jamais
ajouté à la paire.

Après vérification de la phase `ChampSelect`, le PATCH de
`/lol-champ-select/v1/session/my-selection` contient seulement `spell1Id` (D)
et `spell2Id` (F). Le skin est conservé. La disponibilité des sorts pour le mode
et le compte est contrôlée par le client ; son refus reste visible pour l'interface.

## Concurrence et erreurs

Les commandes Tauri sérialisent les imports du même type. Un appel direct à la
crate doit lui aussi sérialiser ses lectures/écritures de pages. La LCU ne fournit
ici aucune transaction globale : l'interface doit afficher le résultat de chaque
import indépendamment.

Chaque requête est bornée à cinq secondes. Les réponses 200/201 et 204 sont
acceptées pour les écritures, les réponses non réussies deviennent des codes
d'erreur. Le connecteur ne suit aucune redirection et n'utilise aucun proxy.
Après une coupure ou un délai dépassé, le résultat d'une écriture peut être
incertain : vérifier l'état du client avant de relancer. Aucun retry automatique
n'est effectué.

## Sources et validation

- [Schéma extrait du client LCU 26.16](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json), endpoints et structures.
- [Catalogue des styles du client](https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perkstyles.json), lignes et fragments.
- [Data Dragon 16.19.1 : sorts](https://ddragon.leagueoflegends.com/cdn/16.19.1/data/en_US/summoner.json).
- [Politique et limites de la LCU](https://developer.riotgames.com/docs/lol#league-client-api) : API locale non supportée officiellement, susceptible de changer à chaque patch.

Les tests utilisent un serveur HTTP local simulé et des données sans identité
réelle ; ils ne modifient pas le client installé. Ils couvrent la validation,
les payloads, les erreurs et la conservation des données personnelles.

Une recette réelle partielle a été exécutée sur macOS le 1er octobre 2026.
Les contrôles restants restent requis avant clôture des tickets : macOS Matthieu,
Windows Louison. Une lecture de la LCU ne confirme pas à elle seule le rendu visuel
dans la boutique.

| Scénario | macOS | Windows |
| --- | --- | --- |
| Créer puis réimporter les runes, vérifier la page active et les fragments | Validé LCU ; page active confirmée visuellement | À faire |
| Refuser deux secondaires de la même ligne ; préserver les pages personnelles | Validé dans le client réel | À faire |
| Capacité de pages atteinte, page réservée verrouillée ou dupliquée | À faire | À faire |
| Flash sur D/F et paire sans Flash | Validé par relecture LCU ; sorts initiaux restaurés | À faire |
| Skin conservé et sortie de sélection | Non confirmé en réel ; couverture simulée | À faire |
| Client fermé/redémarré et erreur réseau | À faire | À faire |
| Faille, ARAM, Mayhem et Swiftplay selon fonctionnalités du mode | À faire | À faire |

## Recette réelle macOS — 1er octobre 2026

Environnement : macOS **27.0**, jeu **16.19.8230722**, commandes Rust de cette
livraison (app **0.1.0**). Exécution sur le client connecté en `ChampSelect` ;
confirmation visuelle ensuite pendant la partie. Les tests automatisés ci-dessus
restent indépendants de ce client réel.

- **Runes (#14)** : création de `Open LoL Companion : Jinx`, page active et neuf
  identifiants de runes conformes ; réimport sur le même identifiant sans doublon.
  Matthieu confirme visuellement que la page est active. Les noms, arbres et
  runes des quatre pages personnelles sont conservés. Deux
  secondaires de la même ligne sont refusées sans écriture.
- **Sorts (#15)** : Flash sur D puis F, paire sans Flash (`7`, `14`), valeurs
  relues dans la session du client. Les sorts initiaux sont restaurés après recette.
  La conservation réelle du skin n’est pas confirmée ; elle reste couverte par
  le test du payload qui contient uniquement les deux sorts.

La version du jeu provient de `GET /lol-patch/v1/game-version`. La recette Windows
n'a pas été exécutée. Aucun identifiant de joueur ni secret du client n'est conservé
dans ces preuves.

Consigner dans chaque ticket : version de l'app et du client, OS, scénario,
résultat et anomalie éventuelle. Le branchement #13 doit notamment désactiver
l'import de runes pour un mode sans runes.
