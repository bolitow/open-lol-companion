//! Publications de référentiels sur une base jetable, sans joueur ni clé Riot.
mod common;
use common::TestDb;
use olc_collector::catalog::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn projection(price: u32) -> CatalogProjection {
    let source = make_source(
        "ddragon",
        "fr_FR/item.json",
        "16.19.1",
        Some("fr_FR"),
        "https://ddragon.leagueoflegends.com/cdn/16.19.1/data/fr_FR/item.json",
        "2026-10-01T00:00:00Z",
        json!({"type":"item","version":"16.19.1","data":{"1001":{"name":"Bottes","gold":{"total":price}}}}),
    );
    let value = CatalogValue {
        value: json!(price),
        unit: Some("gold".into()),
        status: ValueStatus::Verified,
        sources: vec![ValueSource {
            source_id: source.id.clone(),
            pointer: "/data/1001/gold/total".into(),
        }],
    };
    CatalogProjection {
        version: "16.19.1".into(),
        sources: vec![source],
        records: vec![CatalogRecord {
            kind: "item".into(),
            id: "1001".into(),
            namespace: "standard".into(),
            locale: "fr_FR".into(),
            name: "Bottes".into(),
            description: None,
            icon: None,
            fields: BTreeMap::from([("price_total".into(), value)]),
            stats: BTreeMap::new(),
            effects: vec![],
            coverage: RecordCoverage {
                source_fields: 2,
                normalized_fields: 2,
                ..Default::default()
            },
        }],
        degraded: false,
        warnings: vec![],
    }
}

#[tokio::test]
async fn publication_idempotente_et_sources_historiques_preservees_apres_correction() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let first = publish(&db.storage, &projection(300)).await.unwrap();
    let repeat = publish(&db.storage, &projection(300)).await.unwrap();
    assert_eq!(first, repeat);
    let corrected = publish(&db.storage, &projection(310)).await.unwrap();
    assert_ne!(first.publication_id, corrected.publication_id);
    let old = archived_sources(&db.storage, &first.publication_id)
        .await
        .unwrap();
    assert_eq!(old[0].data["data"]["1001"]["gold"]["total"], 300);
    let current: String = sqlx::query_scalar(
        "SELECT publication_id FROM game_catalog_current WHERE version='16.19.1'",
    )
    .fetch_one(db.storage.pool())
    .await
    .unwrap();
    assert_eq!(current, corrected.publication_id);
    assert_eq!(
        db.scalar("SELECT count(*) FROM game_catalog_publications")
            .await,
        2
    );
    db.cleanup().await;
}

#[tokio::test]
async fn un_echec_au_commit_ne_publie_ni_sources_ni_entites_partielles() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let first = publish(&db.storage, &projection(300)).await.unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_catalog() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'private detail'; END $$; CREATE CONSTRAINT TRIGGER reject_catalog AFTER UPDATE ON game_catalog_current DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_catalog();").execute(db.storage.pool()).await.unwrap();
    assert!(matches!(
        publish(&db.storage, &projection(310)).await,
        Err(CatalogError::Database)
    ));
    let current: String = sqlx::query_scalar(
        "SELECT publication_id FROM game_catalog_current WHERE version='16.19.1'",
    )
    .fetch_one(db.storage.pool())
    .await
    .unwrap();
    assert_eq!(current, first.publication_id);
    assert_eq!(
        db.scalar("SELECT count(*) FROM game_catalog_sources").await,
        1
    );
    assert_eq!(
        db.scalar("SELECT count(*) FROM game_catalog_entries").await,
        1
    );
    db.cleanup().await;
}

#[tokio::test]
async fn une_source_falsifiee_et_des_identites_dupliquees_sont_refusees() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let mut invalid = projection(300);
    invalid.sources[0].data = Value::Null;
    assert!(matches!(
        publish(&db.storage, &invalid).await,
        Err(CatalogError::InvalidSource)
    ));
    let mut duplicate = projection(300);
    duplicate.records.push(duplicate.records[0].clone());
    assert!(matches!(
        publish(&db.storage, &duplicate).await,
        Err(CatalogError::InvalidSource)
    ));
    assert_eq!(
        db.scalar("SELECT count(*) FROM game_catalog_publications")
            .await,
        0
    );
    db.cleanup().await;
}

#[tokio::test]
async fn un_pointeur_de_provenance_absent_ne_peut_pas_etre_publie() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let mut invalid = projection(300);
    invalid.records[0]
        .fields
        .get_mut("price_total")
        .unwrap()
        .sources[0]
        .pointer = "/data/1001/nonexistent".into();
    assert!(matches!(
        publish(&db.storage, &invalid).await,
        Err(CatalogError::InvalidSource)
    ));
    assert_eq!(
        db.scalar("SELECT count(*) FROM game_catalog_publications")
            .await,
        0
    );
    db.cleanup().await;
}

#[tokio::test]
async fn archive_preserve_le_zero_negatif_et_les_decimales_des_exports() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let mut projection = projection(300);
    let mut source = projection.sources.remove(0);
    let previous = source.id.clone();
    source.data["numeric_edge_cases"] = json!([
        -0.0,
        0.30000001192092896,
        1.600000023841858,
        0.1599999964237213,
        0.11999999731779099,
        0.9300068616867065
    ]);
    let source = make_source(
        &source.provider,
        &source.key,
        &source.version,
        source.locale.as_deref(),
        &source.url,
        &source.observed_at,
        source.data,
    );
    for field in projection.records[0].fields.values_mut() {
        for origin in &mut field.sources {
            if origin.source_id == previous {
                origin.source_id = source.id.clone();
            }
        }
    }
    projection.sources.push(source.clone());
    let published = publish(&db.storage, &projection).await.unwrap();
    let archived = archived_sources(&db.storage, &published.publication_id)
        .await
        .unwrap();
    assert_eq!(archived[0].data.to_string(), source.data.to_string());
    assert_eq!(
        publish(
            &db.storage,
            &CatalogProjection {
                sources: archived,
                ..projection
            }
        )
        .await
        .unwrap(),
        published
    );
    db.cleanup().await;
}

#[tokio::test]
async fn reconstruire_une_ancienne_revision_avec_un_nouveau_mapping_ne_rembobine_pas_la_tete() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    // Cette projection réduite simule une ancienne version du normaliseur.
    let old = publish(&db.storage, &projection(300)).await.unwrap();
    let current = publish(&db.storage, &projection(310)).await.unwrap();
    let rebuilt = rebuild(&db.storage, &old.publication_id).await.unwrap();
    assert_ne!(rebuilt.publication_id, old.publication_id);
    let head: String = sqlx::query_scalar(
        "SELECT publication_id FROM game_catalog_current WHERE version='16.19.1'",
    )
    .fetch_one(db.storage.pool())
    .await
    .unwrap();
    assert_eq!(head, current.publication_id);
    db.cleanup().await;
}

#[tokio::test]
async fn retraitement_du_cache_et_reconstruction_archives_sans_riot() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    olc_collector::static_data::sync_with_transport(
        &db.storage,
        2,
        olc_collector::static_data::test_support::FakeCdn::new(),
        false,
    )
    .await
    .unwrap();
    let sources = cached_sources(&db.storage, "16.19.1").await.unwrap();
    assert!(sources.iter().any(|s| s.key == "fr_FR/item.json"));
    assert!(sources
        .iter()
        .filter(|s| s.provider == "riot_catalog")
        .all(|s| s.version == "unversioned" && s.locale.is_none()));
    let manifest = build(&db.storage, "16.19.1", CommunityPolicy::Off, false)
        .await
        .unwrap();
    assert!(manifest.coverage.by_kind["item"] >= 2);
    assert!(manifest.degraded);
    // Le replay doit continuer après suppression du cache renouvelable.
    sqlx::query("DELETE FROM static_data_releases")
        .execute(db.storage.pool())
        .await
        .unwrap();
    assert_eq!(
        rebuild(&db.storage, &manifest.publication_id)
            .await
            .unwrap(),
        manifest
    );
    assert!(cached_sources(&db.storage, "../16.19.1").await.is_err());
    db.cleanup().await;
}

#[test]
fn seul_un_complement_facultatif_en_panne_reseau_peut_degrader_la_publication() {
    let sources = projection(300).sources;
    let optional = combine_sources(
        "16.19.1",
        sources.clone(),
        CommunityPolicy::Optional,
        Err(CatalogError::Network),
    )
    .unwrap();
    assert!(optional.degraded);
    assert_eq!(optional.warnings, ["community_unavailable"]);
    assert!(!optional.records.is_empty());
    assert!(matches!(
        combine_sources(
            "16.19.1",
            sources.clone(),
            CommunityPolicy::Required,
            Err(CatalogError::Network)
        ),
        Err(CatalogError::Network)
    ));
    assert!(matches!(
        combine_sources(
            "16.19.1",
            sources,
            CommunityPolicy::Optional,
            Err(CatalogError::InvalidSource)
        ),
        Err(CatalogError::InvalidSource)
    ));
}

#[test]
fn la_commande_de_reconstruction_refuse_une_version_simultanee() {
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_olc-collector"))
        .args(["catalog", "--rebuild", "abc", "--version", "16.19.1"])
        .output()
        .unwrap();
    let help = std::process::Command::new(env!("CARGO_BIN_EXE_olc-collector"))
        .args(["catalog", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success(), "commande catalogue absente");
    assert!(!result.status.success());
}

#[tokio::test]
#[ignore = "recette explicite sur une base de catalogues publics isolée"]
async fn reconstruction_reelle_des_sources_archivees() {
    let url = std::env::var("OLC_CATALOG_RECIPE_DATABASE_URL").expect("base de recette isolée");
    let storage = olc_collector::storage::Storage::connect(&url, 4)
        .await
        .unwrap();
    let heads: Vec<String> =
        sqlx::query_scalar("SELECT publication_id FROM game_catalog_current ORDER BY version")
            .fetch_all(storage.pool())
            .await
            .unwrap();
    assert!(!heads.is_empty());
    for head in heads {
        let sources = archived_sources(&storage, &head).await.unwrap();
        for source in &sources {
            let computed = make_source(
                &source.provider,
                &source.key,
                &source.version,
                source.locale.as_deref(),
                &source.url,
                &source.observed_at,
                source.data.clone(),
            );
            assert_eq!(
                computed.id, source.id,
                "empreinte archive {} {}",
                source.provider, source.key
            );
        }
        let original: Value =
            sqlx::query_scalar("SELECT manifest FROM game_catalog_publications WHERE id=$1")
                .bind(&head)
                .fetch_one(storage.pool())
                .await
                .unwrap();
        let rebuilt = rebuild(&storage, &head)
            .await
            .expect("replay de la publication réelle");
        assert_eq!(serde_json::to_value(rebuilt).unwrap(), original);
    }
}
