# Filtrage indexé des morceaux de statistiques — #19

## Problème et correction

La lecture d'un build filtrait les entrées JSON de chaque morceau de la publication,
même quand son champion ou sa population ne correspondaient pas à la demande.
La migration `0009` stocke les combinaisons distinctes de dimensions sous leur section
et les indexe avec GIN. La requête sélectionne ces morceaux avant le filtre JSON fin.
Les tuples restent entiers : la présence de TOP/GOLD et JUNGLE/ALL dans un morceau
ne doit pas sélectionner ce morceau pour JUNGLE/GOLD.

Les rapports publics, la pagination et la lecture atomique tête/morceaux restent
inchangés. Les anciens rapports monolithiques v1 restent lisibles ; ils bénéficieront
de l'index après un nouveau calcul du collecteur qui les publiera en v2.

## Non-régression

Commande depuis la racine, avec `OLC_TEST_DATABASE_URL` pointant vers un PostgreSQL 17
jetable autorisant la création de bases de test :

```sh
cargo test -p olc-api --test stats
pnpm test
pnpm lint
```

- TDD : le nouveau test échoue avant correction (2 049 morceaux retournés au lieu de 1),
  puis réussit avec le filtre indexé.
- Sept tests de statistiques : réponses v1/v2 identiques, reprise des morceaux déjà
  présents par la migration, maintien des clés après modification du JSON, pagination,
  périmètres absents, morceaux mixtes et exclusion des builds pour une tierlist.
- `EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)` porte sur le fichier SQL réellement chargé
  par l'API : présence de l'index et nombre de morceaux sélectionnés vérifiés, sans
  seuil de durée dépendant de la machine.
- Validation globale locale : 319 tests Rust et 10 tests TypeScript réussis ; deux
  recettes existantes sur des exports réels restent explicitement ignorées. Tests
  PostgreSQL activés. `pnpm lint` réussi. Hôte macOS, PostgreSQL Linux ARM64 en Docker ;
  Windows non exécuté, aucun nouveau code spécifique à un OS.

## Mesure synthétique du 2 octobre 2026

PostgreSQL 17.11, base isolée sans données Riot ni accès externe. 4 096 morceaux de
512 entrées de builds, soit **2 097 152 entrées**. Chaque morceau a une population
distincte par un identifiant de champion synthétique ; une requête pour le champion 1
ne doit en sélectionner qu'un. Ces identifiants et les variantes servent uniquement
au test de charge SQL et ne représentent pas un catalogue de champions réel.

La tête utilise `storage_version=2` et `report={}` : la mesure vise l'exécution SQL,
sans désérialisation Rust ni HTTP. Les tests ci-dessus valident les réponses métier.
Après `VACUUM ANALYZE`, quatre paires ancienne/nouvelle requête sont exécutées par
mode ; la première paire est écartée, les trois suivantes donnent la médiane.

| Plan PostgreSQL forcé | Avant | Après | Morceaux sélectionnés avant → après |
| --- | ---: | ---: | ---: |
| `force_custom_plan` | 1 277,787 ms | 1,053 ms | 4 096 → 1 |
| `force_generic_plan` | 1 408,674 ms | 1,329 ms | 4 096 → 1 |

L'ancien plan fait un `Seq Scan`, le nouveau un `Bitmap Heap Scan` via
`snapshot_chunk_populations_idx`. Au dernier passage, 13 772 blocs partagés consultés
pour la requête complète avant, contre 76 après, sans lecture disque dans ces plans.
La table et ses index occupent 23 Mio, dont 272 Kio pour le nouvel index.
La migration des 4 096 morceaux préexistants prend 5,619 s, temps client compris.

Ce scénario mesure une sélection très étroite, avec cache chaud et sans concurrence.
Il ne prédit pas la latence d'une publication réelle à populations mélangées ni le
temps d'un déploiement sur une autre machine.

### Reproduire le volume et le plan

Dans une base jetable vide, appliquer `0002_champion_stats.sql` puis
`0007_snapshot_chunks.sql`, et insérer ce jeu synthétique :

```sql
INSERT INTO champion_stats_snapshot VALUES (1, now(), now(), '{}'::jsonb, 2);
INSERT INTO champion_stats_snapshot_chunks (snapshot_id, section, chunk_index, items)
SELECT 1, 'builds', n, (
    SELECT jsonb_agg(jsonb_build_object(
        'patch','16.19','platform_id','EUW1','queue_id',420,
        'role','TOP','rank','ALL','champion_id',n+1,
        'category','item','selection',jsonb_build_array(i),
        'games',100,'population',200,'wins',50,'pick_rate',0.5,'win_rate',0.5
    )) FROM generate_series(1,512) i
) FROM generate_series(0,4095) n;
```

Appliquer ensuite `0009_snapshot_chunk_populations.sql`, puis `VACUUM ANALYZE` sur
les deux tables. Préparer la [requête API](../../services/api/src/sql/stats_snapshot.sql)
avec les types `(jsonb, jsonpath, jsonpath, boolean)`. Paramètres :

1. `{"patch":"16.19","platform":"EUW1","queue":420,"role":"TOP","rank":"ALL","champion":1}`.
2. `$[*] ? (@.patch == $patch && @.platform_id == $platform && @.queue_id == $queue && @.role == $role && @.rank == $rank && ($champion == null || @.champion_id == $champion))`.
3. `$[*] ? (@.patch == $patch && @.platform_id == $platform && @.queue_id == $queue)`.
4. `true`.

Exécuter `EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) EXECUTE ...` dans chaque mode
`SET plan_cache_mode = force_custom_plan` puis `force_generic_plan`. Pour reproduire
l'ancienne lecture, conserver les projections et remplacer uniquement le prédicat
`c.populations @> ANY (...)` par `(c.section IN ('coverage','groups','bans') OR $4)`.
Le CTE `filters` devient alors inutilisé, sans changer les résultats du plan.

## Migration et références

La colonne générée et l'index ajoutent un coût aux publications. La migration réécrit
et verrouille la table des morceaux existants ; prévoir une fenêtre adaptée avant
remise en trafic. Aucun recalcul des statistiques n'est requis pour un stockage v2.
La migration s'applique au démarrage des nouveaux services ; les bases de collecte
existantes n'ont pas été modifiées pendant cette recette.

Le migrateur SQLx embarque les fichiers SQL à la compilation. Le `build.rs` du
collecteur suit leur répertoire pour inclure une nouvelle migration même en compilation
incrémentale.

Références PostgreSQL 17 : [indexation JSONB et `jsonb_path_ops`](https://www.postgresql.org/docs/17/datatype-json.html#JSON-INDEXING),
[colonnes générées stockées](https://www.postgresql.org/docs/17/ddl-generated-columns.html).
