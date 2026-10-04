# Recette étendue #18 — 1er octobre 2026

## État

Recette bornée terminée le 2 octobre 2026 : la collecte s'est suspendue sur un
refus d'authentification Riot, après environ 23 h 27 sur les 24 h autorisées.
Le dernier agrégat du runner est publié ; les contrôles hors ligne et le bilan
final figurent en fin de document. Le ticket #18 a été clôturé le 2 octobre
2026 à 07:55 UTC ; ce document publie le bilan daté de cette recette. Les sections
initiales ci-dessous conservent leurs chiffres historiques, datés du 1er octobre.
Les imports #14–16, l’API #19 et le référentiel #61 restent des livrables séparés.

## Patches et données statiques

Vérification des sources Riot le 1er octobre : patch public **26.19**, précédent
**26.18**, correspondant aux versions Data Dragon **16.19.1 / 16.18.1**.

- [Notes du patch 26.19](https://www.leagueoflegends.com/en-us/news/game-updates/league-of-legends-patch-26-19-notes/)
- [Versions Data Dragon](https://ddragon.leagueoflegends.com/api/versions.json)
- [Realm EUW](https://ddragon.leagueoflegends.com/realms/euw.json)
- [Documentation Riot](https://developer.riotgames.com/docs/lol)

Synchronisation publique réelle : 504 documents pour 16.19.1 et 496 pour 16.18.1,
FR/EN, 173 champions standard par patch, respectivement 72/68 champions Classic.
Les index, détails Q/W/E/R/passif, objets, runes, sorts d’invocateur, cartes et
icônes sont conservés. Les images restent des références CDN.

Premier cache publié à 07:25 UTC ; une seconde synchronisation réutilise les deux
bundles, conserve `completed_at` et avance la vérification du manifeste. Les tests
couvrent aussi téléchargement incomplet, échec de commit et conservation du cache.

## Environnement et essai initial

- CLI Rust native macOS, PostgreSQL 17 dans un conteneur Linux dédié.
- Base `olc18`, conteneur `olc-ticket18-rehearsal`, port local 55418, volume persistant.
- Copie des 1 000 parties existantes ; la base originale du #17 est conservée intacte.
- Configuration locale ignorée par Git, fichier accessible seulement au propriétaire.
- Aucune donnée nominative ni clé dans ce document ou les rapports publiables.

Run #2 : EUW1, toutes les files, patches 16.19/16.18, seeds Iron et Challenger,
15 joueurs de départ, cible 100 parties, observations des rangs activées.
**100 nouvelles parties et 100 timelines obtenues**, deux remakes dans ce lot,
**1 071 appels** en **970 secondes** (07:20:26–07:36:35 UTC).
Des réponses 429 ont déclenché la pause demandée puis une reprise ; aucun échec
final de détail ou timeline. Le bilan annonce 63 découvertes non téléchargées
après atteinte de la cible, pas une collecte exhaustive.

## Agrégation après correction des formats réels

Sur les deux patches sélectionnés : **640 parties sources**, **624 incluses**,
**16 remakes exclus**, aucune partie invalide restante. Les 460 parties d’autres
patches de la copie initiale restent stockées, hors sélection.

| Patch technique | Plateforme | File | Parties incluses |
| --- | --- | --- | --- |
| 16.18 | EUW1 | 400 | 7 |
| 16.18 | EUW1 | 420 | 212 |
| 16.18 | EUW1 | 440 | 4 |
| 16.19 | EUW1 | 400 | 3 |
| 16.19 | EUW1 | 420 | 361 |
| 16.19 | EUW1 | 440 | 15 |
| 16.19 | EUW1 | 480 | 12 |
| 16.19 | EUW1 | 880 | 2 |
| 16.19 | EUW1 | 1740 | 7 |
| 16.19 | EUW1 | 1750 | 1 |

Le rapport expose 2 616 groupes champion/rôle/rang, 462 lignes de bans,
72 593 variantes de builds publiées, 52 880 agrégats de points de compétence et
350 038 agrégats d’événements d’objets. 5 553 variantes de builds supplémentaires
ne sont pas publiées à cause du plafond de 20 par catégorie/groupe ; leurs sources
brutes sont conservées, les dénominateurs restent complets.

6 294 participations humaines, toutes avec timeline exploitable ; dix participations
de bots sont comptées séparément et exclues des statistiques humaines.
Classement : 651 observations classées, neuf non classées, 5 260 rangs inconnus
(essentiellement le jeu de données initial), 374 participations de modes non classés.
Le rang observé est récent, jamais présenté comme le MMR ou le rang historique.

**Aucun groupe n’atteint encore le seuil de 100 parties par champion et dimension** :
les comptes sont disponibles, les taux et tier lists restent masqués. La collecte
longue augmente l’échantillon sans garantir des effectifs suffisants dans chaque
région, rôle et elo. Un seuil plus bas permet l’exploration mais ne rend pas cet
échantillon représentatif.

Cas réels corrigés avec tests de régression :

- Arena 1740/1750 : 18 participants en six sous-équipes de trois.
- Coop 880 : cinq humains dans les métadonnées, dix fiches avec les bots adverses.
- Événements système `participantId=0` ignorés.
- 55 remboursements d’or sans ID d’objet concernent 54 participations : compétences
  conservées, ordre net des achats omis pour ces participations, compteur explicite.
- Les victoires/winrates d’items Arena et tout tri lié sont masqués suivant la
  [politique Riot](https://developer.riotgames.com/docs/lol#game-policy).

## Vérifications indépendantes

Des requêtes SQL directement sur les réponses brutes ont reproduit exactement les
**1 125 groupes ALL** (participations et victoires) et les **462 comptes de bans**.
Les identités joueurs ne sont pas exportées. Invariants vérifiés : comptes de
couverture, victoires + défaites = participations, sources = inclusions + exclusions,
masquage des performances d’items Arena.

Les rapports locaux sont dans `target/aggregation-rehearsal.json` et
`target/audit-ticket18.json`, ignorés par Git. Taille du premier JSON : environ
86 Mo. La mesure release après les correctifs atteint 562 Mo de RSS maximal,
71 s en concurrence avec les tests PostgreSQL. Le second calcul isolé dure
**27,49 s**, maximum RSS **560 873 472 octets** (environ 535 Mio).
Les deux JSON sont strictement identiques, SHA-256
`040eec232002e6b8c3185fb8c319f2f1cb0b1fe969d112751ba96e27ffd44c47`.
La sérialisation SQLx évite de construire un second arbre JSON complet.
La lecture par lots ne borne pas les compteurs de variantes ni les buffers finaux :
une montée en charge importante reste à mesurer pendant la campagne.

## Validation du code

Commande exécutée avec PostgreSQL réel :

```sh
OLC_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55418/postgres pnpm test
pnpm lint
```

**178 tests réussis** : 5 TypeScript, 19 connecteur LCU, 102 unitaires collecteur,
11 intégration agrégation, 12 campagne, 23 pipeline et 6 statiques.
Aucun test PostgreSQL ignoré ; typage, format et Clippy sans avertissement.
Les journaux de preuve finaux sont `target/ticket18-pr-tests.log` et `target/ticket18-pr-lint.log`.

Tests rouges puis verts observés pour les nouveaux comportements et anomalies :
routage et files, rangs, formats, synchronisation/cache, métriques et timelines,
filtrage CLI, priorité des refus de clé même à la limite de budget/durée.
Tests de concurrence, annulation, rollback et reprise exécutés sur bases jetables.

La validation finale a révélé une transaction laissée ouverte après annulation de
`BEGIN`, correspondant au [défaut SQLx confirmé](https://github.com/transact-rs/sqlx/pull/4394).
Le test déterministe reproduit `idle in transaction` avant correction, puis `idle`
après remplacement de la connexion incertaine. Toutes les ouvertures de transaction
du collecteur, de la campagne, des statiques et des agrégats utilisent ce garde ;
un second test confirme que les connexions saines restent réutilisées.

Auto-revue du diff et revue indépendante : **OK avec réserves** de dimensionnement
mémoire, finalisation SQL après échéance et validation Windows native encore à faire.
Le refus d’authentification prime désormais sur les limites de collecte ; un filtre
de plateforme erroné échoue avant publication et conserve l’ancien rapport.
Aucune nouvelle dépendance ni commande Tauri ; aucun contrat `@olc/shared` modifié.

## Campagne longue — historique des incidents

### Incident du deuxième recalcul horaire

Le calcul de 09:44 UTC a échoué à 09:45:35 : la représentation interne du rapport
dépassait la limite PostgreSQL de 268 435 455 octets pour un objet JSONB.
Le premier calcul horaire de 08:44 UTC restait publié (869 sources, 851 incluses,
18 remakes). La collecte et les quotas continuaient normalement ; aucun 401/403.

Le correctif #18 répartit les six listes en morceaux de 512 entrées / 1 Mio de JSON
au maximum et publie tête et morceaux atomiquement. Les métadonnées et le schéma
JSON public sont conservés. Le lecteur API accepte les anciens et nouveaux formats,
filtre chaque morceau en SQL et reconstruit les listes côté Rust. Les tests couvrent
les limites, l'ordre, l'équivalence, la publication vide et les échecs partiels/au commit.
Un ancien binaire ne peut pas écraser silencieusement un instantané au nouveau format.
Ce changement évite aussi le buffer de sérialisation du rapport entier à la publication ;
l'accumulateur reste proportionnel au volume en mémoire.

Le premier binaire de reprise est construit à part depuis le commit `5467fa4`,
avec ce seul correctif. Il sera remplacé à 11:50 UTC par un binaire compatible avec
les migrations 6 et 7, comme décrit ci-dessous.

Recalcul de reprise réussi sur un instantané pris à **10:17:44 UTC** : **1 273 sources,
1 253 incluses, 20 remakes**, aucun autre rejet. Durée **51,27 s**, pic RSS
**753 975 296 octets** (environ 719 Mio), aucun swap mesuré.
Les six listes sont publiées intégralement en **2 538 morceaux** : 13 009 groupes,
2 855 bans, 209 898 builds, 222 754 niveaux de compétences, 849 120 événements
d'objets et 102 lignes de couverture. Contrôle SQL indépendant : **12 835 participations**
pour les groupes ALL comme pour la couverture, aucune incohérence victoires/défaites,
aucune performance d'objet Arena exposée. La base occupe alors environ 456 Mo.

Validation locale du correctif et de sa compatibilité API : **261 tests réussis**
(253 Rust, 8 TypeScript, y compris les autres chantiers présents dans le dépôt),
`pnpm lint` sans avertissement. Tests rouges observés avant correction : ancien stockage
non découpé et lecteur API incapable de lire les morceaux. Preuves locales ignorées :
`target/ticket18-chunks-full-tests.log`, `target/ticket18-chunks-lint.log`,
`target/ticket18-chunks-real.log`. Revue indépendante : OK avec réserve de suivi mémoire.
Le binaire isolé passe aussi **158 tests du collecteur en release** et Clippy sans
avertissement. Le runner reprend la même campagne à **10:20:20 UTC**, avec un binaire
copié dans `target/ticket18-runtime/olc-collector` pour rester indépendant des builds
de développement. L'échéance du 2 octobre à 07:44:36 UTC reste inchangée.
Ces preuves décrivent le correctif testé pendant la campagne, avant publication
du présent compte rendu final.

### Compatibilité des migrations et autres erreurs

À 11:44 UTC, le calcul horaire échoue parce que le binaire isolé ne connaît pas la
migration 6 des quotas partagés, appliquée à 11:22 UTC par le chantier API.
Le binaire de recette est remplacé par une version compatible avec les migrations
6 et 7. La même campagne reprend à **11:50:58 UTC**, sans changer son échéance ;
le recalcul de reprise publie 1 670 sources / 1 643 incluses à 11:51 UTC.
Les recalculs horaires suivants réussissent jusqu'au dernier calcul à l'arrêt.

Deux erreurs HTTP 503 de timeline à 18:51 UTC et une interruption réseau à
23:24 UTC sont suivies de reprises réussies : les trois travaux sont `done`.
Les journaux comptent 378 tentatives reprogrammées après quota, avec pauses de
1, 2 ou 4 secondes. À **07:11:59 UTC le 2 octobre**, deux appels concurrents
reçoivent un HTTP 401 : la campagne passe à `paused / riot_auth_rejected` et le
processus sort avec le code 3. Aucun nouvel essai d'authentification n'est lancé.
Le refus est constaté ; sa cause exacte, expiration ou révocation, n'est pas connue.

Campagne **#1**, démarrée le **1er octobre à 07:44:36 UTC** (09:44:36 Paris),
échéance **2 octobre à 07:44:36 UTC** (09:44:36 Paris). Runs #3 à #17.
Le processus local est détaché du terminal ; `caffeinate` maintient l’activité de
la machine tant que la collecte fonctionne. Fermer le capot ou arrêter Docker
peut toujours interrompre l’essai. Aucun service permanent n’est installé.

Un runner local ignoré par Git (`target/ticket18-runner.py`) lance la collecte,
le recalcul/statique horaire et un dernier calcul à l’arrêt. État :
`target/ticket18-runner-state.json`. Journaux : `target/ticket18-campaign.log`,
`target/ticket18-aggregate.log`, `target/ticket18-runner.log`.
Le suivi Codex a vérifié la campagne toutes les 30 minutes pendant que l’application
restait disponible. Il n'a exécuté aucun second collecteur. Sa désactivation fait
partie de la clôture de cette recette.

## Bilan final — 2 octobre 2026

### Arrêt, volume et réutilisation

La campagne a travaillé du **1er octobre 07:44:36 UTC au 2 octobre 07:11:59 UTC**,
soit environ **23 h 27 min**, avec **89 463 appels Riot**. L'arrêt sur HTTP 401
précède l'échéance de 32 min 37 s. Le runner termine son dernier agrégat à
07:15:46 UTC puis s'arrête, ainsi que son processus de maintien en activité.
La configuration locale devait recevoir une nouvelle clé pour reprendre avant
l'échéance ; aucune reprise n'a été effectuée. L'autorisation de 24 h est échue.
La campagne reste enregistrée `paused / riot_auth_rejected`, sans processus actif.

Les 15 plateformes ont toutes été parcourues. Les cibles de 10 000 parties et
100 000 appels par plateforme étaient des plafonds, pas des résultats garantis.
La campagne utilise 2 325 seeds répartis sur les dix rangs, d'Iron à Challenger.
Elle a découvert 56 394 parties distinctes (57 632 mentions), retenu 8 773 parties,
dont **8 601 nouvellement téléchargées et 172 déjà présentes**. Il reste 46 407
travaux de détail non téléchargés et 111 observations de rang en attente, dont une
en attente de reprise après quota. Aucun travail `running` ou `failed` ne subsiste.
Les 8 601 nouvelles timelines sont disponibles ; aucun échec final de timeline.

La base contient **9 701 parties distinctes**, dont les 1 000 copiées initialement,
100 de l'essai initial et 8 601 de la campagne. **9 241** appartiennent aux patches
sélectionnés ; les 460 autres restent hors agrégation. Le rapport retient
**9 012 parties**, exclut **228 remakes** et **une partie invalide**.
Cette dernière est une partie EUW1, file 1740, sans opposition entre les booléens
de victoire des participants ; elle est exclue intégralement, sans contribution
partielle. Les identifiants des joueurs et des parties ne figurent pas ici.

### Couverture réellement obtenue

Les nombres suivants portent sur les **parties incluses**, donc après exclusions.
La publication vérifiée utilise l'instantané du **2 octobre à 07:40:30 UTC** et
est publiée à **07:42:06 UTC**. Elle couvre 205 combinaisons patch/plateforme/file.

| Plateforme | Patch 16.18 | Patch 16.19 | Total |
| --- | ---: | ---: | ---: |
| BR1 | 136 | 392 | 528 |
| EUN1 | 148 | 371 | 519 |
| EUW1 | 330 | 660 | 990 |
| JP1 | 188 | 457 | 645 |
| KR | 147 | 364 | 511 |
| LA1 | 164 | 392 | 556 |
| LA2 | 134 | 377 | 511 |
| ME1 | 457 | 458 | 915 |
| NA1 | 109 | 459 | 568 |
| OC1 | 146 | 423 | 569 |
| RU | 130 | 411 | 541 |
| SG2 | 119 | 338 | 457 |
| TR1 | 144 | 383 | 527 |
| TW2 | 135 | 477 | 612 |
| VN2 | 185 | 378 | 563 |
| **Total** | **2 672** | **6 340** | **9 012** |

| File | Mode retourné par match-v5 | Parties incluses |
| --- | --- | ---: |
| 400 | CLASSIC — normal draft | 1 007 |
| 420 | CLASSIC — Solo/Duo | 6 011 |
| 440 | CLASSIC — Flex | 1 081 |
| 450 | ARAM | 218 |
| 480 | SWIFTPLAY | 215 |
| 710 | CLASSIC | 50 |
| 870 | SWIFTPLAY — coop IA | 1 |
| 880 | SWIFTPLAY — coop IA | 2 |
| 890 | SWIFTPLAY — coop IA | 10 |
| 1740 | CHERRY — Arena | 106 |
| 1750 | CHERRY — Arena | 308 |
| 3130 | CLASSIC | 3 |

Le mode technique `CLASSIC` ne suffit pas à assimiler toutes ces files : leurs
identifiants restent séparés. Les 13 parties coop IA excluent 65 participations de
bots. Les 93 367 participations humaines retenues ont toutes une timeline exploitable :
zéro timeline manquante ou invalide pour ces participations. Toutes les 9 241
parties sources, y compris les exclusions, ont également une timeline stockée.
Les 686 remboursements sans ID d'objet sont comptés explicitement ; les ordres
nets d'achat ambigus sont omis, les compétences restent utilisables.

| Rang observé / catégorie | Participations humaines |
| --- | ---: |
| BRONZE | 6 799 |
| CHALLENGER | 2 838 |
| DIAMOND | 8 955 |
| EMERALD | 9 560 |
| GOLD | 7 178 |
| GRANDMASTER | 4 320 |
| IRON | 3 484 |
| MASTER | 7 795 |
| PLATINUM | 7 933 |
| SILVER | 6 797 |
| UNKNOWN | 4 810 |
| UNRANKED | 451 |
| UNRANKED_MODE | 22 447 |

Les rangs classés totalisent 65 659 participations. `UNKNOWN` signifie absence
d'observation récente, `UNRANKED` un classement consulté mais non classé, et
`UNRANKED_MODE` un mode sans rang compétitif applicable. Les groupes `ALL`
représentent les mêmes 93 367 participations : ne pas les additionner aux rangs.
10 261 rôles sont inconnus, notamment dans les modes sans affectation de lane ;
aucun rôle n'est inventé. Les rangs sont des observations récentes, jamais un MMR
ni le rang historique au moment de la partie.

### Agrégats, seuils et contrôles

La publication contient **76 467 groupes**, 9 823 lignes de bans,
**1 464 875 variantes de builds**, 1 398 830 lignes de points de compétence et
5 928 543 lignes d'événements d'objets. Ce sont des lignes agrégées, pas autant de
parties distinctes ; les représentations `ALL` et par rang coexistent.
Les huit catégories de builds couvrent objets individuels, inventaires finaux,
trinkets, ordres d'achat, runes, sorts d'invocateur et ordres de compétences
normales/spéciales. **35 339 variantes** dépassent le plafond de 20 par
catégorie/groupe et ne sont pas publiées ; les dénominateurs restent complets.

**Aucun groupe champion ou build n'atteint 100 parties** dans ses dimensions :
le maximum par groupe champion est de 80. Aucun winrate/pickrate champion,
classement ou tier n'est publié. En revanche,
3 615 lignes de banrate ont un dénominateur de draft suffisant. La collecte
valide le pipeline, pas une tierlist exhaustive ou représentative de la population.

L'audit indépendant du snapshot en transaction `REPEATABLE READ READ ONLY`
confirme zéro anomalie : sources = inclusions + exclusions, couverture = inclusions,
partition des rangs = groupes ALL, victoires + défaites = participations,
populations cohérentes, bornes et plafonds des variantes respectés, morceaux
continus. Les **100 528 lignes d'items Arena** ne publient aucune victoire ni
winrate. Cet audit vérifie la cohérence du rapport publié ; il ne reproduit pas
intégralement les calculs à partir de toutes les réponses brutes.

### Idempotence, performances et stockage

Le runner a enregistré **25 recalculs : 23 réussites et deux échecs**, décrits
plus haut, avec conservation du snapshot précédent et reprise réussie. Deux
recalculs hors ligne supplémentaires, sans clé ni appel Riot, vérifient la fin de
recette : **137,60 s puis 135,43 s**, zéro swap signalé. Le maximum RSS mesuré est
respectivement **2 903 752 704** et **3 353 198 592 octets** (environ 2,70 et 3,12 Gio).
Le pic de mémoire déclaré par macOS atteint environ 3,31 Gio. Le découpage résout
la limite d'un objet JSONB, mais l'accumulateur reste proportionnel au volume :
ce coût mémoire doit être traité avant une collecte beaucoup plus importante.

Les observations du run initial expirent entre 07:20 et 07:36 UTC. Le recalcul
hors ligne réduit donc les groupes de rang de 76 712 à 76 467, sans nouvelle partie.
Les deux recalculs hors ligne, commencés à 07:36:56 et 07:40:30, utilisent le même
ensemble d'observations éligibles et produisent des empreintes identiques pour la
tête et chacune des six listes. La prochaine expiration était à 07:46:44 UTC.
L'idempotence est vérifiée pour des entrées **et une fenêtre de validité des rangs
stables** ; un recalcul ultérieur peut légitimement changer les rangs.

> Note postérieure (#80) : cette dépendance à l'heure du calcul est supprimée. Le rang
> est désormais l'observation la plus proche du début de la partie, dans un écart
> configurable (168 h par défaut) ; ce paragraphe décrit le comportement de l'époque.

La comparaison porte sur les empreintes MD5 des morceaux JSONB ordonnés et les
métadonnées, en excluant les horodatages techniques de publication. Le SHA-256 du
résumé ordonné de ces empreintes est
`440d463241e037e34a8b6bfb7b6313cb06d590304ef29d5c85d442f7233dbf75`.
Ce n'est pas l'empreinte d'un export JSON complet.

La dernière publication occupe **17 346 morceaux**, tous de 512 entrées au plus,
pour **112 385 511 octets** de valeurs JSONB compressées. La base occupe environ
**2,41 Gio**, tables, index et espace PostgreSQL compris. Au contrôle de 07:51 UTC
après arrêt, le conteneur utilisait environ 251 Mio. Les données statiques des deux
patches restent présentes et leurs `completed_at` du 1er octobre 07:25 sont inchangés.

Preuves locales ignorées par Git :

- `target/ticket18-final-independent-audit.json` : audit agrégé et limites.
- `target/ticket18-final-operations.json` : appels, réutilisation, journaux et runner.
- `target/ticket18-final-resource-audit.json` : arrêt, travaux restants, timelines, statiques et stockage.
- `target/ticket18-final-fingerprint-stable-1.json` et `stable-2.json` : empreintes.
- `target/ticket18-final-idempotence.log` et `ticket18-final-idempotence-repeat.log` : temps et mémoire.
- `target/ticket18-final-tests.log` et `ticket18-final-lint.log` : validation du dépôt.

### Validation finale et limites restantes

`pnpm test` avec PostgreSQL réel : **328 tests réussis (318 Rust, 10 TypeScript)**,
zéro échec. Deux recettes explicites du référentiel #61 sont marquées ignorées
par défaut ; elles ne font pas partie de la recette de collecte #18. Aucun test
PostgreSQL standard n'a été sauté. `pnpm lint` passe sans avertissement après
reconstruction des artefacts locaux du connecteur LCU, sans changement de code.
Cette finalisation modifie seulement ce compte rendu et `CHANGELOG.md` ; elle
préserve les autres travaux présents dans le répertoire partagé.

Limites : échantillon issu de seeds classés, quotas d'une clé de développement,
refus de clé avant les 24 h, files accessibles mais seulement douze observées,
seuils non atteints par champion/build, données de rang périssables et mémoire
encore proportionnelle aux variantes. Swarm n'est validé que sur fixture.
La Chine continentale n'a pas de route publique Riot couverte par ce collecteur ;
elle n'est pas annoncée comme collectée. Recette exécutée sur macOS et PostgreSQL
Linux ; aucune recette native du client LoL sous Windows n'est revendiquée.
Le dimensionnement production et une nouvelle campagne avec une clé valide sont
à prévoir séparément ; aucune prolongation de cette campagne n'est autorisée ici.

Le ticket #18 a été clôturé le 2 octobre 2026 à 07:55 UTC. La présente livraison
documentaire conserve les résultats historiques et leurs limites ; elle ne relance
aucune collecte et ne modifie pas le statut du ticket. Le suivi automatique avait
été désactivé à la remise du bilan.
