//! Revalidation HTTP du cache statique sur PostgreSQL jetable, sans appel à Riot.
mod common;

use axum::body::to_bytes;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use common::TestDb;
use olc_api::error::ApiError;
use olc_api::static_data::{document, manifest};
use serde_json::{json, Value};
use sqlx::PgPool;

const VERSION: &str = "16.19.1";

macro_rules! db_or_skip {
    () => {
        match TestDb::create().await {
            Some(db) => db,
            None => return,
        }
    };
}

fn item(name: &str) -> Value {
    json!({
        "type":"item", "version":VERSION,
        "data":{"1001":{"name":name,"description":"Description", "image":{"full":"1001.png"}}}
    })
}

fn bundle(french_name: &str) -> Value {
    json!({
        "schema_version":1, "version":VERSION, "patch":"16.19",
        "asset_base":format!("https://ddragon.leagueoflegends.com/cdn/{VERSION}/img/"),
        "classic_asset_base":format!("https://ddragon.leagueoflegends.com/cdn/{VERSION}/img/mode/classic/"),
        "rune_asset_base":"https://ddragon.leagueoflegends.com/cdn/img/",
        "champion_count":0, "classic_champion_count":0,
        "documents":{
            "fr_FR/item.json":{
                "url":format!("https://ddragon.leagueoflegends.com/cdn/{VERSION}/data/fr_FR/item.json"),
                "data":item(french_name)
            },
            "en_US/item.json":{
                "url":format!("https://ddragon.leagueoflegends.com/cdn/{VERSION}/data/en_US/item.json"),
                "data":item("Boots")
            }
        }
    })
}

async fn publish(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO static_data_releases (version,patch,completed_at,bundle)
        VALUES ($1,'16.19',to_timestamp(2000),$2)",
    )
    .bind(VERSION)
    .bind(bundle("Bottes"))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO static_data_manifest (id,live_version,checked_at,versions,catalogs)
        VALUES (1,$1,to_timestamp(2000),$2,$3)",
    )
    .bind(VERSION)
    .bind(json!([VERSION]))
    .bind(
        json!({"queues":{"url":"https://static.developer.riotgames.com/docs/lol/queues.json",
            "data":[{"queueId":420,"map":"Summoner's Rift","description":"Ranked Solo games"}]}}),
    )
    .execute(pool)
    .await
    .unwrap();
}

fn conditional(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(IF_NONE_MATCH, HeaderValue::from_str(value).unwrap());
    headers
}

fn public_tag(response: &Response) -> String {
    let cache = response
        .headers()
        .get(CACHE_CONTROL)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cache
        .split(',')
        .any(|directive| directive.trim() == "public"));
    assert!(!cache
        .split(',')
        .any(|directive| directive.trim() == "immutable"));
    let tag = response.headers().get(ETAG).unwrap().to_str().unwrap();
    assert!(!tag.is_empty());
    tag.to_owned()
}

fn json_tag(response: &Response) -> String {
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("application/json"));
    public_tag(response)
}

async fn json_body(response: Response) -> Value {
    let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn assert_not_modified(response: Response, tag: &str) {
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(public_tag(&response), tag);
    assert!(to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn manifeste_absent_est_indisponible_et_document_absent_est_introuvable() {
    let db = db_or_skip!();
    let headers = HeaderMap::new();
    assert_eq!(
        manifest(db.storage.pool(), &headers).await.err(),
        Some(ApiError::Unavailable)
    );
    assert_eq!(
        document(db.storage.pool(), VERSION, "fr_FR", "item.json", &headers)
            .await
            .err(),
        Some(ApiError::NotFound)
    );
    publish(db.storage.pool()).await;
    assert_eq!(
        document(
            db.storage.pool(),
            VERSION,
            "fr_FR",
            "champion/Aatrox.json",
            &headers
        )
        .await
        .err(),
        Some(ApiError::NotFound)
    );
    db.cleanup().await;
}

#[tokio::test]
async fn documents_fr_et_en_ont_des_corps_et_des_etags_distincts() {
    let db = db_or_skip!();
    publish(db.storage.pool()).await;
    let french = document(
        db.storage.pool(),
        VERSION,
        "fr_FR",
        "item.json",
        &HeaderMap::new(),
    )
    .await
    .unwrap();
    let french_tag = json_tag(&french);
    assert_eq!(json_body(french).await, item("Bottes"));
    let english = document(
        db.storage.pool(),
        VERSION,
        "en_US",
        "item.json",
        &conditional(&french_tag),
    )
    .await
    .unwrap();
    let english_tag = json_tag(&english);
    assert_ne!(english_tag, french_tag);
    assert_eq!(json_body(english).await, item("Boots"));
    let response = manifest(db.storage.pool(), &HeaderMap::new())
        .await
        .unwrap();
    json_tag(&response);
    let body = json_body(response).await;
    assert_eq!(body["live_version"], VERSION);
    assert_eq!(body["versions"], json!([VERSION]));
    assert_eq!(body["catalogs"]["queues"]["data"][0]["queueId"], 420);
    assert!(!body["checked_at"].as_str().unwrap().is_empty());
    db.cleanup().await;
}

#[tokio::test]
async fn etag_document_accepte_la_liste_le_tag_faible_et_etoile() {
    let db = db_or_skip!();
    publish(db.storage.pool()).await;
    let response = document(
        db.storage.pool(),
        VERSION,
        "fr_FR",
        "item.json",
        &HeaderMap::new(),
    )
    .await
    .unwrap();
    let tag = json_tag(&response);
    let strong = tag.strip_prefix("W/").unwrap_or(&tag);
    for condition in [
        tag.clone(),
        format!("\"autre\", {tag}"),
        format!("W/{strong}"),
        "*".into(),
    ] {
        let response = document(
            db.storage.pool(),
            VERSION,
            "fr_FR",
            "item.json",
            &conditional(&condition),
        )
        .await
        .unwrap();
        assert_not_modified(response, &tag).await;
    }
    let different = document(
        db.storage.pool(),
        VERSION,
        "fr_FR",
        "item.json",
        &conditional("\"autre\""),
    )
    .await
    .unwrap();
    assert_eq!(json_tag(&different), tag);
    assert_eq!(json_body(different).await, item("Bottes"));
    db.cleanup().await;
}

#[tokio::test]
async fn etag_manifeste_accepte_la_liste_le_tag_faible_et_etoile() {
    let db = db_or_skip!();
    publish(db.storage.pool()).await;
    let response = manifest(db.storage.pool(), &HeaderMap::new())
        .await
        .unwrap();
    let tag = json_tag(&response);
    let strong = tag.strip_prefix("W/").unwrap_or(&tag);
    for condition in [
        tag.clone(),
        format!("\"autre\", {tag}"),
        format!("W/{strong}"),
        "*".into(),
    ] {
        let response = manifest(db.storage.pool(), &conditional(&condition))
            .await
            .unwrap();
        assert_not_modified(response, &tag).await;
    }
    let different = manifest(db.storage.pool(), &conditional("\"autre\""))
        .await
        .unwrap();
    assert_eq!(json_tag(&different), tag);
    db.cleanup().await;
}

#[tokio::test]
async fn refresh_de_la_meme_version_invalide_les_anciens_etags() {
    let db = db_or_skip!();
    publish(db.storage.pool()).await;
    let document_before = document(
        db.storage.pool(),
        VERSION,
        "fr_FR",
        "item.json",
        &HeaderMap::new(),
    )
    .await
    .unwrap();
    let document_tag = json_tag(&document_before);
    let manifest_before = manifest(db.storage.pool(), &HeaderMap::new())
        .await
        .unwrap();
    let manifest_tag = json_tag(&manifest_before);
    sqlx::query("UPDATE static_data_releases SET completed_at=to_timestamp(3000),bundle=$1 WHERE version=$2")
        .bind(bundle("Bottes corrigées")).bind(VERSION).execute(db.storage.pool()).await.unwrap();
    sqlx::query("UPDATE static_data_manifest SET checked_at=to_timestamp(3000) WHERE id=1")
        .execute(db.storage.pool())
        .await
        .unwrap();
    let document_after = document(
        db.storage.pool(),
        VERSION,
        "fr_FR",
        "item.json",
        &conditional(&document_tag),
    )
    .await
    .unwrap();
    assert_ne!(json_tag(&document_after), document_tag);
    assert_eq!(json_body(document_after).await, item("Bottes corrigées"));
    let manifest_after = manifest(db.storage.pool(), &conditional(&manifest_tag))
        .await
        .unwrap();
    assert_ne!(json_tag(&manifest_after), manifest_tag);
    db.cleanup().await;
}

#[tokio::test]
async fn parametres_invalides_sont_rejetes_avant_de_chercher_un_document() {
    let db = db_or_skip!();
    for (version, locale, resource) in [
        ("16.19", "fr_FR", "item.json"),
        ("../16.19.1", "fr_FR", "item.json"),
        (VERSION, "fr", "item.json"),
        (VERSION, "de_DE", "item.json"),
        (VERSION, "fr_FR", ""),
        (VERSION, "fr_FR", "../item.json"),
        (VERSION, "fr_FR", "/item.json"),
        (VERSION, "fr_FR", "item.json?test=1"),
        (VERSION, "fr_FR", "https://example.com/item.json"),
    ] {
        assert_eq!(
            document(
                db.storage.pool(),
                version,
                locale,
                resource,
                &HeaderMap::new()
            )
            .await
            .err(),
            Some(ApiError::InvalidRequest)
        );
    }
    db.cleanup().await;
}
