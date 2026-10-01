# Recette étendue #18 — 1er octobre 2026

## État

Code validé localement ; essai authentifié initial terminé. La campagne de 24 h
est en cours, son bilan final reste à établir. Le ticket #18 reste ouvert.
Ce bilan initial accompagne la PR ; la campagne continue pendant la revue.
Les imports #14–16 et l’API #19 restent séparés.

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

## Campagne longue — en cours

Campagne **#1**, démarrée le **1er octobre à 07:44:36 UTC** (09:44:36 Paris),
échéance **2 octobre à 07:44:36 UTC** (09:44:36 Paris). Runs #3 à #17.
Le processus local est détaché du terminal ; `caffeinate` maintient l’activité de
la machine tant que la collecte fonctionne. Fermer le capot ou arrêter Docker
peut toujours interrompre l’essai. Aucun service permanent n’est installé.

Un runner local ignoré par Git (`target/ticket18-runner.py`) lance la collecte,
le recalcul/statique horaire et un dernier calcul à l’arrêt. État :
`target/ticket18-runner-state.json`. Journaux : `target/ticket18-campaign.log`,
`target/ticket18-aggregate.log`, `target/ticket18-runner.log`.
Le suivi Codex vérifie la campagne toutes les 30 minutes pendant que l’application
reste disponible. Il n’exécute pas de second collecteur et doit être désactivé
après le bilan final.

### Bilan final à compléter

Les quinze plateformes sont parcourues par tranches de 15 minutes, toutes files,
Iron à Challenger, même fenêtre et deux patches figés. Cible 10 000 parties et
budget 100 000 appels par plateforme, durée maximale 24 h ; ce sont des plafonds,
pas une promesse de 150 000 parties. Les quotas Riot restent prioritaires.

Le bilan final doit consigner couverture par plateforme/file/rang, réutilisation,
reprises, erreurs finales, statiques, taux publiables, temps/mémoire/stockage,
et les modes réellement observés. La clé peut expirer avant l’échéance. Ne pas
clôturer #18 avant ce bilan. Laisser la machine et Docker disponibles pendant le run.

Limites : échantillon issu de seeds classés, pas tous les joueurs Riot ; Swarm validé
sur fixture seulement ; pas de recette Windows native ; ni conservation publique
ni API produit traitées ici. La borne de campagne concerne les appels Riot : une
finalisation SQL bloquée peut retarder le retour du processus sans nouvel appel.
