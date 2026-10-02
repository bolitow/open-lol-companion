# Imports dans le client LoL — #14, #15, #16

Les commandes Rust sont disponibles pour le branchement de l'écran build (#13).
Elles ne se déclenchent pas au démarrage : l'interface doit les appeler à la suite
d'une action du joueur ou de son réglage d'import automatique, séparément pour
chaque type. Aucun choix de build ni action dans la partie n'est automatisé ici.

## Contrat Tauri

| Commande | Requête de `@olc/shared` | Effet |
| --- | --- | --- |
| `import_draft_runes` | `ImportDraftRunesRequest` | Runes avec garde du contexte de draft |
| `import_draft_spells` | `ImportDraftSpellsRequest` | Sorts avec garde du contexte de draft |
| `import_runes` | `ImportRunesRequest` | Valide puis crée ou remplace la page de l'app |
| `import_spells` | `ImportSpellsRequest` | Place les sorts choisis sur D/F pendant la sélection |
| `import_items` | `ImportItemsRequest` | Insère le set choisi en tête des sets du compte local |

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
Rust dans `crates/lcu-connector/src/imports/`. Les noms des champions et les labels
des blocs d'items sont fournis par l'appelant dans la langue de l'interface.

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

## Items

```json
{
  "championId": 81,
  "championName": "Ezreal",
  "mapId": 11,
  "blocks": [{ "label": "Objets principaux", "items": [{ "id": 3042, "count": 1 }] }]
}
```

Le compte est lu côté Rust via `/lol-summoner/v1/current-summoner`. Son bundle
`/lol-item-sets/v1/item-sets/{summonerId}/sets` est lu puis réécrit en conservant
les sets des autres UID et les métadonnées inconnues. Seul l'UID réservé
`open-lol-companion-{championId}-{mapId}` est remplacé, ce qui évite les doublons
sur réimport du même champion et de la même carte.

Le set est de type `custom`, associé au champion et à la carte, inséré en tête
du tableau avec un `sortrank` supérieur aux autres. Si un set à conserver utilise
déjà le maximum `i32`, l'import échoue sans modifier les sets personnels. L'ordre
réel dans la boutique est confirmé sur macOS ; il reste à vérifier sur Windows.

Conversions des objets Larme, vérifiées dans Data Dragon 16.19.1 :

| Forme évoluée | Forme achetable importée |
| --- | --- |
| Muramana (3042) | Manamune (3004) |
| Étreinte du Séraphin (3040) | Bâton de l'archange (3003) |
| Fimbulvetr (3121) | Approche de l'hiver (3119) |

Les autres identifiants sont conservés. Le nombre d'exemplaires doit être positif ;
un nom ou un bloc vide est refusé avant accès réseau.

## Concurrence et erreurs

Les commandes Tauri sérialisent les imports du même type ; les trois types peuvent
fonctionner en parallèle. Un appel direct à la crate doit lui aussi sérialiser
ses lectures/écritures de pages et de sets. La LCU ne fournit ici aucune transaction
globale : l'interface doit afficher le résultat de chaque import indépendamment.
Une modification manuelle simultanée dans le client peut encore entrer en conflit
avec la lecture puis réécriture des sets.

Chaque requête est bornée à cinq secondes. Les réponses 200/201 et 204 sont
acceptées pour les écritures, les réponses non réussies deviennent des codes
d'erreur. Le connecteur ne suit aucune redirection et n'utilise aucun proxy.
Après une coupure ou un délai dépassé, le résultat d'une écriture peut être
incertain : vérifier l'état du client avant de relancer. Aucun retry automatique
n'est effectué.

## Sources et validation

- [Schéma extrait du client LCU 26.16](https://raw.githubusercontent.com/KebsCS/lcu-and-riotclient-api/main/lcu/swagger.json), endpoints et structures.
- [Catalogue des styles du client](https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perkstyles.json), lignes et fragments.
- [Data Dragon 16.19.1 : items](https://ddragon.leagueoflegends.com/cdn/16.19.1/data/en_US/item.json) et [sorts](https://ddragon.leagueoflegends.com/cdn/16.19.1/data/en_US/summoner.json).
- [Documentation Riot archivée des sets](https://raw.githubusercontent.com/CommunityDragon/HexDocs/master/lol/misc/itemsets.md), rang décroissant.
- [Politique et limites de la LCU](https://developer.riotgames.com/docs/lol#league-client-api) : API locale non supportée officiellement, susceptible de changer à chaque patch.

Les tests utilisent un serveur HTTP local simulé et des données sans identité
réelle ; ils ne modifient pas le client installé. Ils couvrent la validation,
les payloads, les erreurs et la conservation des données personnelles.

Des recettes réelles partielles ont été exécutées sur macOS et Windows le 1er octobre 2026.
Les contrôles restants restent requis avant clôture des tickets : macOS Matthieu,
Windows Louison. Une lecture de la LCU ne confirme pas à elle seule le rendu visuel
dans la boutique.

| Scénario | macOS | Windows |
| --- | --- | --- |
| Créer puis réimporter les runes, vérifier la page active et les fragments | Validé LCU ; page active confirmée visuellement | Validé LCU : page active et neuf identifiants |
| Refuser deux secondaires de la même ligne ; préserver les pages personnelles | Validé dans le client réel | Validé dans le client réel |
| Capacité de pages atteinte, page réservée verrouillée ou dupliquée | À faire | À faire |
| Flash sur D/F et paire sans Flash | Validé par relecture LCU ; sorts initiaux restaurés | Validé par relecture LCU ; sorts initiaux restaurés |
| Skin conservé et sortie de sélection | Non confirmé en réel ; couverture simulée | Skin relu conforme ; sortie de sélection non testée |
| Objets Larme achetables, réimport sans doublon | Validé par relecture LCU | Validé par relecture LCU |
| Set visible en premier dans la boutique | Confirmé visuellement par Matthieu | Non observé ; priorité LCU vérifiée |
| Sets personnels conservés, plusieurs champions et cartes | Simulé seulement ; aucun set personnel dans la recette réelle | Quatre sets personnels conservés ; plusieurs cartes/champions non testés |
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
- **Items (#16)** : set Jinx (`222`) sur la Faille (`11`), réimport sans doublon,
  premier élément du tableau LCU. Conversions réelles `3042 → 3004`, `3040 → 3003`
  et `3121 → 3119`. Matthieu confirme visuellement que le set apparaît en premier
  dans la boutique. Aucun set personnel n'était présent, sa préservation reste simulée.

La version du jeu provient de `GET /lol-patch/v1/game-version`. Aucun identifiant
de joueur ni secret du client n'est conservé dans ces preuves.

## Recette réelle Windows — 1er octobre 2026

Louison a testé les commandes Rust de la PR #57 sur Windows 11 Pro 25H2, client
16.19.823.722 : création d'une page active, neuf identifiants, refus de secondaires
de même ligne, réimport au même ID et préservation des pages personnelles.
Les pages initiales et l'ancienne page active ont été restaurées et relues.
[Compte rendu détaillé](https://github.com/bolitow/open-lol-companion/issues/14#issuecomment-5935051243).

Louison a aussi testé les commandes Rust de la PR #58 en sélection réelle :
Flash D/F, paire sans Flash et skin identique après chaque import ; les sorts
initiaux ont été restaurés et vérifiés.
[Compte rendu des sorts](https://github.com/bolitow/open-lol-companion/issues/15#issuecomment-5935060302).

Pour la PR #59, Louison a vérifié les trois conversions Larme, l'UID unique au
réimport, la conservation de quatre sets personnels et la priorité dans la LCU.
Le bundle initial a été restauré et relu. L'ordre visuel dans la boutique Windows
n'a pas été observé et reste à vérifier.
[Compte rendu des items](https://github.com/bolitow/open-lol-companion/issues/16#issuecomment-5935062015).

Le parcours Tauri/invoke et l'écran #13 restent à tester. Les cas de capacité
pleine, verrouillage et autres modes n'ont pas été reproduits dans le client réel.

Consigner dans chaque ticket : version de l'app et du client, OS, scénario,
résultat et anomalie éventuelle. Le branchement #13 doit notamment désactiver
l'import de runes pour un mode sans runes.

## Branchement de la préparation — #13

La dépendance #14 est reprise localement depuis la PR #57 de Matthieu, commit
`91fd2ce9d401007a1baed41443a567f4f2bdb31b`. Les recettes réelles Windows/macOS
ci-dessus proviennent de cette PR et concernent la commande Rust générique,
pas le nouveau parcours de l’écran. Aucune fusion distante n’est effectuée ici.

L’interface appelle exclusivement `import_draft_runes`, avec :

```ts
{ request: { championId: 103, runes: { championName: "Ahri", primaryStyleId: 8000,
  subStyleId: 8200, selectedPerkIds: [8005, 9111, 9104, 8014, 8233, 8236, 5005, 5008, 5001] } } }
```

Après validation des runes et lecture des pages, Rust relit
`/lol-gameflow/v1/session` et `/lol-champ-select/v1/session`. Il exige
`ChampSelect`, une file 400/420/440, la carte 11 et `CLASSIC`, une draft 5v5 prise
en charge et le même champion local que celui affiché. Le filtre de statistiques
n’entre pas dans cette garde. Le champ `championId` peut provenir d’un prépick
ou d’un pick exposé par le client ; aucun champion n’est sélectionné par l’app.

Le rejet `DraftRuneImportError` reste une chaîne : les codes `ImportError`,
plus `draftContextChanged`, `unsupportedMode` ou `importBusy`. La LCU ne fournit
pas de transaction entre vérification et écriture : ce contrôle réduit la fenêtre
de course sans garantir une atomicité avec un changement dans le jeu.

Côté interface, la page modifiée n’est pas confondue avec les statistiques de la
variante. Réinitialiser reprend la source courante, changer de variante/contexte
repart de cette nouvelle source. Un envoi reste unique même si le panneau est
démonté ; son résultat n’est pas réaffiché dans un autre contexte. Aucun retry
automatique. La réponse HTTP réussie affiche « import accepté » ; le watcher
doit ensuite rapporter une page valide aux mêmes choix pour afficher « équipée ».

La recette du bundle Tauri macOS a été faite avec un serveur LCU synthétique
isolé : création POST, réimport PUT au même ID, page personnelle intacte, attente
de confirmation puis réception du changement, refus de mode malgré le filtre
Solo/Duo, rejet HTTP, déconnexion et reconnexion. Cette recette ne valide ni
les certificats du client réel ni son rendu. Le nouveau parcours avec League
reste à tester sur macOS et Windows, ainsi que les cas de capacité/permissions
du client avant clôture de #13/#14.


## Sorts dans la préparation — #13 / #15 (1er octobre 2026)

Le moteur `imports/spells.rs` est repris sans modification de la PR #58 de
Matthieu, commit `baa355496d16bb0ac502dc20044893e8618db4a1`. Le contrat générique
`import_spells` est conservé. Le front appelle `import_draft_spells` avec
`{ request: { championId, spells: { spellIds: [idD, idF], flashSlot: "D" | "F" } } }`.
La garde mode/champion des runes est partagée puis le moteur #15 vérifie de
nouveau ChampSelect et écrit **uniquement** `spell1Id` et `spell2Id` par PATCH
sur `/lol-champ-select/v1/session/my-selection`. Aucun skin ou pick n’est écrit.
Un verrou partagé par les deux commandes refuse les clics concurrents du front.

`DraftSession.localSpells` projette les deux emplacements du joueur local,
jamais ceux des autres joueurs ; valeur nulle si absente, incomplète ou ambiguë.
Un PATCH accepté reste « confirmation en attente » tant que le watcher ne
rapporte pas la même paire ordonnée. Le choix Flash D/F n’est ni inventé ni
inféré d’un build : action explicite et préférence locale `olc.flash-slot`.
Modifier seulement l’autre sort ne choisit pas cette préférence. Sans Flash,
l’ordre choisi est préservé. Aucune importation automatique ou retry.

Le catalogue ne propose que les sorts marqués CLASSIC. Il ne connaît pas le
niveau ni les sorts débloqués du compte : le client reste l’autorité et ses
refus sont traduits sans réponse brute. Les garde-fous vérifient les files
400/420/440 et le champion au dernier moment, sans prétendre rendre le contexte
et le PATCH atomiques.

Recette du bundle Tauri macOS avec **LCU synthétique isolée** : choix F, PATCH
exact `[14,4]`, attente de confirmation, retour watcher concordant et refus d’un
mode non pris en charge malgré Solo/Duo dans les filtres. Tests Rust : D/F,
sans Flash, invalides, rejet, phase/mode/champion périmés. Les recettes Rust
réelles antérieures de #15 ne valident pas ce nouveau parcours UI.

Le chat « Test Windows » a été sollicité : League disponible sur la tour,
toolchain de recette à reconstituer. Aucun canal de synchronisation commun
Mac/Windows trouvé (OneDrive sur la tour seulement). Recette UI avec **League
réel encore à faire sur Windows et macOS** ; League était fermé sur le Mac.
Ne pas clôturer #13/#15 sur la base de cette recette synthétique.

## Intégration du set d’objets (#13, #16)

Le front importe explicitement la variante consultée sur la Faille (carte 11), même hors draft si League est connecté. L’ordre et les répétitions des achats sont conservés. Une variante vide ou contenant un ID absent du catalogue est refusée entièrement. Les catégories indépendantes ne sont pas assemblées en prétendue recommandation situationnelle.

Moteur repris sans changement de la PR #59, head `cf04f75` : UID propre à l’app/champion/carte, autres sets préservés, formes évoluées de Larme converties en objets achetables. Un succès signifie que League a accepté le PUT ; la priorité visuelle en boutique reste à vérifier en partie. Aucun import automatique.

## Personnalisées Faille (#12, #13, #14, #15)

Le watcher et la garde d’écriture partagent la classification du gameflow. `gameData.isCustomGame = true` et `map.id = 11`, `map.gameMode = CLASSIC` autorisent une sélection privée avec 1 à 5 alliés et 0 à 5 adversaires, bots compris. Une file inconnue, une autre carte ou un mode alternatif reste refusé. La lecture de contexte est bornée dans le temps ; les événements de gameflow l’actualisent sans polling supplémentaire. La garde relit toujours mode et champion avant chaque import. Les statistiques consultées gardent leur filtre normal/classé explicite : elles ne deviennent pas des statistiques de personnalisées.

ARAM, entraînement, coopératif et modes temporaires restent suivis séparément dans les tickets.

## Synchronisation du 2 octobre 2026

Les PR #57, #58 et #59 sont désormais intégrées depuis `main`. Les moteurs génériques restent ceux de Matthieu ; les gardes de contexte et le support des personnalisées du desktop sont conservés. Les références aux reprises locales ci-dessus décrivent leur intégration initiale.
