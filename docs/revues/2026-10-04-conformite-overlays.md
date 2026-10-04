# Revue de conformité Riot des overlays et projections — 4 octobre 2026 — #30

Revue de l'existant (branche `docs/30-revue-conformite-overlays`, base : projection Live Client par liste blanche #102 / #137). Le détail des fonctions refusées et des cas à soumettre à Riot est dans [`rules/conformite-riot.md`](../../rules/conformite-riot.md) ; cette revue en applique les critères ligne à ligne.

## Méthode et limites

- Périmètre : panneau natif en partie, écran « En partie », projection Live Client (dont `postgame`, le bilan), projection de la sélection des champions, mécanique native de l'overlay, prototype. Les imports dans le client (runes, sorts, items) relèvent d'[`imports-automatiques.md`](../imports-automatiques.md) et ne sont pas revus ici.
- Pour chaque donnée : source, visibilité dans le client ou le jeu, décision, texte Riot qui la fonde. Une donnée **projetée mais non affichée** est tout de même jugée : c'est ce que l'interface peut atteindre sans modifier Rust.
- « Visible dans le client » est déduit de la documentation du Live Client Data API et du comportement connu du HUD ; **aucune capture n'a été comparée donnée par donnée à l'écran de jeu**. Chaque ligne marquée « à confirmer » demande une vérification en partie réelle (recette) avant la soumission à Riot.
- Textes Riot cités : ceux déjà relus et consignés dans `rules/conformite-riot.md` (Developer API Policy > Game Integrity ; Game Policy, « Approved » et « Unapproved Use Cases »). Fragment complémentaire, relevé le 4 octobre 2026 par une lecture assistée de docs/lol (à recouper sur la page à la prochaine relecture manuelle) : parmi les usages approuvés figurent « Game overlays that provide static data that is available prior to the game », « Showing (self) player stats » et « Aggregate player stats (no specific players) ».
- Abréviations des textes : **GI** = « Game Integrity » (« Products must not use or incorporate information not present in the game client that would give players a competitive edge ») ; **GI-anon** = « Products cannot identify or analyze players who are deliberately hidden by the game » ; **GP-inconnu** = Game Policy, « game-session-specific information that would be previously unknown to the player » ; **GP-décision** = Game Policy, « Apps that dictate player decisions » ; **GP-approuvé** = usages approuvés cités ci-dessus ; **GP-augments** = « Products cannot display win rates for Augments or Arena Mode items ».
- Décisions : **Conforme** (affichable), **Conforme, non affiché** (projeté pour un usage futur, aucune interface ne le montre), **À soumettre** (suspendu, voir `rules/conformite-riot.md`), **Refusé**.

## 1. Panneau natif en partie (`OverlayView`)

| Donnée affichée | Source | Visible dans le client ? | Décision | Texte Riot |
| --- | --- | --- | --- | --- |
| Marque « Open LoL Companion » | Constante de l'app | Sans objet (identité du produit, pas de logo ni de style Riot) | Conforme | Checklist « pas d'imitation de l'interface de Riot » |
| Champion, niveau du joueur local | Live Client `allPlayers[local]` | Oui : HUD du joueur | Conforme | GP-approuvé (« Showing (self) player stats ») |
| K/D/A, CS du joueur local | Live Client `scores` | Oui : HUD et tableau des scores. Les CS peuvent progresser par paliers de 10 (note affichée) | Conforme | GP-approuvé (self) |
| Temps de jeu | Live Client `gameData.gameTime` | Oui : HUD | Conforme | GP-approuvé |
| Note « CS par paliers de 10 » | Texte statique | Sans objet | Conforme | Aucune donnée de jeu |
| Inventaire final le plus joué, fondamentale la plus jouée | API Open LoL Companion (agrégats Riot), contexte de la dernière draft réelle | Donnée statique agrégée, connue avant la partie ; elle ne dépend d'aucun événement de la partie | Conforme | GP-approuvé (« Aggregate player stats (no specific players) », données statiques disponibles avant la partie) ; GP-décision respectée (aucune adaptation aux événements, deux catégories indépendantes) |
| « Victoire observée » (%), effectif de parties, seuil d'échantillon | Idem | Idem. Restreint à la Faille (carte 11, mode CLASSIC) : jamais en Arena, ARAM ni Mayhem | Conforme | GP-augments non applicable : aucun augment ni objet Arena (garde `liveBuildContext`, test `liveBuildRequest` carte 12 → `null`) |
| Plateforme, file, poste, patch, mention « personnalisée · Solo/Duo » | Contexte de draft + service | Oui : choix du joueur dans la sélection | Conforme | GP-inconnu non applicable (information du joueur) |
| Pied de page « Source : API Open LoL Companion » | Texte statique | Sans objet | Conforme | Aucune donnée de jeu |
| Aperçu de 30 s hors partie | Texte statique, aucune partie simulée | Sans objet | Conforme | Aucune donnée inventée |

Observation : le panneau n'affiche pas la mention légale « isn't endorsed by Riot Games ». Elle figure dans la coquille de l'application (`copy.ts`, `legal`) et le README. La mention dans le panneau lui-même est une décision produit (voir questions ouvertes), pas un écart constaté.

## 2. Écran « En partie » (`InGameScreen`, `LiveBuilds`, `GameDetails`)

| Donnée affichée | Source | Visible dans le client ? | Décision | Texte Riot |
| --- | --- | --- | --- | --- |
| Résumé du joueur local (même contenu que le panneau) | Live Client | Oui | Conforme | GP-approuvé (self) |
| Builds complets en lecture seule (variantes, effectifs, taux publiés, patch, poste, population) | API Open LoL Companion | Statique agrégé, connu avant la partie | Conforme | GP-approuvé (agrégats) ; aucun bouton d'import en partie |
| Fiches de détail (objets, runes, sorts) | Catalogue Data Dragon / CommunityDragon | Oui : données statiques du jeu | Conforme | Checklist « données statiques issues de Data Dragon / CommunityDragon » |
| Dates de collecte et de publication, source | Métadonnées du service | Sans objet | Conforme | Aucune donnée de jeu |
| Contexte manquant : message « statistiques indisponibles » | Texte statique | Sans objet | Conforme | Le poste ou la file n'est jamais deviné |

## 3. Sélection des champions (`DraftBoard`, `PreparationPanels`)

Projection Rust `DraftSession` ([`crates/lcu-connector/src/draft.rs`](../../crates/lcu-connector/src/draft.rs)) : seule la Faille (files 400, 420, 440 et personnalisée carte 11) est prise en charge ; tout autre mode est marqué non pris en charge et rien n'est affiché.

| Donnée projetée / affichée | Source | Visible dans le client ? | Décision | Texte Riot |
| --- | --- | --- | --- | --- |
| Champion et intention des alliés (survol) | LCU `myTeam[].championId`, `championPickIntent` | Oui : les alliés voient les survols de leur équipe | Conforme | GI |
| Champion adverse **seulement une fois verrouillé** (une action `pick` terminée) | LCU `theirTeam[]` + `actions` | Oui, une fois annoncé. Avant verrouillage, l'intention éventuelle divulguée par l'API est forcée à vide en Rust | Conforme | GI ; GI-anon ; test `masque_les_intentions_adverses_et_le_champion_avant_verrouillage` |
| Poste assigné des alliés ; `local` (cellule du joueur) | LCU `assignedPosition`, `localPlayerCellId` | Oui pour l'équipe du joueur | Conforme | GI |
| Poste des adversaires | LCU | Non affiché par le client : forcé à vide en Rust | Refusé (non projeté) | GI ; GI-anon ; test `accepte_une_draft_cinq_contre_cinq_et_masque_les_postes_adverses` |
| `cellId` (index de cellule, non un identifiant de joueur) | LCU | Position de l'emplacement dans la sélection, sans identité | Conforme | GI-anon (aucune identité) |
| `acting` (joueur en train de pick ou ban) | LCU `actions[].isInProgress` | Oui : l'emplacement actif est mis en évidence par le client | Conforme | GI |
| `locked` | LCU `actions[].completed` | Oui | Conforme | GI |
| Bans alliés | LCU `bans.myTeamBans` | Oui | Conforme | GI |
| Bans adverses | LCU `bans.theirTeamBans` | Oui une fois annoncés. **À confirmer en recette** que le client n'en renseigne aucun avant l'annonce : aucun filtre défensif n'existe côté Rust (contrairement aux intentions) | Conforme, à confirmer | GI |
| Chronomètre de phase (`remainingMs`, `observedAtMs`) | LCU `timer`, hors chronomètre infini | Oui : horloge de la sélection | Conforme | GI |
| Côté bleu / rouge | LCU `myTeam[].team` | Oui | Conforme | GI |
| `gameId`, `queueId`, `customGame`, `supported` | Gameflow LCU | Oui (métadonnées de la partie ; l'identifiant sert à lier la draft, jamais affiché comme identité) | Conforme | GI |
| Deux sorts d'invocateur du **joueur local** (`localSpells`) | LCU `myTeam[local].spell1Id/spell2Id` | Oui : ses propres sorts. Ceux des autres joueurs ne sont jamais lus | Conforme | GP-approuvé (self) ; test `exporte_seulement_les_deux_sorts_du_joueur_local` |
| `puuid`, `gameName`, `tagLine`, `summonerName`, `summonerId`, fil de discussion | LCU (si divulgués) | Cachés par la sélection anonymisée | Refusé (jamais projetés, ni IPC, ni service, ni journal) | GI-anon. Test de garde ajouté par #30 : `aucune_identite_des_joueurs_ne_sort_de_la_projection` (canaris + liste blanche des clés) |
| Clic sur un champion adverse verrouillé → builds de ce champion | Sélection locale du joueur, catalogue + agrégats | Le champion est visible ; l'écran montre des builds statiques agrégés, pas un matchup | Conforme | GP-approuvé (agrégats) ; GP-décision respectée |
| Panneaux runes, objets, sorts de préparation | Catalogue, builds agrégés, page de runes du joueur | Statique et propre au joueur | Conforme | GP-approuvé ; aucune action dans le client sans demande du joueur |
| Mode Arena ou ARAM Mayhem : aucune préparation | Garde `supported` | Sans objet | Conforme | GP-augments : aucun taux d'augment ni d'objet Arena n'est calculé ni affiché |

## 4. Projection Live Client (`LiveSession`)

Liste blanche dans [`crates/lcu-connector/src/live/mod.rs`](../../crates/lcu-connector/src/live/mod.rs), test `un_champ_hors_liste_blanche_n_est_jamais_projete` (canaris, interdits, liste de clés). Types miroirs dans `packages/shared/src/live.ts`.

| Donnée projetée | Source | Visible dans le client ? | Affichée aujourd'hui ? | Décision | Texte Riot |
| --- | --- | --- | --- | --- | --- |
| `championKey`, `level`, `kills`, `deaths`, `assists`, `creepScore`, `items` du **joueur local** | Live Client | Oui : HUD du joueur | Oui sauf `items` | Conforme | GP-approuvé (self) |
| `currentGold` (local) | `activePlayer.currentGold` | Oui : or courant du HUD | Non | Conforme, non affiché | GP-approuvé (self) ; or exact adverse jamais projeté |
| `wardScore` (local) | `scores.wardScore` | Oui : sa propre vision | Non | Conforme, non affiché | GP-approuvé (self) |
| `isDead`, `respawnTimer` (local) | Joueur local | Oui : écran de mort du joueur | Non | Conforme, non affiché | GP-approuvé (self) |
| `abilityLevels` Q/W/E/R (local) | `activePlayer.abilities` | Oui : points de compétence | Non | Conforme, non affiché | GP-approuvé (self) ; aucun cooldown ni recharge projeté |
| `team`, `position` (local) | Joueur local | Oui : côté de la carte, rôle assigné | Non | Conforme, non affiché | GP-approuvé (self) |
| `teams.allies` / `teams.enemies` : totaux K/D/A et CS | Somme des `scores` par camp, calculée en Rust | K/D/A et CS de chaque joueur sont au tableau des scores (Tab). **À confirmer en recette** que le CS adverse y est bien visible | Non | Conforme, non affiché. Aucune comparaison à l'adversaire direct (voir « À soumettre ») | GI ; GP-inconnu (sous réserve de confirmation) |
| Or exact des adversaires | Absent du Live Client (seul `activePlayer.currentGold` est exposé) | Non | Non | Refusé (ne peut pas être projeté) | GI ; GP-inconnu |
| Positions des adversaires sur la carte, vision (wards) adverse non vue | Absentes du Live Client (aucune coordonnée ; `position` désigne le rôle) | Non | Non | Refusé (jamais projetés) | GI (« enemy ultimate cooldowns » en exemple) ; GP-inconnu |
| Objets, sorts d'invocateur (sans recharge), niveau et score de vision des adversaires | Live Client `allPlayers[]` (présents dans la source) | Oui au tableau des scores (Tab) selon le cahier §6 et la documentation ; **à confirmer en recette** | Non | À soumettre (non projetés aujourd'hui, choix de liste blanche #102). Recharges des sorts adverses : refusées | GI ; GP-inconnu (sous réserve de confirmation) |
| `events` : GameStart, MinionsSpawning, GameEnd, ChampionKill, FirstBlood, DragonKill, HeraldKill, BaronKill, TurretKilled, InhibKilled, Ace | Live Client `events` | Oui : annonces publiques à tous (fil d'éliminations, annonces d'objectifs). Sans nom de joueur, ni type de dragon, ni détail de vol | Non | Conforme, non affiché. Les versions nominatives (auteur, victime, type de dragon, vol) sont à soumettre avant tout affichage | GI. Les événements non annoncés à tous (Multikill, FirstBrick, Horde, Atakhan…) sont écartés |
| `ally`, `involvesLocalPlayer` par événement | Dérivés en Rust (camp relatif, joueur local auteur, assistant ou victime) | Dérivé d'annonces publiques | Non | Conforme, non affiché | GI-anon non applicable (aucune identité) |
| `gameTime`, `gameMode`, `mapNumber` | Live Client `gameData` | Oui | Temps seul | Conforme | GP-approuvé |
| `context` (champion verrouillé, poste, plateforme, file, personnalisée) | Dernière draft réelle + compte actif | Oui : choix du joueur | Via les builds | Conforme | GP-inconnu non applicable |
| `status`, `revision`, `generation` | Technique | Sans objet | Messages d'état | Conforme | Aucune donnée de jeu |

Identité : le nom du joueur local ne sert qu'à le retrouver dans la liste ; les noms des événements ne servent qu'à décider d'un booléen, puis sont jetés. Une identité ambiguë invalide la lecture plutôt que de deviner.

## 5. Bilan (`postgame`)

| Donnée | Source | Visible dans le client ? | Décision | Texte Riot |
| --- | --- | --- | --- | --- |
| Dernière lecture valide de la partie terminée (même forme que `game`) | Mémoire du producteur Rust, aucune écriture disque | Mêmes données que pendant la partie, déjà connues du joueur à la fin | Conforme, **non affiché** : aucun écran ne consomme `postgame` aujourd'hui (types et tests seulement) | GI ; GP-approuvé (self). Purgée au début d'une nouvelle partie et à la sortie des écrans d'après-partie |
| Écran de bilan Arena | Cahier §5.5 | Non implémenté | Garde-fou : aucun taux, tier ni note d'augment ou d'objet Arena | GP-augments |

Condition pour afficher `postgame` : seules les données de la partie du joueur et ses propres statistiques ; tout rapprochement de performance d'augments reste refusé.

## 6. Mécanique native de l'overlay

| Point | Constat dans le code | Décision | Texte Riot |
| --- | --- | --- | --- |
| Injection dans le processus du jeu | Aucune : fenêtre Tauri indépendante (NSPanel non activant sur macOS ; fenêtre transparente topmost non activante sur Windows) | Conforme | « Injection de code dans le processus du jeu » interdite (cahier §2) ; checklist « fenêtres externes uniquement » |
| Lecture mémoire, capture d'écran, OCR | Aucune (`OpenProcess` Windows avec `PROCESS_QUERY_LIMITED_INFORMATION` pour lire le nom de l'exécutable ; `NSWorkspace` et CoreGraphics pour le PID et le cadre de fenêtre sur macOS) | Conforme | GI (aucune information hors du client) |
| Entrées simulées ou interceptées | Aucune ; les clics traversent le panneau (`set_ignores_mouse_events`), pas de hook clavier hors raccourci global Ctrl + Alt + O de l'app | Conforme | Checklist « aucune action automatique dans le jeu » |
| Source des données | GET fixe `https://127.0.0.1:2999/liveclientdata/allgamedata`, sans authentification ni proxy, sans redirection | Conforme | Cahier §2 « Live Client Data API (port 2999) pendant la partie » autorisé |
| ACL de la fenêtre `game-overlay` | Écoute d'événements, `live_session`, état, builds communautaires, hauteur de contenu ; aucune commande d'import ni de profil | Conforme | Frontière Rust / interface (AGENTS.md §0) |
| Données transmises à la fenêtre `game-overlay` | L'événement `live-session` est diffusé en entier ; `teams`, `events` et `postgame` y sont donc lisibles bien que non affichés | Conforme, à surveiller : tout affichage futur passe par cette revue | GI |
| Publicité, imitation de l'interface de Riot | Aucune | Conforme | Politique de mai 2025 ; checklist |
| Version gratuite | Produit entièrement gratuit | Conforme | Cahier §2 |

## 7. Prototype (`prototype.html`, `DraftPage`)

| Élément | Constat | Décision | Texte Riot |
| --- | --- | --- | --- |
| Écart d'or « +320 », prochain objectif « Dragon · 1:24 », « Votre fenêtre : niveau 6 », suggestions, courbe de puissance | Données **fictives** scénarisées, libellées « scénario illustratif · recommandations et timings fictifs » ; aucune lecture du jeu | Hors conformité en l'état (aucune donnée de jeu). Les mêmes fonctions câblées à du réel passent par #30 | GI ; GP-décision ; cas « À soumettre » |
| Bouton « Verrouiller · simulation » et « Simuler l'import » | Change un état local ; aucun appel au client, aucun pick ni verrouillage envoyé | Conforme (simulation) ; un vrai bouton Lock reste refusé | Fonctions refusées : bouton Lock |
| Livraison | `vite.config.ts` construit `prototype.html` dans `dist` : la page est empaquetée dans l'installeur, sans lien depuis la fenêtre principale (écran `index.html`) | À trancher (question ouverte) : l'exclure du build de production évite qu'une maquette de fonctions non validées soit livrée | GI |

## 8. Écarts constatés et suites

- **Corrigé dans cette PR** : le test de la projection de draft ne vérifiait que `summoner` et `chat` et n'injectait aucune identité. Le test `aucune_identite_des_joueurs_ne_sort_de_la_projection` injecte `puuid`, `gameName`, `tagLine`, `summonerName`, `summonerId` et fil de discussion dans les deux équipes, vérifie leur absence de la sortie sérialisée et fige la liste des clés de `DraftSession` et de `DraftPlayer`. Il est vert sur le code actuel (l'invariant tenait déjà) ; rouge vérifié par mutation (ajout temporaire d'un champ `puuid` à `DraftPlayer`, retiré ensuite).
- **Aucun autre écart de code constaté** : pas de donnée adverse cachée, pas de MMR, pas de statistique d'augment ou d'objet Arena, pas d'automatisation de décision dans le périmètre revu.
- **Non corrigés, listés** : filtre défensif des bans adverses (à décider après recette), mention légale dans le panneau, exclusion du prototype du build de production, confirmation en partie réelle de la visibilité du CS adverse au tableau des scores.
- **Cas à soumettre à Riot** : voir la section dédiée de `rules/conformite-riot.md` ; aucun n'est développé tant que #2 et #30 n'ont pas de décision écrite.
- **Corrigé au tour 1 de revue** : les lignes « objets, sorts, vision, positions adverses » et « événements nominatifs » classées à tort en fonctions refusées sont requalifiées. Le refus ne couvre plus que ce que les textes couvrent (or exact adverse, positions et vision adverse non vues, recharges) ; objets, sorts (sans recharge), niveau adverse et événements nominatifs sont « à soumettre », non projetés aujourd'hui (choix de liste blanche). La différence d'or est formulée « à confirmer en partie réelle », dans les termes du cahier §6.
- **Décisions à valider par Matthieu** : (1) les objets et sorts adverses visibles au tableau des scores restent « à soumettre » et ne sont pas projetés tant que #2 et #30 n'ont pas tranché ; (2) la vision adverse est scindée : wards non vus refusés, score de vision au tableau des scores à confirmer ; (3) la source de la différence d'or (valeur des items adverses) reste à valider, l'or adverse exact étant refusé.
