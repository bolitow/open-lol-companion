//! Contrats HTTP et filtres du référentiel, sur PostgreSQL jetable.
mod common;

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    response::Response,
    Router,
};
use common::TestDb;
use olc_api::{
    auth::Auth,
    server::{router, AppState},
};
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceExt;

const VERSION: &str = "16.19.1";
const FIRST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SECOND: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn app(pool: PgPool) -> Router {
    router(AppState::new(
        pool,
        Auth::new(b"synthetic-catalog-http-signing-material", "test", "api").unwrap(),
    ))
}
async fn get(app: &Router, path: &str) -> Response {
    app.clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}
async fn body(response: Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}
fn val(value: Value, status: &str) -> Value {
    json!({"value":value,"unit":null,"status":status,"sources":[]})
}
fn item(id: &str, price: Value, purchasable: Value, status: &str) -> Value {
    json!({"kind":"item","id":id,"namespace":"standard","locale":"fr_FR","name":format!("Objet {id}"),"description":null,"icon":null,
        "fields":{"price_total":val(price,status),"purchasable":val(purchasable,status),"categories":val(json!(["AbilityHaste"]),"verified"),"maps":val(json!({"11":true}),"verified")},
        "stats":{"ability_haste":val(json!(15),status)},"effects":[],
        "coverage":{"source_fields":5,"normalized_fields":5,"unmapped_fields":[],"issues":if status=="verified" {vec![]} else {vec!["incomplete"]}}})
}
async fn publish(pool: &PgPool, id: &str, records: &[Value]) {
    let manifest = json!({"publication_id":id,"version":VERSION,"normalizer_version":1,"published_at":"2026-10-01T00:00:00Z","degraded":false,"warnings":[],"sources":[],"coverage":{"records":records.len(),"source_fields":0,"normalized_fields":0,"unmapped_fields":0,"records_with_issues":0,"by_kind":{"item":records.len()}}});
    sqlx::query("INSERT INTO game_catalog_publications (id,version,normalizer_version,published_at,manifest) VALUES ($1,$2,1,now(),$3)")
        .bind(id).bind(VERSION).bind(manifest).execute(pool).await.unwrap();
    for record in records {
        sqlx::query("INSERT INTO game_catalog_entries (publication_id,kind,id,namespace,locale,name,data) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(id).bind(record["kind"].as_str().unwrap()).bind(record["id"].as_str().unwrap())
            .bind(record["namespace"].as_str().unwrap()).bind(record["locale"].as_str().unwrap())
            .bind(record["name"].as_str().unwrap()).bind(record).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO game_catalog_current (version,publication_id) VALUES ($1,$2) ON CONFLICT (version) DO UPDATE SET publication_id=EXCLUDED.publication_id")
        .bind(VERSION).bind(id).execute(pool).await.unwrap();
}

#[tokio::test]
async fn requetes_invalides_rejetees_en_json_avant_sql() {
    let app = app(PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap());
    for path in [
        "/v1/catalog/invalid/manifest",
        "/v1/catalog/16.19.1/de_DE/item",
        "/v1/catalog/16.19.1/fr_FR/player",
        "/v1/catalog/16.19.1/und/item?namespace=global",
        "/v1/catalog/16.19.1/und/queue",
        "/v1/catalog/16.19.1/fr_FR/queue?namespace=global",
        "/v1/catalog/16.19.1/fr_FR/item?limit=0",
        "/v1/catalog/16.19.1/fr_FR/item?limit=201",
        "/v1/catalog/16.19.1/fr_FR/item?offset=1000001",
        "/v1/catalog/16.19.1/fr_FR/item?min_price=-1",
        "/v1/catalog/16.19.1/fr_FR/item?min_price=20&max_price=10",
        "/v1/catalog/16.19.1/fr_FR/item?min_price=NaN",
        "/v1/catalog/16.19.1/fr_FR/item?purchasable=no",
        "/v1/catalog/16.19.1/fr_FR/item?min_stat=10",
        "/v1/catalog/16.19.1/fr_FR/item?coverage=unknown",
        "/v1/catalog/16.19.1/fr_FR/item?typo=yes",
        "/v1/catalog/16.19.1/fr_FR/item/1001?namespace=bad",
        "/v1/catalog-diff?from=latest&to=latest&locale=fr_FR&kind=item",
    ] {
        let response = get(&app, path).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(body(response).await["error"]["code"], "invalid_request");
    }
}

#[tokio::test]
async fn filtres_excluent_inconnus_uniquement_si_actifs_et_pagination_en_sql() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let pool = db.storage.pool();
    publish(
        pool,
        FIRST,
        &[
            item("1001", json!(0), json!(false), "verified"),
            item("1002", json!(3000), json!(true), "verified"),
            item("1003", Value::Null, Value::Null, "missing"),
            item("1004", json!(3000), json!(true), "conflict"),
        ],
    )
    .await;
    let app = app(pool.clone());
    for (query, total, first) in [
        ("", 4, "1001"),
        ("?min_price=0", 2, "1001"),
        ("?max_price=0", 1, "1001"),
        ("?purchasable=false", 1, "1001"),
        ("?purchasable=true", 1, "1002"),
        ("?stat=ability_haste&min_stat=15", 2, "1001"),
        ("?stat=ability_haste&min_stat=16", 0, ""),
        ("?category=AbilityHaste&map=11", 4, "1001"),
        ("?coverage=incomplete", 2, "1003"),
        ("?coverage=complete", 2, "1001"),
        ("?search=1002", 1, "1002"),
        ("?offset=1&limit=1", 4, "1002"),
    ] {
        let response = get(&app, &format!("/v1/catalog/{VERSION}/fr_FR/item{query}")).await;
        assert_eq!(response.status(), StatusCode::OK, "{query}");
        let page = body(response).await;
        assert_eq!(page["publication_id"], FIRST);
        assert_eq!(page["total"], total, "{query}");
        if total > 0 {
            assert_eq!(page["records"][0]["id"], first, "{query}");
        }
        if query.contains("limit=1") {
            assert_eq!(page["records"].as_array().unwrap().len(), 1);
        }
    }
    let empty = body(get(&app, &format!("/v1/catalog/{VERSION}/fr_FR/item?offset=20")).await).await;
    assert_eq!(empty["total"], 4);
    assert_eq!(empty["records"], json!([]));
    db.cleanup().await;
}

#[tokio::test]
async fn revision_modifie_etag_et_diff_distingue_source_texte_et_valeur() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let pool = db.storage.pool();
    let app = app(pool.clone());
    assert_eq!(
        get(&app, &format!("/v1/catalog/{VERSION}/manifest"))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let original = item("1001", json!(1000), json!(true), "verified");
    publish(
        pool,
        FIRST,
        &[
            original.clone(),
            item("1002", json!(1), json!(true), "verified"),
        ],
    )
    .await;
    for path in [
        format!("/v1/catalog/{VERSION}/manifest"),
        format!("/v1/catalog/{VERSION}/fr_FR/item"),
        format!("/v1/catalog/{VERSION}/fr_FR/item/1001"),
    ] {
        let response = get(&app, &path).await;
        assert_eq!(response.status(), StatusCode::OK);
        let tag = response.headers()["etag"].to_str().unwrap();
        assert!(!response.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("immutable"));
        let cached = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&path)
                    .header("if-none-match", format!("\"another\", W/{tag}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cached.status(), StatusCode::NOT_MODIFIED);
        assert!(to_bytes(cached.into_body(), 1024).await.unwrap().is_empty());
    }
    assert_eq!(
        get(&app, &format!("/v1/catalog/{VERSION}/fr_FR/item/404"))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let before = get(&app, &format!("/v1/catalog/{VERSION}/fr_FR/item/1001")).await;
    let tag = before.headers()["etag"].clone();
    let mut changed = original;
    changed["fields"]["price_total"]["value"] = json!(1100);
    changed["name"] = json!("Objet corrigé");
    changed["fields"]["price_total"]["sources"] = json!([{"source_id":"new","pointer":"/price"}]);
    publish(
        pool,
        SECOND,
        &[changed, item("1003", json!(2), json!(true), "verified")],
    )
    .await;
    let after = get(&app, &format!("/v1/catalog/{VERSION}/fr_FR/item/1001")).await;
    assert_ne!(after.headers()["etag"], tag);
    let diff = body(
        get(
            &app,
            &format!(
                "/v1/catalog-diff?from={FIRST}&to={SECOND}&locale=fr_FR&kind=item&offset=0&limit=1"
            ),
        )
        .await,
    )
    .await;
    assert_eq!(diff["total"], 3);
    assert_eq!(diff["changes"].as_array().unwrap().len(), 1);
    assert_eq!(diff["changes"][0]["id"], "1001");
    assert_eq!(diff["changes"][0]["change"], "modified");
    let sections = diff["changes"][0]["sections"].as_array().unwrap();
    for section in ["fields", "text", "source"] {
        assert!(sections.contains(&json!(section)), "{diff}");
    }
    let rest = body(
        get(
            &app,
            &format!("/v1/catalog-diff?from={FIRST}&to={SECOND}&locale=fr_FR&kind=item&offset=1"),
        )
        .await,
    )
    .await;
    assert_eq!(rest["changes"][0]["change"], "removed");
    assert_eq!(rest["changes"][1]["change"], "added");
    db.cleanup().await;
}

#[tokio::test]
async fn lecture_reste_coherente_pendant_les_publications_concurrentes() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let pool = db.storage.pool();
    publish(
        pool,
        FIRST,
        &[item("1001", json!(1), json!(true), "verified")],
    )
    .await;
    publish(
        pool,
        SECOND,
        &[
            item("2001", json!(1), json!(true), "verified"),
            item("2002", json!(2), json!(true), "verified"),
        ],
    )
    .await;
    let app = app(pool.clone());
    let writer_pool = pool.clone();
    let writer = tokio::spawn(async move {
        for iteration in 0..80 {
            sqlx::query("UPDATE game_catalog_current SET publication_id=$1 WHERE version=$2")
                .bind(if iteration % 2 == 0 { FIRST } else { SECOND })
                .bind(VERSION)
                .execute(&writer_pool)
                .await
                .unwrap();
            tokio::task::yield_now().await;
        }
    });
    for _ in 0..40 {
        let response = get(&app, &format!("/v1/catalog/{VERSION}/fr_FR/item")).await;
        assert_eq!(response.status(), StatusCode::OK);
        let page = body(response).await;
        let (total, first) = if page["publication_id"] == FIRST {
            (1, "1001")
        } else {
            (2, "2001")
        };
        assert_eq!(page["total"], total);
        assert_eq!(page["records"].as_array().unwrap().len(), total);
        assert_eq!(page["records"][0]["id"], first);
    }
    writer.await.unwrap();
    db.cleanup().await;
}

#[tokio::test]
async fn langues_contextes_et_filtres_ne_coercent_pas_les_valeurs() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let pool = db.storage.pool();
    let mut english = item("1001", json!(10), json!(true), "verified");
    english["locale"] = json!("en_US");
    english["name"] = json!("English item");
    let mut classic = item("1001", json!(20), json!(true), "verified");
    classic["namespace"] = json!("classic");
    classic["name"] = json!("Objet Classic");
    let mut non_numeric = item("1002", json!("3000"), json!("true"), "verified");
    non_numeric["stats"]["ability_haste"]["value"] = json!([15]);
    publish(
        pool,
        FIRST,
        &[
            item("1001", json!(0), json!(false), "derived"),
            non_numeric,
            english,
            classic,
            item("1003", json!(10), json!(true), "unsupported"),
            item("1004", json!(10), json!(true), "descriptive"),
        ],
    )
    .await;
    let app = app(pool.clone());
    for query in ["?min_price=0", "?purchasable=false", "?stat=ability_haste"] {
        let page = body(get(&app, &format!("/v1/catalog/{VERSION}/fr_FR/item{query}")).await).await;
        assert_eq!(page["total"], 1, "{query}: {page}");
        assert_eq!(page["records"][0]["id"], "1001");
    }
    let english = body(get(&app, &format!("/v1/catalog/{VERSION}/en_US/item/1001")).await).await;
    assert_eq!(english["record"]["name"], "English item");
    let classic = body(
        get(
            &app,
            &format!("/v1/catalog/{VERSION}/fr_FR/item/1001?namespace=classic"),
        )
        .await,
    )
    .await;
    assert_eq!(classic["record"]["name"], "Objet Classic");
    let none = body(
        get(
            &app,
            &format!("/v1/catalog/{VERSION}/fr_FR/item?search=%25"),
        )
        .await,
    )
    .await;
    assert_eq!(none["total"], 0);
    let same = body(
        get(
            &app,
            &format!("/v1/catalog-diff?from={FIRST}&to={FIRST}&locale=fr_FR&kind=item"),
        )
        .await,
    )
    .await;
    assert_eq!(same["total"], 0);
    assert_eq!(same["changes"], json!([]));
    assert_eq!(
        get(
            &app,
            &format!("/v1/catalog-diff?from={FIRST}&to={SECOND}&locale=fr_FR&kind=item")
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    db.cleanup().await;
}

#[tokio::test]
async fn catalogues_globaux_sont_accessibles_sans_inventer_de_traduction() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let pool = db.storage.pool();
    let mut queue = item("420", json!(null), json!(null), "missing");
    queue["kind"] = json!("queue");
    queue["namespace"] = json!("global");
    queue["locale"] = json!("und");
    queue["name"] = json!("Ranked Solo games");
    publish(pool, FIRST, &[queue]).await;
    let app = app(pool.clone());
    let response = get(
        &app,
        &format!("/v1/catalog/{VERSION}/und/queue?namespace=global"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page = body(response).await;
    assert_eq!(page["total"], 1);
    assert_eq!(page["locale"], "und");
    assert_eq!(page["records"][0]["name"], "Ranked Solo games");
    let detail = get(
        &app,
        &format!("/v1/catalog/{VERSION}/und/queue/420?namespace=global"),
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);
    let diff = get(
        &app,
        &format!("/v1/catalog-diff?from={FIRST}&to={FIRST}&locale=und&kind=queue&namespace=global"),
    )
    .await;
    assert_eq!(diff.status(), StatusCode::OK);
    db.cleanup().await;
}
