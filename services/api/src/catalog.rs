//! Lecture cohérente et paginée du référentiel public, avec revalidation HTTP.
use crate::{error::ApiError, server::AppState};
use axum::{
    body::Body,
    extract::{
        rejection::{PathRejection, QueryRejection},
        Path, Query, State,
    },
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use olc_collector::catalog::{CatalogManifest, CatalogRecord};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, QueryBuilder, Row};

fn standard() -> String {
    "standard".into()
}
fn default_limit() -> u32 {
    50
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CatalogQuery {
    pub namespace: String,
    pub search: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub purchasable: Option<bool>,
    pub category: Option<String>,
    pub map: Option<String>,
    pub stat: Option<String>,
    pub min_stat: Option<f64>,
    pub coverage: Option<String>,
    pub offset: u32,
    pub limit: u32,
}
impl Default for CatalogQuery {
    fn default() -> Self {
        Self {
            namespace: standard(),
            search: None,
            min_price: None,
            max_price: None,
            purchasable: None,
            category: None,
            map: None,
            stat: None,
            min_stat: None,
            coverage: None,
            offset: 0,
            limit: default_limit(),
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetailQuery {
    #[serde(default = "standard")]
    pub namespace: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffQuery {
    pub from: String,
    pub to: String,
    pub locale: String,
    pub kind: String,
    #[serde(default = "standard")]
    pub namespace: String,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}
#[derive(Debug, Serialize)]
pub struct CatalogPage {
    pub publication_id: String,
    pub version: String,
    pub locale: String,
    pub kind: String,
    pub namespace: String,
    pub total: i64,
    pub offset: u32,
    pub limit: u32,
    pub records: Vec<CatalogRecord>,
}
#[derive(Debug, Serialize)]
pub struct CatalogDetail {
    pub publication_id: String,
    pub version: String,
    pub record: CatalogRecord,
}
#[derive(Debug, Serialize)]
pub struct CatalogChange {
    pub id: String,
    pub change: &'static str,
    pub sections: Vec<&'static str>,
}
#[derive(Debug, Serialize)]
pub struct CatalogDiffPage {
    pub from: String,
    pub to: String,
    pub locale: String,
    pub kind: String,
    pub namespace: String,
    pub total: i64,
    pub offset: u32,
    pub limit: u32,
    pub changes: Vec<CatalogChange>,
}

type ManifestPath = Result<Path<String>, PathRejection>;
type ListPath = Result<Path<(String, String, String)>, PathRejection>;
type DetailPath = Result<Path<(String, String, String, String)>, PathRejection>;

pub async fn manifest(
    State(state): State<AppState>,
    path: ManifestPath,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Path(version) = path.map_err(|_| ApiError::InvalidRequest)?;
    validate_version(&version)?;
    let manifest: sqlx::types::Json<CatalogManifest> = sqlx::query_scalar(
        "SELECT p.manifest FROM game_catalog_current c JOIN game_catalog_publications p ON p.id=c.publication_id WHERE c.version=$1")
        .bind(version).fetch_optional(&state.pool).await?.ok_or(ApiError::NotFound)?;
    cached_response(&manifest.0, &headers)
}

pub async fn list(
    State(state): State<AppState>,
    path: ListPath,
    query: Result<Query<CatalogQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Path((version, locale, kind)) = path.map_err(|_| ApiError::InvalidRequest)?;
    let Query(query) = query.map_err(|_| ApiError::InvalidRequest)?;
    validate_version(&version)?;
    validate_dimensions(&locale, &kind, &query.namespace)?;
    query.validate()?;
    // Une requête assure la cohérence, sans transaction susceptible de survivre
    // à l'annulation HTTP. PostgreSQL pagine avant tout transfert des fiches.
    let mut page = QueryBuilder::new(
        "WITH head AS (SELECT publication_id FROM game_catalog_current WHERE version=",
    );
    page.push_bind(&version)
        .push("), filtered AS (SELECT id,data FROM game_catalog_entries WHERE ");
    list_filters(&mut page, &locale, &kind, &query);
    page.push(") SELECT head.publication_id,(SELECT count(*) FROM filtered) AS total,p.data FROM head LEFT JOIN LATERAL (SELECT id,data FROM filtered ORDER BY id COLLATE \"C\" OFFSET ")
        .push_bind(i64::from(query.offset)).push(" LIMIT ").push_bind(i64::from(query.limit))
        .push(") p ON true ORDER BY p.id COLLATE \"C\"");
    let rows = page.build().fetch_all(&state.pool).await?;
    let head = rows.first().ok_or(ApiError::NotFound)?;
    let publication_id = head.try_get("publication_id")?;
    let total = head.try_get("total")?;
    let mut records = Vec::with_capacity(rows.len());
    for row in &rows {
        if let Some(record) = row.try_get::<Option<sqlx::types::Json<CatalogRecord>>, _>("data")? {
            records.push(record.0);
        }
    }
    cached_response(
        &CatalogPage {
            publication_id,
            version,
            locale,
            kind,
            namespace: query.namespace,
            total,
            offset: query.offset,
            limit: query.limit,
            records,
        },
        &headers,
    )
}

pub async fn detail(
    State(state): State<AppState>,
    path: DetailPath,
    query: Result<Query<DetailQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Path((version, locale, kind, id)) = path.map_err(|_| ApiError::InvalidRequest)?;
    let Query(query) = query.map_err(|_| ApiError::InvalidRequest)?;
    validate_version(&version)?;
    validate_dimensions(&locale, &kind, &query.namespace)?;
    if !identifier(&id) {
        return Err(ApiError::InvalidRequest);
    }
    let row = sqlx::query("SELECT c.publication_id,e.data FROM game_catalog_current c JOIN game_catalog_entries e ON e.publication_id=c.publication_id WHERE c.version=$1 AND e.locale=$2 AND e.kind=$3 AND e.id=$4 AND e.namespace=$5")
        .bind(&version).bind(locale).bind(kind).bind(id).bind(query.namespace)
        .fetch_optional(&state.pool).await?.ok_or(ApiError::NotFound)?;
    let record = row
        .try_get::<sqlx::types::Json<CatalogRecord>, _>("data")?
        .0;
    cached_response(
        &CatalogDetail {
            publication_id: row.try_get("publication_id")?,
            version,
            record,
        },
        &headers,
    )
}

pub async fn diff(
    State(state): State<AppState>,
    query: Result<Query<DiffQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::InvalidRequest)?;
    validate_dimensions(&query.locale, &query.kind, &query.namespace)?;
    validate_pagination(query.offset, query.limit)?;
    if !publication_id(&query.from) || !publication_id(&query.to) {
        return Err(ApiError::InvalidRequest);
    }
    let mut page = diff_sql(&query);
    page.push(") SELECT heads.valid,(SELECT count(*) FROM changes) AS total,p.id,p.before,p.after FROM heads LEFT JOIN LATERAL (SELECT * FROM changes ORDER BY id COLLATE \"C\" OFFSET ")
        .push_bind(i64::from(query.offset)).push(" LIMIT ").push_bind(i64::from(query.limit))
        .push(") p ON true ORDER BY p.id COLLATE \"C\"");
    let rows = page.build().fetch_all(&state.pool).await?;
    let head = rows.first().ok_or(ApiError::Unavailable)?;
    if !head.try_get::<bool, _>("valid")? {
        return Err(ApiError::NotFound);
    }
    let total = head.try_get("total")?;
    let mut changes = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(id) = row.try_get::<Option<String>, _>("id")? else {
            continue;
        };
        let before: Option<Value> = row.try_get("before")?;
        let after: Option<Value> = row.try_get("after")?;
        let (change, sections) = match (&before, &after) {
            (None, Some(_)) => ("added", vec![]),
            (Some(_), None) => ("removed", vec![]),
            (Some(before), Some(after)) => ("modified", changed_sections(before, after)),
            _ => return Err(ApiError::Unavailable),
        };
        changes.push(CatalogChange {
            id,
            change,
            sections,
        });
    }
    cached_response(
        &CatalogDiffPage {
            from: query.from,
            to: query.to,
            locale: query.locale,
            kind: query.kind,
            namespace: query.namespace,
            total,
            offset: query.offset,
            limit: query.limit,
            changes,
        },
        &headers,
    )
}

fn diff_sql(query: &DiffQuery) -> QueryBuilder<'_, Postgres> {
    let mut sql = QueryBuilder::new(
        "WITH heads AS (SELECT EXISTS(SELECT 1 FROM game_catalog_publications WHERE id=",
    );
    sql.push_bind(&query.from).push(") AND EXISTS(SELECT 1 FROM game_catalog_publications WHERE id=").push_bind(&query.to)
        .push(") AS valid), changes AS (SELECT COALESCE(a.id,b.id) AS id,a.data AS before,b.data AS after");
    sql.push(" FROM (SELECT id,data FROM game_catalog_entries WHERE publication_id=")
        .push_bind(&query.from);
    for (field, value) in [
        ("locale", &query.locale),
        ("kind", &query.kind),
        ("namespace", &query.namespace),
    ] {
        sql.push(" AND ").push(field).push("=").push_bind(value);
    }
    sql.push(") a FULL OUTER JOIN (SELECT id,data FROM game_catalog_entries WHERE publication_id=")
        .push_bind(&query.to);
    for (field, value) in [
        ("locale", &query.locale),
        ("kind", &query.kind),
        ("namespace", &query.namespace),
    ] {
        sql.push(" AND ").push(field).push("=").push_bind(value);
    }
    sql.push(") b ON a.id=b.id WHERE a.data IS DISTINCT FROM b.data");
    sql
}

fn list_filters<'a>(
    sql: &mut QueryBuilder<'a, Postgres>,
    locale: &'a str,
    kind: &'a str,
    query: &'a CatalogQuery,
) {
    sql.push("publication_id=(SELECT publication_id FROM head) AND locale=")
        .push_bind(locale)
        .push(" AND kind=")
        .push_bind(kind)
        .push(" AND namespace=")
        .push_bind(&query.namespace);
    if let Some(search) = &query.search {
        // strpos cherche un texte littéral : %, _ et antislash ne sont pas des jokers.
        sql.push(" AND strpos(lower(name),lower(")
            .push_bind(search)
            .push("))>0");
    }
    for (operator, bound) in [(">=", query.min_price), ("<=", query.max_price)] {
        if let Some(bound) = bound {
            sql.push(" AND CASE WHEN data->'fields'->'price_total'->>'status' IN ('verified','derived') AND jsonb_typeof(data->'fields'->'price_total'->'value')='number' THEN (data->'fields'->'price_total'->>'value')::numeric END ")
                .push(operator).push_bind(bound);
        }
    }
    if let Some(purchasable) = query.purchasable {
        sql.push(" AND data->'fields'->'purchasable'->>'status' IN ('verified','derived') AND data->'fields'->'purchasable'->'value'=")
            .push_bind(json!(purchasable));
    }
    if let Some(category) = &query.category {
        sql.push(" AND ((data->'fields'->'categories'->>'status' IN ('verified','derived') AND jsonb_typeof(data->'fields'->'categories'->'value')='array' AND data->'fields'->'categories'->'value' ? ")
            .push_bind(category).push(") OR (data->'fields'->'tags'->>'status' IN ('verified','derived') AND jsonb_typeof(data->'fields'->'tags'->'value')='array' AND data->'fields'->'tags'->'value' ? ")
            .push_bind(category).push("))");
    }
    if let Some(map) = &query.map {
        sql.push(" AND data->'fields'->'maps'->>'status' IN ('verified','derived') AND data->'fields'->'maps'->'value' -> ")
            .push_bind(map).push(" = 'true'::jsonb");
    }
    if let Some(stat) = &query.stat {
        sql.push(" AND data->'stats' -> ")
            .push_bind(stat)
            .push(" ->>'status' IN ('verified','derived') AND jsonb_typeof(data->'stats' -> ")
            .push_bind(stat)
            .push(" ->'value')='number'");
        if let Some(bound) = query.min_stat {
            sql.push(" AND CASE WHEN jsonb_typeof(data->'stats' -> ")
                .push_bind(stat)
                .push(" ->'value')='number' THEN (data->'stats' -> ")
                .push_bind(stat)
                .push(" ->>'value')::numeric END >= ")
                .push_bind(bound);
        }
    }
    if let Some(coverage) = &query.coverage {
        sql.push(if coverage == "complete" {
            " AND "
        } else {
            " AND NOT "
        });
        sql.push("(COALESCE(data->'coverage'->'issues','[]'::jsonb)='[]'::jsonb AND COALESCE(data->'coverage'->'unmapped_fields','[]'::jsonb)='[]'::jsonb AND NOT jsonb_path_exists(data, '$.**.status ? (@ == \"missing\" || @ == \"unsupported\" || @ == \"conflict\")'))");
    }
}
use serde_json::json;

impl CatalogQuery {
    fn validate(&self) -> Result<(), ApiError> {
        validate_pagination(self.offset, self.limit)?;
        if self
            .min_price
            .into_iter()
            .chain(self.max_price)
            .any(|v| !v.is_finite() || v < 0.0)
            || self
                .min_price
                .zip(self.max_price)
                .is_some_and(|(a, b)| a > b)
            || self
                .min_stat
                .is_some_and(|v| !v.is_finite() || self.stat.is_none())
            || self
                .search
                .as_ref()
                .is_some_and(|v| v.trim().is_empty() || v.len() > 120)
            || [&self.category, &self.map, &self.stat]
                .into_iter()
                .flatten()
                .any(|v| !identifier(v))
            || self
                .coverage
                .as_deref()
                .is_some_and(|v| !["complete", "incomplete"].contains(&v))
        {
            return Err(ApiError::InvalidRequest);
        }
        Ok(())
    }
}
fn validate_pagination(offset: u32, limit: u32) -> Result<(), ApiError> {
    if offset > 1_000_000 || !(1..=200).contains(&limit) {
        Err(ApiError::InvalidRequest)
    } else {
        Ok(())
    }
}
fn validate_version(version: &str) -> Result<(), ApiError> {
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || p.len() > 4 || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        Err(ApiError::InvalidRequest)
    } else {
        Ok(())
    }
}
fn validate_dimensions(locale: &str, kind: &str, namespace: &str) -> Result<(), ApiError> {
    let valid_context = if namespace == "global" {
        locale == "und" && ["map", "queue", "mode", "game_type"].contains(&kind)
    } else {
        ["fr_FR", "en_US"].contains(&locale) && ["standard", "classic"].contains(&namespace)
    };
    if !valid_context
        || ![
            "item",
            "champion",
            "ability",
            "rune",
            "rune_shard",
            "summoner_spell",
            "augment",
            "map",
            "queue",
            "mode",
            "game_type",
            "profile_icon",
        ]
        .contains(&kind)
    {
        Err(ApiError::InvalidRequest)
    } else {
        Ok(())
    }
}
fn identifier(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}
fn publication_id(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn changed_sections(before: &Value, after: &Value) -> Vec<&'static str> {
    let mut sections = Vec::new();
    for key in ["fields", "stats", "effects"] {
        if without_sources(&before[key]) != without_sources(&after[key]) {
            sections.push(key);
        }
    }
    if ["name", "description", "icon"]
        .into_iter()
        .any(|key| before[key] != after[key])
    {
        sections.push("text");
    }
    if sources(before) != sources(after) {
        sections.push("source");
    }
    if before["coverage"] != after["coverage"] {
        sections.push("coverage");
    }
    sections
}
fn is_catalog_value(value: &Value) -> bool {
    value.get("value").is_some()
        && value.get("status").is_some()
        && value.get("sources").is_some_and(Value::is_array)
}
fn conflict_candidates(value: &Value) -> Option<&Vec<Value>> {
    (value.get("status")?.as_str()? == "conflict")
        .then(|| value.get("value")?.get("candidates")?.as_array())
        .flatten()
        .filter(|values| values.iter().all(is_catalog_value))
}
fn without_sources(value: &Value) -> Value {
    match value {
        // Le JSON d'une valeur/calculation peut lui-même contenir une clé
        // `sources` : elle appartient à la valeur, pas à notre provenance.
        Value::Object(map) if is_catalog_value(value) => {
            let mut clean: serde_json::Map<String, Value> = map
                .iter()
                .filter(|(key, _)| key.as_str() != "sources")
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            if let Some(candidates) = conflict_candidates(value) {
                if let Some(Value::Object(inner)) = clean.get_mut("value") {
                    inner.insert(
                        "candidates".into(),
                        Value::Array(candidates.iter().map(without_sources).collect()),
                    );
                }
            }
            Value::Object(clean)
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), without_sources(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(without_sources).collect()),
        value => value.clone(),
    }
}
fn sources(value: &Value) -> Value {
    if is_catalog_value(value) {
        let mut result = serde_json::Map::new();
        if let Some(references) = value["sources"]
            .as_array()
            .filter(|values| !values.is_empty())
        {
            result.insert("sources".into(), Value::Array(references.clone()));
        }
        if let Some(candidates) = conflict_candidates(value) {
            let nested: serde_json::Map<String, Value> = candidates
                .iter()
                .enumerate()
                .filter_map(|(index, candidate)| {
                    let refs = sources(candidate);
                    (!refs.is_null()).then(|| (index.to_string(), refs))
                })
                .collect();
            if !nested.is_empty() {
                result.insert("candidates".into(), Value::Object(nested));
            }
        }
        return if result.is_empty() {
            Value::Null
        } else {
            Value::Object(result)
        };
    }
    let extracted: serde_json::Map<String, Value> = match value {
        Value::Object(map) => map
            .iter()
            .filter_map(|(key, value)| {
                let extracted = sources(value);
                (!extracted.is_null()).then(|| (key.clone(), extracted))
            })
            .collect(),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .filter_map(|(index, value)| {
                let extracted = sources(value);
                (!extracted.is_null()).then(|| (index.to_string(), extracted))
            })
            .collect(),
        _ => return Value::Null,
    };
    if extracted.is_empty() {
        Value::Null
    } else {
        Value::Object(extracted)
    }
}
fn cached_response(value: &impl Serialize, headers: &HeaderMap) -> Result<Response, ApiError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ApiError::Unavailable)?;
    let tag = format!("\"{:x}\"", Sha256::digest(&bytes));
    let matched = headers
        .get_all("if-none-match")
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(','))
        .any(|candidate| {
            let c = candidate.trim();
            c == "*" || c.strip_prefix("W/").unwrap_or(c) == tag
        });
    let mut response = if matched {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        Response::new(Body::from(bytes))
    };
    response.headers_mut().insert(
        "etag",
        HeaderValue::from_str(&tag).map_err(|_| ApiError::Unavailable)?,
    );
    // La même version peut recevoir une correction : ne jamais la marquer immutable.
    response.headers_mut().insert(
        "cache-control",
        HeaderValue::from_static("public, max-age=60, s-maxage=300, stale-while-revalidate=60"),
    );
    response
        .headers_mut()
        .insert("content-type", HeaderValue::from_static("application/json"));
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_augments_sont_une_famille_standard_localisee_jamais_globale() {
        // #118 : catalogue statique des augments Arena et Mayhem, sans taux ni tier.
        for locale in ["fr_FR", "en_US"] {
            assert!(validate_dimensions(locale, "augment", "standard").is_ok());
        }
        assert!(validate_dimensions("und", "augment", "global").is_err());
        assert!(validate_dimensions("fr_FR", "augment", "global").is_err());
        assert!(validate_dimensions("fr_FR", "augment_tier", "standard").is_err());
    }

    #[test]
    fn une_nouvelle_valeur_sans_provenance_ne_signale_pas_un_changement_de_source() {
        let before = json!({"fields":{},"stats":{},"effects":[],"coverage":{}});
        let after = json!({"fields":{"price_total":{"value":10,"status":"verified","sources":[]}},"stats":{},"effects":[],"coverage":{}});
        assert_eq!(changed_sections(&before, &after), vec!["fields"]);
    }

    #[test]
    fn une_cle_sources_dans_une_valeur_brute_reste_une_modification_de_valeur() {
        let before = json!({"fields":{"raw":{"value":{"sources":["a"]},"status":"unsupported","sources":[]}},"stats":{},"effects":[],"coverage":{}});
        let mut after = before.clone();
        after["fields"]["raw"]["value"]["sources"] = json!(["b"]);
        assert_eq!(changed_sections(&before, &after), vec!["fields"]);
    }

    #[test]
    fn provenance_seule_des_candidats_conflictuels_reste_un_changement_de_source() {
        let before = json!({"fields":{"price_total":{"status":"conflict","value":{"candidates":[{"value":10,"unit":"gold","status":"verified","sources":[{"source_id":"a","pointer":"/gold"}]}]},"sources":[{"source_id":"a","pointer":"/gold"}]}},"stats":{},"effects":[],"coverage":{}});
        let mut after = before.clone();
        after["fields"]["price_total"]["value"]["candidates"][0]["sources"][0]["source_id"] =
            json!("b");
        after["fields"]["price_total"]["sources"][0]["source_id"] = json!("b");
        assert_eq!(changed_sections(&before, &after), vec!["source"]);
        // Même si les références externes sont inchangées, la provenance interne compte.
        after["fields"]["price_total"]["sources"] =
            before["fields"]["price_total"]["sources"].clone();
        assert_eq!(changed_sections(&before, &after), vec!["source"]);
        after["fields"]["price_total"]["value"]["candidates"][0]["value"] = json!(20);
        assert_eq!(changed_sections(&before, &after), vec!["fields", "source"]);
    }

    #[test]
    fn changement_de_provenance_seul_ne_devient_pas_un_changement_de_valeur() {
        let before = json!({"fields":{"price_total":{"value":10,"status":"verified","sources":[{"source_id":"a","pointer":"/gold"}]}},"stats":{},"effects":[],"coverage":{}});
        let mut after = before.clone();
        after["fields"]["price_total"]["sources"][0]["source_id"] = json!("b");
        assert_eq!(changed_sections(&before, &after), vec!["source"]);
    }
}
