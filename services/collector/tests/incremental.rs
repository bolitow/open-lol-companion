//! Recalcul par lots (#89) contre PostgreSQL réel : même instantané que le recalcul
//! complet, seuls les lots modifiés relus, publication atomique conservée.
mod common;

use std::collections::BTreeMap;
use std::time::Duration;

use common::TestDb;
use olc_collector::aggregation::{
    recalculate_filtered, recalculate_incremental, AggregationError, AggregationOptions,
    AggregationReport, IncrementalReport, QualityThresholds, DEFAULT_RANK_MAX_AGE_HOURS,
};
use olc_collector::config::RunParams;
use olc_collector::model::fixtures::match_detail;
use serde_json::{json, Value};
use sqlx::Row;

macro_rules! db_or_skip {
    () => {
        match TestDb::create().await {
            Some(db) => db,
            None => return,
        }
    };
}

const SECTIONS: [&str; 8] = [
    "coverage",
    "groups",
    "bans",
    "builds",
    "skill_levels",
    "item_events",
    "splits",
    "performance",
];

async fn run(db: &TestDb) -> i64 {
    db.storage
        .create_run(&RunParams::default(), 1_000_000, 2_000_000)
        .await
        .unwrap()
}

/// Partie classée ou non, `start_s` secondes après l'époque Unix.
async fn insert(db: &TestDb, run_id: i64, id: &str, queue: i32, patch: &str, start_s: i64) {
    let platform = id.split_once('_').unwrap().0;
    let version = format!("{patch}.1.1");
    let mut detail = match_detail(id, platform, queue, start_s * 1000);
    detail["info"]["gameVersion"] = json!(version);
    if id.ends_with("_ban") {
        detail["info"]["teams"] = json!([
            {"teamId":100,"bans":(1..=5).map(|t|json!({"championId":40+t,"pickTurn":t})).collect::<Vec<_>>()},
            {"teamId":200,"bans":(6..=10).map(|t|json!({"championId":40+t,"pickTurn":t})).collect::<Vec<_>>()}
        ]);
    }
    sqlx::query(
        "INSERT INTO matches (match_id, platform_id, queue_id, game_version, patch,
        game_start, game_duration_s, is_remake, detail, first_run_id)
        VALUES ($1, $2, $3, $4, $5, to_timestamp($6), 1800, $7, $8, $9)",
    )
    .bind(id)
    .bind(platform)
    .bind(queue)
    .bind(&version)
    .bind(patch)
    .bind(start_s as f64)
    .bind(id.ends_with("_remake"))
    .bind(detail)
    .bind(run_id)
    .execute(db.storage.pool())
    .await
    .unwrap();
}

async fn observe(db: &TestDb, platform: &str, puuid: &str, tier: &str, at_s: i64) {
    sqlx::query(
        "INSERT INTO participant_rank_observations
        (platform_id,puuid,queue_id,tier,division,league_points,status,observed_at)
        VALUES ($1,$2,420,$3,'II',10,'ranked',to_timestamp($4))",
    )
    .bind(platform)
    .bind(puuid)
    .bind(tier)
    .bind(at_s as f64)
    .execute(db.storage.pool())
    .await
    .unwrap();
}

const START: i64 = 1_700_000_000;

/// Quatre lots : EUW1 Solo 15.19 (rangs, bans, timeline, remake), EUW1 ARAM 15.19,
/// NA1 Solo 15.19 et EUW1 Solo 15.18, un mois plus tôt.
async fn seed(db: &TestDb) -> i64 {
    let run_id = run(db).await;
    for id in ["EUW1_1_ban", "EUW1_2", "EUW1_3_remake"] {
        insert(db, run_id, id, 420, "15.19", START).await;
    }
    for (i, tier) in [
        "GOLD", "GOLD", "SILVER", "GOLD", "PLATINUM", "GOLD", "EMERALD",
    ]
    .iter()
    .enumerate()
    {
        observe(db, "EUW1", &format!("fake-puuid-{i}"), tier, START + 3600).await;
    }
    let timeline = json!({"metadata":{"matchId":"EUW1_2"},"info":{"participants":[{"participantId":1}],
        "frames":[{"timestamp":0,"events":[
            {"type":"SKILL_LEVEL_UP","timestamp":1000,"participantId":1,"skillSlot":2,"levelUpType":"NORMAL"},
            {"type":"ITEM_PURCHASED","timestamp":2000,"participantId":1,"itemId":1055}]}]}});
    sqlx::query(
        "INSERT INTO match_timelines(match_id,status,timeline) VALUES ('EUW1_2','available',$1)",
    )
    .bind(timeline)
    .execute(db.storage.pool())
    .await
    .unwrap();
    insert(db, run_id, "EUW1_aram", 450, "15.19", START).await;
    sqlx::query("INSERT INTO match_timelines(match_id,status) VALUES ('EUW1_aram','unavailable')")
        .execute(db.storage.pool())
        .await
        .unwrap();
    insert(db, run_id, "NA1_1", 420, "15.19", START).await;
    insert(db, run_id, "EUW1_old", 420, "15.18", START - 30 * 86_400).await;
    run_id
}

async fn incremental(db: &TestDb, min_games: u32) -> Result<IncrementalReport, AggregationError> {
    recalculate_incremental(
        &db.storage,
        min_games,
        DEFAULT_RANK_MAX_AGE_HOURS,
        &AggregationOptions::default(),
        &QualityThresholds::default(),
    )
    .await
}

async fn full(db: &TestDb, min_games: u32) -> AggregationReport {
    recalculate_filtered(
        &db.storage,
        min_games,
        DEFAULT_RANK_MAX_AGE_HOURS,
        &AggregationOptions::default(),
    )
    .await
    .unwrap()
}

/// Instantané publié, sections concaténées dans l'ordre des morceaux.
async fn raw_published(db: &TestDb) -> Value {
    let rows = sqlx::query(
        "SELECT s.report,c.section,c.items FROM champion_stats_snapshot s
        LEFT JOIN champion_stats_snapshot_chunks c ON c.snapshot_id=s.id
        WHERE s.id=1 ORDER BY c.section,c.chunk_index",
    )
    .fetch_all(db.storage.pool())
    .await
    .unwrap();
    let mut report: Value = rows[0].get("report");
    for section in SECTIONS {
        report[section] = json!([]);
    }
    for row in rows {
        if let Some(section) = row.get::<Option<String>, _>("section") {
            let items: Value = row.get("items");
            report[&section]
                .as_array_mut()
                .unwrap()
                .extend(items.as_array().unwrap().iter().cloned());
        }
    }
    report
}

/// Forme canonique : l'ordre des lots entre eux suit l'ordre d'écriture et n'est pas lu
/// par l'API (elle filtre toujours un périmètre) ; un tri stable par périmètre conserve
/// l'ordre exact des entrées à l'intérieur de chaque lot.
async fn published(db: &TestDb) -> Value {
    let mut report = raw_published(db).await;
    for section in SECTIONS {
        report[section].as_array_mut().unwrap().sort_by_key(|e| {
            (
                e["patch"].to_string(),
                e["platform_id"].to_string(),
                e["queue_id"].to_string(),
            )
        });
    }
    report
}

async fn lots(db: &TestDb) -> BTreeMap<String, (String, i32)> {
    sqlx::query("SELECT patch||'/'||platform_id||'/'||queue_id AS lot,fingerprint,chunks FROM champion_stats_snapshot_lots")
        .fetch_all(db.storage.pool())
        .await
        .unwrap()
        .into_iter()
        .map(|r| (r.get("lot"), (r.get("fingerprint"), r.get("chunks"))))
        .collect()
}

fn without_sections(mut report: AggregationReport) -> AggregationReport {
    report.coverage.clear();
    report.groups.clear();
    report.bans.clear();
    report.builds.clear();
    report.skill_levels.clear();
    report.item_events.clear();
    report.splits.clear();
    report.performance.clear();
    report
}

#[tokio::test]
async fn le_recalcul_par_lots_publie_exactement_le_recalcul_complet() {
    let db = db_or_skip!();
    seed(&db).await;
    let reference = full(&db, 1).await;
    let expected = published(&db).await;
    assert!(expected["bans"]
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["rank"] == "GOLD"));
    assert!(!expected["skill_levels"].as_array().unwrap().is_empty());
    assert_eq!(reference.exclusions.get("remake"), Some(&1));

    let first = incremental(&db, 1).await.unwrap();
    assert_eq!(
        (first.lots, first.recomputed_lots, first.reused_lots),
        (4, 4, 0)
    );
    assert_eq!(first.report, without_sections(reference));
    assert_eq!(published(&db).await, expected);
    assert_eq!(
        db.scalar("SELECT count(*) FROM champion_stats_snapshot_chunks WHERE lot_patch IS NULL")
            .await,
        0
    );
    let chunks: i64 = db
        .scalar("SELECT count(*) FROM champion_stats_snapshot_chunks")
        .await;
    assert_eq!(
        lots(&db)
            .await
            .values()
            .map(|(_, n)| i64::from(*n))
            .sum::<i64>(),
        chunks
    );

    // Rien n'a changé : aucun lot n'est relu, la publication reste identique.
    let second = incremental(&db, 1).await.unwrap();
    assert_eq!((second.recomputed_lots, second.reused_lots), (0, 4));
    assert_eq!(second.report, first.report);
    assert_eq!(published(&db).await, expected);
    db.cleanup().await;
}

#[tokio::test]
async fn seuls_les_lots_modifies_sont_relus_et_le_resultat_reste_exact() {
    let db = db_or_skip!();
    let run_id = seed(&db).await;
    incremental(&db, 1).await.unwrap();
    let before = lots(&db).await;

    // Nouvelle partie (EUW1 Solo 15.19), rang observé près de la partie NA1, timeline
    // ARAM devenue disponible, et une observation EUW1 lointaine de toutes les parties.
    insert(&db, run_id, "EUW1_4", 420, "15.19", START + 600).await;
    observe(&db, "NA1", "fake-puuid-3", "DIAMOND", START + 7200).await;
    observe(&db, "EUW1", "fake-puuid-9", "IRON", START + 400 * 86_400).await;
    sqlx::query(
        "UPDATE match_timelines SET status='available',timeline=$1 WHERE match_id='EUW1_aram'",
    )
    .bind(
        json!({"metadata":{"matchId":"EUW1_aram"},"info":{"participants":[{"participantId":1}],
        "frames":[{"timestamp":0,"events":[{"type":"SKILL_LEVEL_UP","timestamp":1000,
        "participantId":1,"skillSlot":1,"levelUpType":"NORMAL"}]}]}}),
    )
    .execute(db.storage.pool())
    .await
    .unwrap();

    let report = incremental(&db, 1).await.unwrap();
    assert_eq!(
        (report.lots, report.recomputed_lots, report.reused_lots),
        (4, 3, 1)
    );
    let after = lots(&db).await;
    assert_eq!(after["15.18/EUW1/420"], before["15.18/EUW1/420"]);
    for lot in ["15.19/EUW1/420", "15.19/EUW1/450", "15.19/NA1/420"] {
        assert_ne!(after[lot].0, before[lot].0, "{lot}");
    }
    let incremental_snapshot = published(&db).await;
    assert!(incremental_snapshot["groups"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["platform_id"] == "NA1" && g["rank"] == "DIAMOND"));

    let reference = full(&db, 1).await;
    assert_eq!(report.report, without_sections(reference));
    assert_eq!(incremental_snapshot, published(&db).await);
    // Le recalcul complet remplace aussi les lots : le suivant les relit tous.
    assert!(lots(&db).await.is_empty());
    let next = incremental(&db, 1).await.unwrap();
    assert_eq!((next.recomputed_lots, next.reused_lots), (4, 0));
    assert_eq!(published(&db).await, incremental_snapshot);
    db.cleanup().await;
}

#[tokio::test]
async fn un_lot_sorti_des_filtres_est_retire_sans_relire_les_autres() {
    let db = db_or_skip!();
    seed(&db).await;
    incremental(&db, 1).await.unwrap();
    let euw = AggregationOptions {
        platforms: vec!["EUW1".into()],
        ..AggregationOptions::default()
    };
    let report = recalculate_incremental(
        &db.storage,
        1,
        DEFAULT_RANK_MAX_AGE_HOURS,
        &euw,
        &QualityThresholds::default(),
    )
    .await
    .unwrap();
    // Un filtre ne change pas l'empreinte des lots qu'il garde.
    assert_eq!(
        (report.lots, report.recomputed_lots, report.reused_lots),
        (3, 0, 3)
    );
    assert!(!lots(&db).await.contains_key("15.19/NA1/420"));
    assert_eq!(
        db.scalar(
            "SELECT count(*) FROM champion_stats_snapshot_chunks WHERE lot_platform_id='NA1'"
        )
        .await,
        0
    );
    let snapshot = published(&db).await;
    let reference = recalculate_filtered(&db.storage, 1, DEFAULT_RANK_MAX_AGE_HOURS, &euw)
        .await
        .unwrap();
    assert_eq!(report.report, without_sections(reference));
    assert_eq!(snapshot, published(&db).await);
    db.cleanup().await;
}

#[tokio::test]
async fn un_parametre_modifie_recalcule_tous_les_lots() {
    let db = db_or_skip!();
    seed(&db).await;
    incremental(&db, 1).await.unwrap();
    let report = incremental(&db, 2).await.unwrap();
    assert_eq!((report.recomputed_lots, report.reused_lots), (4, 0));
    let snapshot = published(&db).await;
    assert_eq!(report.report, without_sections(full(&db, 2).await));
    assert_eq!(snapshot, published(&db).await);
    db.cleanup().await;
}

#[tokio::test]
async fn un_echec_pendant_un_lot_conserve_la_publication_et_les_lots() {
    let db = db_or_skip!();
    let run_id = seed(&db).await;
    incremental(&db, 1).await.unwrap();
    let before = (raw_published(&db).await, lots(&db).await);
    insert(&db, run_id, "EUW1_4", 420, "15.19", START).await;
    sqlx::raw_sql(
        "CREATE FUNCTION reject_chunk() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.section='groups' THEN RAISE EXCEPTION 'synthetic chunk failure'; END IF;
        RETURN NEW; END $$;
        CREATE TRIGGER reject_chunk BEFORE INSERT ON champion_stats_snapshot_chunks
        FOR EACH ROW EXECUTE FUNCTION reject_chunk();",
    )
    .execute(db.storage.pool())
    .await
    .unwrap();
    assert!(matches!(
        incremental(&db, 1).await,
        Err(AggregationError::Database(_))
    ));
    assert_eq!((raw_published(&db).await, lots(&db).await), before);

    // Filtres invalides et calcul concurrent : refus sans rien toucher.
    let invalid = AggregationOptions {
        queues: vec![0],
        ..AggregationOptions::default()
    };
    assert!(matches!(
        recalculate_incremental(&db.storage, 1, 1, &invalid, &QualityThresholds::default()).await,
        Err(AggregationError::InvalidFilters)
    ));
    let mut holder = db.storage.pool().begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(0x0018_A660_0001_i64)
        .execute(&mut *holder)
        .await
        .unwrap();
    assert!(matches!(
        incremental(&db, 1).await,
        Err(AggregationError::Busy)
    ));
    holder.rollback().await.unwrap();
    assert_eq!((raw_published(&db).await, lots(&db).await), before);
    db.cleanup().await;
}

async fn wait_for_publication_lock(db: &TestDb) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM pg_stat_activity
                WHERE datname = current_database() AND wait_event_type = 'Lock'
                AND query LIKE 'INSERT INTO champion_stats_snapshot %')",
            )
            .fetch_one(db.storage.pool())
            .await
            .unwrap();
            if waiting {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("le calcul atteint la publication bloquée");
}

#[tokio::test]
async fn le_recalcul_par_lots_n_ecrase_pas_une_publication_plus_recente() {
    let db = db_or_skip!();
    seed(&db).await;
    incremental(&db, 1).await.unwrap();
    let mut blocker = db.storage.pool().begin().await.unwrap();
    sqlx::query("SELECT id FROM champion_stats_snapshot WHERE id = 1 FOR UPDATE")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let other = db.separate_storage().await;
    let writer = other.clone();
    let task = tokio::spawn(async move {
        recalculate_incremental(
            &writer,
            2,
            DEFAULT_RANK_MAX_AGE_HOURS,
            &AggregationOptions::default(),
            &QualityThresholds::default(),
        )
        .await
    });
    wait_for_publication_lock(&db).await;
    // Simule une publication dont le commit arrive après la prise de l'instantané.
    sqlx::query(
        "UPDATE champion_stats_snapshot SET report = jsonb_set(report, '{min_games}', '123')",
    )
    .execute(&mut *blocker)
    .await
    .unwrap();
    blocker.commit().await.unwrap();
    assert!(matches!(
        task.await.unwrap(),
        Err(AggregationError::Database(_))
    ));
    assert_eq!(raw_published(&db).await["min_games"], 123);
    other.pool().close().await;
    db.cleanup().await;
}

fn cli(db: &TestDb, extra: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_olc-collector"))
        .args(["aggregate", "--all-stored", "--min-games", "1"])
        .args(extra)
        .env("DATABASE_URL", db.database_url())
        .env("RIOT_API_KEY", "")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn la_cli_recalcule_par_lots_sur_demande_et_publie_le_meme_instantane() {
    let db = db_or_skip!();
    seed(&db).await;
    let complete = cli(&db, &[]);
    assert!(
        complete.status.success(),
        "{}",
        String::from_utf8_lossy(&complete.stderr)
    );
    let expected = published(&db).await;
    let mut outputs = vec![];
    for _ in 0..2 {
        let output = cli(&db, &["--incremental", "--json"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        outputs.push(serde_json::from_slice::<Value>(&output.stdout).unwrap());
        assert_eq!(published(&db).await, expected);
    }
    assert_eq!(
        (
            outputs[0]["lots"].as_u64(),
            outputs[0]["recomputed_lots"].as_u64()
        ),
        (Some(4), Some(4))
    );
    assert_eq!(outputs[1]["reused_lots"], 4);
    assert_eq!(
        outputs[1]["report"]["included_matches"],
        expected["included_matches"]
    );
    let human = cli(&db, &["--incremental"]);
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(
        text.contains("4 lots (recalculés : 0, réutilisés : 4)"),
        "{text}"
    );
    db.cleanup().await;
}

/// Publie (ou republie, sous la même version) un catalogue d'objets normalisé (#61).
async fn publish_item_catalog(
    db: &TestDb,
    publication: &str,
    version: &str,
    items: &[(&str, Value)],
) {
    sqlx::query("INSERT INTO game_catalog_publications(id,version,normalizer_version,manifest) VALUES ($1,$2,1,'{}')")
        .bind(publication).bind(version).execute(db.storage.pool()).await.unwrap();
    for (id, fields) in items {
        let normalized: serde_json::Map<String, Value> = fields
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, value)| {
                let field = json!({"value":value,"unit":null,"status":"verified","sources":[]});
                (name.clone(), field)
            })
            .collect();
        sqlx::query("INSERT INTO game_catalog_entries(publication_id,kind,id,namespace,locale,name,data) VALUES ($1,'item',$2,'standard','en_US',$2,$3)")
            .bind(publication).bind(id).bind(json!({"fields": normalized}))
            .execute(db.storage.pool()).await.unwrap();
    }
    // Même chemin que `catalog::storage` : une republication remplace la publication
    // courante de la version sans changer la version.
    sqlx::query(
        "INSERT INTO game_catalog_current(version,publication_id) VALUES ($1,$2)
        ON CONFLICT (version) DO UPDATE SET publication_id=EXCLUDED.publication_id",
    )
    .bind(version)
    .bind(publication)
    .execute(db.storage.pool())
    .await
    .unwrap();
}

fn has_core_item(snapshot: &Value, item: u64) -> bool {
    snapshot["builds"].as_array().unwrap().iter().any(|b| {
        b["category"] == "core"
            && b["selection"]
                .as_array()
                .is_some_and(|i| i.contains(&json!(item)))
    })
}

#[tokio::test]
async fn une_republication_du_catalogue_recalcule_les_lots_de_son_patch() {
    let db = db_or_skip!();
    seed(&db).await;
    // Achats de la partie EUW1_2 : 1055 au départ, puis trois objets chers.
    let events: Vec<_> = [
        (1055, 2_000),
        (3031, 600_000),
        (6672, 900_000),
        (3072, 1_200_000),
    ]
    .iter()
    .map(|(id, at)| json!({"type":"ITEM_PURCHASED","participantId":1,"timestamp":at,"itemId":id}))
    .collect();
    let timeline = json!({"metadata":{"matchId":"EUW1_2"},"info":{"participants":[{"participantId":1}],
        "frames":[{"timestamp":0,"events":events}]}});
    sqlx::query("UPDATE match_timelines SET timeline=$1 WHERE match_id='EUW1_2'")
        .bind(timeline)
        .execute(db.storage.pool())
        .await
        .unwrap();
    let starter = json!({"price_total":450,"purchasable":true,"categories":["Lane"]});
    let item = |price: u32| json!({"price_total":price,"purchasable":true,"categories":["Damage"]});
    // 3031 n'est pas encore un objet complet (prix sous 2000) : pas de core à trois objets.
    publish_item_catalog(
        &db,
        "pub-15-19-a",
        "15.19.1",
        &[
            ("1055", starter.clone()),
            ("3031", item(1500)),
            ("6672", item(3000)),
            ("3072", item(3400)),
        ],
    )
    .await;
    let first = incremental(&db, 1).await.unwrap();
    assert_eq!((first.recomputed_lots, first.reused_lots), (4, 0));
    assert!(!has_core_item(&published(&db).await, 3031));
    let before = lots(&db).await;

    // Même version republiée (normaliseur, `--refresh`, complément CommunityDragon) :
    // 3031 devient complet, les étapes de builds de 15.19 changent.
    publish_item_catalog(
        &db,
        "pub-15-19-b",
        "15.19.1",
        &[
            ("1055", starter.clone()),
            ("3031", item(3500)),
            ("6672", item(3000)),
            ("3072", item(3400)),
        ],
    )
    .await;
    let report = incremental(&db, 1).await.unwrap();
    assert_eq!(
        (report.lots, report.recomputed_lots, report.reused_lots),
        (4, 3, 1)
    );
    let after = lots(&db).await;
    assert_eq!(after["15.18/EUW1/420"], before["15.18/EUW1/420"]);
    for lot in ["15.19/EUW1/420", "15.19/EUW1/450", "15.19/NA1/420"] {
        assert_ne!(after[lot].0, before[lot].0, "{lot}");
    }
    let snapshot = published(&db).await;
    assert!(has_core_item(&snapshot, 3031));
    let reference = full(&db, 1).await;
    assert_eq!(report.report, without_sections(reference));
    assert_eq!(snapshot, published(&db).await);

    // Nouvelle version du même patch, contenu identique : les lots 15.19 sont relus.
    incremental(&db, 1).await.unwrap();
    publish_item_catalog(
        &db,
        "pub-15-19-c",
        "15.19.2",
        &[
            ("1055", starter),
            ("3031", item(3500)),
            ("6672", item(3000)),
            ("3072", item(3400)),
        ],
    )
    .await;
    let report = incremental(&db, 1).await.unwrap();
    assert_eq!((report.recomputed_lots, report.reused_lots), (3, 1));
    assert_eq!(
        report.report.item_catalogs,
        without_sections(full(&db, 1).await).item_catalogs
    );
    assert_eq!(
        published(&db).await["item_catalogs"][0]["version"],
        "15.19.2"
    );
    db.cleanup().await;
}
